//! Fully resolved initialization settings.

use std::path::PathBuf;

use crate::{framework::Framework, package_manager::PackageManager, ssr::SsrOptions};

/// Values resolved before templates are rendered.
#[derive(Clone, Debug)]
pub struct InitOptions {
    /// Application root.
    pub root: PathBuf,
    /// Destination frontend directory.
    pub frontend_dir: PathBuf,
    /// Framework to scaffold.
    pub framework: Framework,
    /// Package manager for install and script guidance.
    pub package_manager: PackageManager,
    /// Whether dependencies should be installed.
    pub install: bool,
    /// SSR configuration.
    pub ssr: SsrOptions,
}
