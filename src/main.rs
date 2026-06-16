//! hold-sccache — wire a size-capped local sccache compilation cache.
//!
//! Manages sccache installation and configuration for the wintermute fleet.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod install;
mod stats;
mod status;
mod wire;

/// Manage a size-capped sccache compilation cache for the wintermute fleet.
#[derive(Debug, Parser)]
#[command(name = "hold-sccache", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Ensure an sccache binary is present at ~/.local/bin/sccache.
    Install(install::InstallArgs),
    /// Merge RUSTC_WRAPPER=sccache into ~/wintermute/.cargo/config.toml.
    Wire(wire::WireArgs),
    /// Remove RUSTC_WRAPPER from the fleet config (idempotent).
    Unwire(wire::UnwireArgs),
    /// Parse `sccache --show-stats` output and emit JSON.
    Stats(stats::StatsArgs),
    /// Report whether sccache is installed and wired, and cache size vs cap.
    Status(status::StatusArgs),
}

fn main() {
    let cli = Cli::parse();
    let result = run(cli);
    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Install(args) => install::run(args),
        Commands::Wire(args) => wire::run_wire(args),
        Commands::Unwire(args) => wire::run_unwire(args),
        Commands::Stats(args) => stats::run(args),
        Commands::Status(args) => status::run(args),
    }
}
