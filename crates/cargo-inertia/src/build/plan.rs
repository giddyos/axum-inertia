//! Shell-free production build planning.

use std::{ffi::OsString, path::PathBuf};

use crate::{
    build::{args::BuildArgs, config::BuildConfiguration},
    package_manager::CommandSpec,
};

/// Ordered frontend and Cargo process specifications.
#[derive(Clone, Debug)]
pub struct BuildPlan {
    /// Exact configured frontend process.
    pub frontend: CommandSpec,
    /// Cargo build process with internal artifact reporting enabled.
    pub cargo: CommandSpec,
    /// Expected Cargo package identifier.
    pub package_id: String,
    /// Selected package name.
    pub package: String,
    /// Selected binary name.
    pub binary: String,
    /// Manifest verified between the two processes.
    pub manifest: PathBuf,
    /// Build directory summarized after successful frontend execution.
    pub build_dir: PathBuf,
    /// Configured Vite entry key.
    pub entry: String,
}

/// Creates a process plan after Cargo metadata has validated package selection.
pub fn build_plan(
    configuration: &BuildConfiguration,
    args: BuildArgs,
    package_id: String,
) -> BuildPlan {
    let package = args
        .package
        .unwrap_or_else(|| configuration.package.clone());
    let mut cargo_args = vec![OsString::from("build")];
    if args.release {
        cargo_args.push(OsString::from("--release"));
    }
    cargo_args.extend([
        OsString::from("--package"),
        OsString::from(&package),
        OsString::from("--bin"),
        OsString::from(&configuration.binary),
    ]);
    if let Some(target) = args.target {
        cargo_args.extend([OsString::from("--target"), OsString::from(target)]);
    }
    cargo_args.push(OsString::from("--message-format=json-render-diagnostics"));
    cargo_args.extend(args.cargo_args);

    BuildPlan {
        frontend: CommandSpec {
            program: configuration.build_command[0].clone(),
            args: configuration.build_command[1..].to_vec(),
            current_dir: configuration.frontend_root.clone(),
        },
        cargo: CommandSpec {
            program: OsString::from("cargo"),
            args: cargo_args,
            current_dir: configuration.root.clone(),
        },
        package_id,
        package,
        binary: configuration.binary.clone(),
        manifest: configuration.manifest.clone(),
        build_dir: configuration.build_dir.clone(),
        entry: configuration.entry.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_supported_options_and_every_forwarded_argument_in_order() {
        let configuration = BuildConfiguration {
            root: PathBuf::from("/workspace"),
            path: PathBuf::from("/workspace/inertia.toml"),
            frontend_root: PathBuf::from("/workspace/frontend"),
            build_command: ["pnpm", "run", "build"].map(OsString::from).to_vec(),
            build_dir: PathBuf::from("/workspace/frontend/dist"),
            manifest: PathBuf::from("/workspace/frontend/dist/.vite/manifest.json"),
            entry: "src/main.ts".to_owned(),
            public_path: "/build".to_owned(),
            package: "configured".to_owned(),
            binary: "server".to_owned(),
        };
        let plan = build_plan(
            &configuration,
            BuildArgs {
                release: true,
                package: Some("selected".to_owned()),
                target: Some("x86_64-unknown-linux-musl".to_owned()),
                cargo_args: ["--features", "ssr", "--no-default-features"]
                    .map(OsString::from)
                    .to_vec(),
            },
            "selected 1.0.0 (path+file:///workspace)".to_owned(),
        );
        assert_eq!(plan.frontend.program, "pnpm");
        assert_eq!(plan.frontend.args, ["run", "build"].map(OsString::from));
        assert_eq!(
            plan.cargo.args,
            [
                "build",
                "--release",
                "--package",
                "selected",
                "--bin",
                "server",
                "--target",
                "x86_64-unknown-linux-musl",
                "--message-format=json-render-diagnostics",
                "--features",
                "ssr",
                "--no-default-features",
            ]
            .map(OsString::from)
        );
    }
}
