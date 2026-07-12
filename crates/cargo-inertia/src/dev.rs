//! Cross-platform Vite and Cargo development supervision.

pub mod args;
mod plan;
mod readiness;
mod supervisor;

use std::{path::Path, time::Duration};

use crate::{error::CliError, package_manager::detect};
pub use plan::{DevPlan, build_dev_plan};

/// Starts the frontend first, then Cargo once Vite is ready.
pub fn run_args(args: args::DevArgs) -> Result<(), CliError> {
    if !args.frontend.join("package.json").is_file() {
        return Err(CliError::Message(format!(
            "{} does not contain package.json",
            args.frontend.display()
        )));
    }
    let root = std::env::current_dir()?;
    let manager = detect(args.package_manager, &args.frontend, &root, None)
        .map_err(|error| CliError::Message(error.to_string()))?;
    which::which(manager.executable()).map_err(|error| CliError::Message(error.to_string()))?;
    let mut plan = build_dev_plan(
        args.frontend,
        manager,
        &args.host,
        args.port,
        args.cargo_args,
    )?;
    plan.readiness_timeout = Duration::from_secs(args.readiness_timeout);
    supervisor::run(plan, !args.no_readiness_wait)
}

/// Compatibility wrapper for callers that only configured a port.
pub fn run(frontend: &Path, port: u16) -> Result<(), String> {
    run_args(args::DevArgs {
        frontend: frontend.to_owned(),
        package_manager: Default::default(),
        host: "127.0.0.1".into(),
        port,
        readiness_timeout: 10,
        no_readiness_wait: false,
        cargo_args: vec![],
    })
    .map_err(|error| error.to_string())
}
