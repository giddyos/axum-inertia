//! Fully resolved initialization settings.

use std::path::PathBuf;

use crate::{
    framework::Framework, package_manager::PackageManager, server_framework::ServerFramework,
    ssr::SsrOptions,
};

/// Values resolved before templates are rendered.
#[derive(Clone, Debug)]
pub struct InitOptions {
    /// Application root.
    pub root: PathBuf,
    /// Destination frontend directory.
    pub frontend_dir: PathBuf,
    /// Framework to scaffold.
    pub framework: Framework,
    /// Optional Rust framework for a complete generated project.
    pub server_framework: Option<ServerFramework>,
    /// Package manager for install and script guidance.
    pub package_manager: PackageManager,
    /// Whether dependencies should be installed.
    pub install: bool,
    /// SSR configuration.
    pub ssr: SsrOptions,
}
