mod check;
mod dev;
mod init;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "cargo inertia",
    version,
    about = "Optional inertia-axum project tooling"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Creates a minimal Vite frontend.
    Init {
        #[arg(long)]
        frontend: Frontend,
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Runs Vite and cargo run together.
    Dev {
        #[arg(long, default_value = "frontend")]
        frontend: PathBuf,
        #[arg(long, default_value_t = 5173)]
        port: u16,
    },
    /// Validates Rust component declarations and Vite files.
    Check {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "frontend")]
        frontend: PathBuf,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Frontend {
    Svelte,
    React,
    Vue,
}

fn main() {
    let mut arguments = std::env::args().collect::<Vec<_>>();
    if arguments
        .get(1)
        .is_some_and(|argument| argument == "inertia")
    {
        arguments.remove(1);
    }
    let cli = Cli::parse_from(arguments);
    let result = match cli.command {
        Command::Init { frontend, path } => init::run(&path, frontend),
        Command::Dev { frontend, port } => dev::run(&frontend, port),
        Command::Check { path, frontend } => check::run(&path, &frontend),
    };
    if let Err(error) = result {
        eprintln!("cargo inertia: {error}");
        std::process::exit(1);
    }
}
