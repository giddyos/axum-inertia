//! Typed errors returned by CLI commands.

use std::{io, path::PathBuf};

use thiserror::Error;

/// Errors produced by `cargo inertia` commands.
#[derive(Debug, Error)]
pub enum CliError {
    /// A scaffold destination already exists.
    #[error("scaffold destination already exists: {0}")]
    FrontendExists(PathBuf),
    /// The user cancelled initialization before files were generated.
    #[error("initialization cancelled")]
    Cancelled,
    /// Required initialization input is missing or inconsistent.
    #[error("invalid initialization options: {0}")]
    InvalidOptions(String),
    /// A destination cannot be used for atomic scaffolding.
    #[error("invalid frontend destination: {0}")]
    InvalidDestination(PathBuf),
    /// A generated output path is unsafe.
    #[error("unsafe generated output path: {0}")]
    UnsafeOutputPath(PathBuf),
    /// An explicitly catalogued embedded template was not found.
    #[error("embedded template is missing: {0}")]
    MissingEmbeddedTemplate(&'static str),
    /// An embedded template cannot be decoded as UTF-8.
    #[error("embedded template is not UTF-8: {0}")]
    TemplateIsNotUtf8(&'static str),
    /// A source template is invalid.
    #[cfg(feature = "templates")]
    #[error("invalid template syntax")]
    TemplateSyntax(#[source] minijinja::Error),
    /// A source template could not be rendered.
    #[cfg(feature = "templates")]
    #[error("could not render template `{template}`")]
    Template {
        template: String,
        #[source]
        source: minijinja::Error,
    },
    /// The project build configuration is invalid.
    #[error("invalid build configuration at {path}: {message}")]
    InvalidBuildConfiguration { path: PathBuf, message: String },
    /// The configured frontend production command failed.
    #[error("frontend build command failed: {command}")]
    FrontendBuildFailed { command: String },
    /// The frontend build did not produce the configured Vite manifest.
    #[error("frontend build did not produce the configured Vite manifest: {0}")]
    MissingViteManifest(PathBuf),
    /// The configured Vite manifest is malformed or incomplete.
    #[error("invalid Vite manifest at {path}: {message}")]
    InvalidViteManifest { path: PathBuf, message: String },
    /// Cargo failed after the frontend build completed.
    #[error("Cargo build failed; the frontend build and manifest are available for inspection")]
    CargoBuildFailed,
    /// Cargo did not report the configured binary artifact.
    #[error("Cargo did not report executable `{binary}` for package `{package}`")]
    MissingExecutableArtifact { package: String, binary: String },
    /// Cargo reported more than one path for the configured binary.
    #[error(
        "Cargo reported multiple executable paths for `{binary}` in package `{package}`: {paths:?}"
    )]
    AmbiguousExecutableArtifact {
        package: String,
        binary: String,
        paths: Vec<PathBuf>,
    },
    /// The staged frontend could not be moved into place.
    #[error("could not commit generated frontend")]
    CommitScaffold {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: io::Error,
    },
    /// An I/O operation failed.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// Generated JSON is invalid.
    #[cfg(feature = "templates")]
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A legacy command implementation reported an error string.
    #[error("{0}")]
    Message(String),
}

impl From<String> for CliError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}
