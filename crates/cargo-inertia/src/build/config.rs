//! Typed `inertia.toml` production-build configuration.

use std::{
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
};

use serde::Deserialize;

use crate::error::CliError;

/// Fully resolved build configuration.
#[derive(Clone, Debug)]
pub struct BuildConfiguration {
    /// Located Cargo workspace root.
    pub root: PathBuf,
    /// Path to the source configuration.
    pub path: PathBuf,
    /// Frontend working directory.
    pub frontend_root: PathBuf,
    /// Exact configured frontend command.
    pub build_command: Vec<OsString>,
    /// Frontend build output directory.
    pub build_dir: PathBuf,
    /// Vite client manifest.
    pub manifest: PathBuf,
    /// Vite entry key required in the manifest.
    pub entry: String,
    /// Public URL prefix used by the embedded frontend.
    pub public_path: String,
    /// Configured Rust package.
    pub package: String,
    /// Configured Rust binary.
    pub binary: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceConfiguration {
    frontend: FrontendConfiguration,
    rust: RustConfiguration,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct FrontendConfiguration {
    root: PathBuf,
    build_command: Vec<String>,
    build_dir: PathBuf,
    manifest: PathBuf,
    entry: String,
    public_path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RustConfiguration {
    package: String,
    binary: String,
}

impl BuildConfiguration {
    /// Reads and validates `<workspace>/inertia.toml`.
    pub fn load(root: &Path) -> Result<Self, CliError> {
        let path = root.join("inertia.toml");
        let source =
            fs::read_to_string(&path).map_err(|error| CliError::InvalidBuildConfiguration {
                path: path.clone(),
                message: if error.kind() == std::io::ErrorKind::NotFound {
                    "file not found; create inertia.toml with [frontend] and [rust] sections"
                        .to_owned()
                } else {
                    error.to_string()
                },
            })?;
        let source: SourceConfiguration =
            toml::from_str(&source).map_err(|error| CliError::InvalidBuildConfiguration {
                path: path.clone(),
                message: error.to_string(),
            })?;
        source.resolve(root, path)
    }
}

impl SourceConfiguration {
    fn resolve(self, root: &Path, path: PathBuf) -> Result<BuildConfiguration, CliError> {
        validate_relative_path(&path, "frontend.root", &self.frontend.root)?;
        validate_relative_path(&path, "frontend.build-dir", &self.frontend.build_dir)?;
        validate_relative_path(&path, "frontend.manifest", &self.frontend.manifest)?;
        if self.frontend.build_command.is_empty()
            || self
                .frontend
                .build_command
                .iter()
                .any(|part| part.is_empty())
        {
            return invalid(
                &path,
                "frontend.build-command must contain a program and no empty arguments",
            );
        }
        if self.frontend.entry.trim().is_empty() {
            return invalid(&path, "frontend.entry must not be empty");
        }
        if !self.frontend.public_path.starts_with('/')
            || self.frontend.public_path.contains(['?', '#'])
        {
            return invalid(
                &path,
                "frontend.public-path must be an absolute URL path without a query or fragment",
            );
        }
        if self.rust.package.trim().is_empty() || self.rust.binary.trim().is_empty() {
            return invalid(&path, "rust.package and rust.binary must not be empty");
        }
        Ok(BuildConfiguration {
            root: root.to_owned(),
            path,
            frontend_root: root.join(self.frontend.root),
            build_command: self
                .frontend
                .build_command
                .into_iter()
                .map(OsString::from)
                .collect(),
            build_dir: root.join(self.frontend.build_dir),
            manifest: root.join(self.frontend.manifest),
            entry: self.frontend.entry,
            public_path: self.frontend.public_path,
            package: self.rust.package,
            binary: self.rust.binary,
        })
    }
}

fn validate_relative_path(config_path: &Path, field: &str, value: &Path) -> Result<(), CliError> {
    if value.as_os_str().is_empty()
        || value.is_absolute()
        || value
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return invalid(
            config_path,
            format!("{field} must be a non-empty path beneath the workspace root"),
        );
    }
    Ok(())
}

fn invalid<T>(path: &Path, message: impl Into<String>) -> Result<T, CliError> {
    Err(CliError::InvalidBuildConfiguration {
        path: path.to_owned(),
        message: message.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_paths_that_escape_the_workspace() {
        let source = SourceConfiguration {
            frontend: FrontendConfiguration {
                root: PathBuf::from("../frontend"),
                build_command: vec!["pnpm".to_owned(), "build".to_owned()],
                build_dir: PathBuf::from("frontend/dist"),
                manifest: PathBuf::from("frontend/dist/.vite/manifest.json"),
                entry: "src/main.ts".to_owned(),
                public_path: "/build".to_owned(),
            },
            rust: RustConfiguration {
                package: "server".to_owned(),
                binary: "server".to_owned(),
            },
        };
        assert!(matches!(
            source.resolve(
                Path::new("/workspace"),
                PathBuf::from("/workspace/inertia.toml")
            ),
            Err(CliError::InvalidBuildConfiguration { .. })
        ));
    }
}
