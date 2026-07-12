//! Arguments for `cargo inertia check`.

use std::path::PathBuf;

use clap::Args;

use crate::{framework::Framework, package_manager::PackageManagerChoice};

/// Automatic or explicit frontend framework detection.
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum FrameworkChoice {
    /// Resolve from frontend dependencies and file extensions.
    #[default]
    Auto,
    /// React.
    React,
    /// Svelte.
    Svelte,
    /// Vue.
    Vue,
}

impl FrameworkChoice {
    /// Returns an explicit framework if supplied.
    pub const fn explicit(self) -> Option<Framework> {
        match self {
            Self::Auto => None,
            Self::React => Some(Framework::React),
            Self::Svelte => Some(Framework::Svelte),
            Self::Vue => Some(Framework::Vue),
        }
    }
}

/// Validates frontend and Rust Inertia declarations.
#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Application root.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
    /// Frontend directory.
    #[arg(long, default_value = "frontend")]
    pub frontend: PathBuf,
    /// Frontend framework.
    #[arg(long, default_value = "auto")]
    pub framework: FrameworkChoice,
    /// Package manager.
    #[arg(long, default_value = "auto")]
    pub package_manager: PackageManagerChoice,
    /// Vite entry path relative to the frontend directory.
    #[arg(long)]
    pub entry: Option<PathBuf>,
    /// Pages directory relative to the frontend directory.
    #[arg(long)]
    pub pages: Option<PathBuf>,
    /// Cargo package to inspect.
    #[arg(long, conflicts_with = "workspace")]
    pub package: Option<String>,
    /// Inspect workspace packages.
    #[arg(long)]
    pub workspace: bool,
    /// Require built frontend artifacts.
    #[arg(long)]
    pub built: bool,
    /// Require SSR artifacts.
    #[arg(long)]
    pub ssr: bool,
    /// SSR bundle path relative to frontend.
    #[arg(long)]
    pub ssr_bundle: Option<PathBuf>,
}
