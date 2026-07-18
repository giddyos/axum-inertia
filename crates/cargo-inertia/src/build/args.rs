//! Arguments for `cargo inertia build`.

use std::ffi::OsString;

use clap::Args;

/// Builds the configured frontend before invoking Cargo.
#[derive(Args, Clone, Debug, Default)]
pub struct BuildArgs {
    /// Build optimized Rust artifacts.
    #[arg(long)]
    pub release: bool,
    /// Rust package containing the configured binary.
    #[arg(short = 'p', long)]
    pub package: Option<String>,
    /// Rust compilation target triple.
    #[arg(long)]
    pub target: Option<String>,
    /// Additional arguments forwarded verbatim to `cargo build`.
    #[arg(last = true, allow_hyphen_values = true)]
    pub cargo_args: Vec<OsString>,
}
