//! Interactive answer types, populated by the optional interactive feature.

use crate::{framework::Framework, package_manager::PackageManager, ssr::SsrOptions};

/// Answers collected by interactive initialization.
#[derive(Clone, Debug)]
pub struct InitAnswers {
    /// Selected framework.
    pub framework: Framework,
    /// Selected package manager.
    pub package_manager: PackageManager,
    /// Selected SSR options.
    pub ssr: SsrOptions,
    /// Whether dependencies should be installed.
    pub install: bool,
}
