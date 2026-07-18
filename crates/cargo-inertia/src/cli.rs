//! Command-line parsing and Cargo-subcommand normalization.

use clap::{Parser, Subcommand};

use crate::error::CliError;

/// Parses and dispatches the optional project-tooling commands.
#[derive(Parser)]
#[command(
    name = "cargo inertia",
    version,
    about = "Project tooling for Inertia Rust applications"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Creates a minimal Vite frontend.
    #[cfg(feature = "init")]
    Init(crate::init::args::InitArgs),
    /// Runs Vite and cargo run together.
    #[cfg(feature = "dev")]
    Dev(crate::dev::args::DevArgs),
    /// Builds the configured frontend before the Rust release artifact.
    #[cfg(feature = "build")]
    Build(crate::build::args::BuildArgs),
    /// Validates Rust component declarations and Vite files.
    #[cfg(feature = "check")]
    Check(crate::check::args::CheckArgs),
    /// Generates TypeScript contracts for typed Inertia props.
    #[cfg(feature = "sync")]
    Sync(crate::sync::args::SyncArgs),
}

/// Normalizes both `cargo inertia …` and `cargo-inertia …` invocations, then runs the command.
pub fn run() -> Result<(), CliError> {
    let mut arguments = std::env::args().collect::<Vec<_>>();
    if arguments
        .get(1)
        .is_some_and(|argument| argument == "inertia")
    {
        arguments.remove(1);
    }
    match Cli::parse_from(arguments).command {
        #[cfg(feature = "init")]
        Command::Init(args) => crate::init::run_args(args, &mut std::io::stdout().lock()),
        #[cfg(feature = "dev")]
        Command::Dev(args) => crate::dev::run_args(args),
        #[cfg(feature = "build")]
        Command::Build(args) => crate::build::run_args(args, &mut std::io::stdout().lock()),
        #[cfg(feature = "check")]
        Command::Check(args) => crate::check::run_args(args).map_err(Into::into),
        #[cfg(feature = "sync")]
        Command::Sync(args) => crate::sync::run_args(args).map_err(Into::into),
    }
}

#[cfg(all(test, feature = "build"))]
mod tests {
    use std::ffi::OsString;

    use super::*;

    #[test]
    fn build_parsing_keeps_every_argument_after_the_separator() {
        let cli = Cli::try_parse_from([
            "cargo-inertia",
            "build",
            "--release",
            "--package",
            "server",
            "--target",
            "x86_64-unknown-linux-musl",
            "--",
            "--features",
            "ssr",
            "--no-default-features",
        ])
        .unwrap();
        let Command::Build(args) = cli.command else {
            panic!("expected build command");
        };
        assert!(args.release);
        assert_eq!(args.package.as_deref(), Some("server"));
        assert_eq!(args.target.as_deref(), Some("x86_64-unknown-linux-musl"));
        assert_eq!(
            args.cargo_args,
            ["--features", "ssr", "--no-default-features"].map(OsString::from)
        );
    }
}
