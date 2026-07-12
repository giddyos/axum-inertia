//! Arguments for `cargo inertia dev`.

use std::{ffi::OsString, path::PathBuf};

use clap::Args;

use crate::package_manager::PackageManagerChoice;

/// Runs Vite and the Rust application together.
#[derive(Args, Debug)]
pub struct DevArgs {
    /// Frontend directory.
    #[arg(long, default_value = "frontend")]
    pub frontend: PathBuf,
    /// Package manager to use or automatically detect.
    #[arg(long, default_value = "auto")]
    pub package_manager: PackageManagerChoice,
    /// Vite bind host.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Vite bind port.
    #[arg(long, default_value_t = 5173)]
    pub port: u16,
    /// Seconds to wait for Vite readiness.
    #[arg(long, default_value_t = 10)]
    pub readiness_timeout: u64,
    /// Do not wait for Vite readiness before starting Cargo.
    #[arg(long)]
    pub no_readiness_wait: bool,
    /// Arguments passed to `cargo run` after `--`.
    #[arg(last = true)]
    pub cargo_args: Vec<OsString>,
}
