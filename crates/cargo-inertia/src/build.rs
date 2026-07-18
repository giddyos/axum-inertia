//! Frontend-first, artifact-evidenced production builds.

pub mod args;
pub mod config;
pub mod plan;

use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use cargo_metadata::{Message, Metadata, MetadataCommand, Package, TargetKind};

use crate::{
    build::{config::BuildConfiguration, plan::BuildPlan},
    error::CliError,
    package_manager::CommandSpec,
};

/// Runs the configured frontend build followed by the selected Cargo binary build.
pub fn run_args(args: args::BuildArgs, output: &mut impl Write) -> Result<(), CliError> {
    let current_dir = std::env::current_dir()?;
    let metadata = locate_workspace(&current_dir)?;
    let configuration = BuildConfiguration::load(metadata.workspace_root.as_std_path())?;
    let package_name = args.package.as_deref().unwrap_or(&configuration.package);
    let package = select_package(&metadata, package_name, &configuration.binary)?;
    let plan = plan::build_plan(&configuration, args, package.id.to_string());
    let outcome = execute(&plan, &mut SystemRunner)?;

    if let Some((assets, bytes)) = outcome.assets {
        writeln!(
            output,
            "Embedded frontend: {assets} assets, {bytes} uncompressed bytes"
        )?;
    }
    writeln!(output, "Executable: {}", outcome.executable.display())?;
    Ok(())
}

fn locate_workspace(current_dir: &Path) -> Result<Metadata, CliError> {
    MetadataCommand::new()
        .current_dir(current_dir)
        .no_deps()
        .exec()
        .map_err(|error| {
            CliError::Message(format!(
                "could not locate the Cargo workspace from {}: {error}",
                current_dir.display()
            ))
        })
}

fn select_package<'a>(
    metadata: &'a Metadata,
    package_name: &str,
    binary: &str,
) -> Result<&'a Package, CliError> {
    let matches = metadata
        .packages
        .iter()
        .filter(|package| package.name == package_name)
        .collect::<Vec<_>>();
    let package = match matches.as_slice() {
        [package] => *package,
        [] => {
            return Err(CliError::Message(format!(
                "configured Rust package `{package_name}` is not a member of {}",
                metadata.workspace_root
            )));
        }
        _ => {
            return Err(CliError::Message(format!(
                "Rust package name `{package_name}` is ambiguous in {}",
                metadata.workspace_root
            )));
        }
    };
    if !package
        .targets
        .iter()
        .any(|target| target.name == binary && target.kind.contains(&TargetKind::Bin))
    {
        return Err(CliError::Message(format!(
            "package `{package_name}` has no binary target named `{binary}`"
        )));
    }
    Ok(package)
}

struct BuildOutcome {
    executable: PathBuf,
    assets: Option<(usize, u64)>,
}

trait Runner {
    fn frontend(&mut self, command: &CommandSpec) -> Result<bool, CliError>;
    fn cargo(&mut self, command: &CommandSpec) -> Result<CargoOutcome, CliError>;
}

struct CargoOutcome {
    success: bool,
    artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Artifact {
    package_id: String,
    target: String,
    executable: PathBuf,
}

fn execute(plan: &BuildPlan, runner: &mut impl Runner) -> Result<BuildOutcome, CliError> {
    if !runner.frontend(&plan.frontend)? {
        return Err(CliError::FrontendBuildFailed {
            command: display_command(&plan.frontend),
        });
    }
    verify_manifest(&plan.manifest, &plan.build_dir, &plan.entry)?;
    let assets = asset_summary(&plan.build_dir).ok();
    let cargo = runner.cargo(&plan.cargo)?;
    if !cargo.success {
        return Err(CliError::CargoBuildFailed);
    }
    let paths = cargo
        .artifacts
        .into_iter()
        .filter(|artifact| artifact.package_id == plan.package_id && artifact.target == plan.binary)
        .map(|artifact| artifact.executable)
        .collect::<BTreeSet<_>>();
    let executable = match paths.len() {
        0 => {
            return Err(CliError::MissingExecutableArtifact {
                package: plan.package.clone(),
                binary: plan.binary.clone(),
            });
        }
        1 => paths.into_iter().next().expect("one path"),
        _ => {
            return Err(CliError::AmbiguousExecutableArtifact {
                package: plan.package.clone(),
                binary: plan.binary.clone(),
                paths: paths.into_iter().collect(),
            });
        }
    };
    Ok(BuildOutcome { executable, assets })
}

struct SystemRunner;

impl Runner for SystemRunner {
    fn frontend(&mut self, command: &CommandSpec) -> Result<bool, CliError> {
        Ok(Command::new(&command.program)
            .args(&command.args)
            .current_dir(&command.current_dir)
            .status()
            .map_err(|error| {
                CliError::Message(format!(
                    "could not start frontend build `{}`: {error}",
                    display_command(command)
                ))
            })?
            .success())
    }

    fn cargo(&mut self, command: &CommandSpec) -> Result<CargoOutcome, CliError> {
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .current_dir(&command.current_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| CliError::Message(format!("could not start Cargo build: {error}")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CliError::Message("could not capture Cargo output".to_owned()))?;
        let artifacts = collect_artifacts(BufReader::new(stdout), &mut io::stderr().lock())?;
        let success = child.wait()?.success();
        Ok(CargoOutcome { success, artifacts })
    }
}

fn collect_artifacts(
    reader: impl io::BufRead,
    diagnostics: &mut impl Write,
) -> Result<Vec<Artifact>, CliError> {
    let mut artifacts = Vec::new();
    for message in Message::parse_stream(reader) {
        match message.map_err(CliError::Io)? {
            Message::CompilerArtifact(artifact) => {
                if let Some(executable) = artifact.executable {
                    artifacts.push(Artifact {
                        package_id: artifact.package_id.to_string(),
                        target: artifact.target.name,
                        executable: executable.into_std_path_buf(),
                    });
                }
            }
            Message::CompilerMessage(message) => {
                if let Some(rendered) = message.message.rendered {
                    write!(diagnostics, "{rendered}")?;
                }
            }
            Message::TextLine(line) => writeln!(diagnostics, "{line}")?,
            Message::BuildScriptExecuted(_) | Message::BuildFinished(_) => {}
            _ => {}
        }
    }
    Ok(artifacts)
}

fn verify_manifest(manifest: &Path, build_dir: &Path, entry: &str) -> Result<(), CliError> {
    let source = fs::read(manifest).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            CliError::MissingViteManifest(manifest.to_owned())
        } else {
            CliError::InvalidViteManifest {
                path: manifest.to_owned(),
                message: error.to_string(),
            }
        }
    })?;
    let value: serde_json::Value =
        serde_json::from_slice(&source).map_err(|error| CliError::InvalidViteManifest {
            path: manifest.to_owned(),
            message: error.to_string(),
        })?;
    let record = value
        .as_object()
        .and_then(|manifest| manifest.get(entry))
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| CliError::InvalidViteManifest {
            path: manifest.to_owned(),
            message: format!("missing entry `{entry}`"),
        })?;
    let file = record
        .get("file")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| CliError::InvalidViteManifest {
            path: manifest.to_owned(),
            message: format!("entry `{entry}` has no output file"),
        })?;
    let output = build_dir.join(file);
    if !output.is_file() {
        return Err(CliError::InvalidViteManifest {
            path: manifest.to_owned(),
            message: format!(
                "entry `{entry}` references missing output {}",
                output.display()
            ),
        });
    }
    Ok(())
}

fn asset_summary(root: &Path) -> io::Result<(usize, u64)> {
    fn visit(path: &Path, assets: &mut usize, bytes: &mut u64) -> io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                visit(&entry.path(), assets, bytes)?;
            } else if metadata.is_file() {
                *assets += 1;
                *bytes = bytes.checked_add(metadata.len()).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "asset byte count overflow")
                })?;
            }
        }
        Ok(())
    }

    let mut assets = 0;
    let mut bytes = 0;
    visit(root, &mut assets, &mut bytes)?;
    Ok((assets, bytes))
}

fn display_command(command: &CommandSpec) -> String {
    std::iter::once(command.program.as_os_str())
        .chain(command.args.iter().map(AsRef::<OsStr>::as_ref))
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    struct FakeRunner {
        calls: Vec<&'static str>,
        frontend_success: bool,
        cargo: Option<CargoOutcome>,
    }

    impl Runner for FakeRunner {
        fn frontend(&mut self, _command: &CommandSpec) -> Result<bool, CliError> {
            self.calls.push("frontend");
            Ok(self.frontend_success)
        }

        fn cargo(&mut self, _command: &CommandSpec) -> Result<CargoOutcome, CliError> {
            self.calls.push("cargo");
            Ok(self.cargo.take().unwrap())
        }
    }

    fn fixture() -> (tempfile::TempDir, BuildPlan) {
        let root = tempfile::tempdir().unwrap();
        let build_dir = root.path().join("frontend/dist");
        fs::create_dir_all(build_dir.join(".vite")).unwrap();
        fs::create_dir_all(build_dir.join("assets")).unwrap();
        fs::write(build_dir.join("assets/main.js"), "export default 1").unwrap();
        fs::write(
            build_dir.join(".vite/manifest.json"),
            r#"{"src/main.ts":{"file":"assets/main.js","isEntry":true}}"#,
        )
        .unwrap();
        (
            root,
            BuildPlan {
                frontend: CommandSpec {
                    program: OsString::from("pnpm"),
                    args: vec![OsString::from("build")],
                    current_dir: PathBuf::from("frontend"),
                },
                cargo: CommandSpec {
                    program: OsString::from("cargo"),
                    args: vec![OsString::from("build")],
                    current_dir: PathBuf::from("."),
                },
                package_id: "server 1.0.0 (path+file:///server)".to_owned(),
                package: "server".to_owned(),
                binary: "server".to_owned(),
                manifest: build_dir.join(".vite/manifest.json"),
                build_dir,
                entry: "src/main.ts".to_owned(),
            },
        )
    }

    #[test]
    fn executes_frontend_then_manifest_verification_then_cargo() {
        let (_root, plan) = fixture();
        let executable = plan.cargo.current_dir.join("target/release/server");
        let mut runner = FakeRunner {
            calls: Vec::new(),
            frontend_success: true,
            cargo: Some(CargoOutcome {
                success: true,
                artifacts: vec![Artifact {
                    package_id: plan.package_id.clone(),
                    target: plan.binary.clone(),
                    executable: executable.clone(),
                }],
            }),
        };
        let outcome = execute(&plan, &mut runner).unwrap();
        assert_eq!(runner.calls, ["frontend", "cargo"]);
        assert_eq!(outcome.executable, executable);
        let (assets, bytes) = outcome.assets.unwrap();
        assert_eq!(assets, 2);
        assert!(bytes > 0);
    }

    #[test]
    fn never_invokes_cargo_when_manifest_is_missing() {
        let (_root, plan) = fixture();
        fs::remove_file(&plan.manifest).unwrap();
        let mut runner = FakeRunner {
            calls: Vec::new(),
            frontend_success: true,
            cargo: Some(CargoOutcome {
                success: true,
                artifacts: Vec::new(),
            }),
        };
        assert!(matches!(
            execute(&plan, &mut runner),
            Err(CliError::MissingViteManifest(_))
        ));
        assert_eq!(runner.calls, ["frontend"]);
    }

    #[test]
    fn cargo_json_provides_the_real_executable_path() {
        let input = concat!(
            "{\"reason\":\"compiler-artifact\",\"package_id\":\"path+file:///server#0.1.0\",",
            "\"manifest_path\":\"/server/Cargo.toml\",\"target\":{\"kind\":[\"bin\"],",
            "\"crate_types\":[\"bin\"],\"name\":\"server\",\"src_path\":\"/server/src/main.rs\",",
            "\"edition\":\"2024\",\"doc\":true,\"doctest\":false,\"test\":true},",
            "\"profile\":{\"opt_level\":\"3\",\"debuginfo\":0,\"debug_assertions\":false,",
            "\"overflow_checks\":false,\"test\":false},\"features\":[],",
            "\"filenames\":[\"/custom/server\"],\"executable\":\"/custom/server\",",
            "\"fresh\":false}\n",
            "{\"reason\":\"build-finished\",\"success\":true}\n"
        );
        let mut diagnostics = Vec::new();
        let artifacts =
            collect_artifacts(io::Cursor::new(input.as_bytes()), &mut diagnostics).unwrap();
        assert_eq!(
            artifacts,
            [Artifact {
                package_id: "path+file:///server#0.1.0".to_owned(),
                target: "server".to_owned(),
                executable: PathBuf::from("/custom/server"),
            }]
        );
        assert!(diagnostics.is_empty());
    }
}
