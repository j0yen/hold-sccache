//! hold-sccache — manage a size-capped local sccache installation for the wintermute fleet.
//!
//! Installs and wires `sccache` as `RUSTC_WRAPPER` with a configurable size cap,
//! enabling parallel cargo builds across repos to share a single compilation cache
//! without the serialization penalty of a shared `CARGO_TARGET_DIR`.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod config;
mod install;
mod stats;
mod status;

#[derive(Parser)]
#[command(name = "hold-sccache", about = "Manage local sccache install + wiring")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ensure sccache binary is on PATH
    Install {
        /// Install locally via `cargo install sccache`
        #[arg(long)]
        local_build: bool,
        /// Installation directory (default: ~/.local/bin)
        #[arg(long)]
        install_dir: Option<String>,
        /// Destination directory for the sccache binary (default: ~/.local/bin)
        #[arg(long)]
        bin_dir: Option<String>,
    },
    /// Wire sccache into cargo config
    Wire {
        /// Maximum cache size (e.g. 20G, 10G)
        #[arg(long, default_value = "20G")]
        max_size: String,
        /// Path to the fleet cargo config.toml
        #[arg(long)]
        config: Option<String>,
        /// Path to the sccache config directory
        #[arg(long)]
        sccache_config_dir: Option<String>,
    },
    /// Remove sccache wiring from cargo config
    Unwire {
        /// Path to the fleet cargo config.toml
        #[arg(long)]
        config: Option<String>,
    },
    /// Show sccache statistics as JSON
    Stats {
        /// Read sccache stats from a file instead of running sccache --show-stats
        #[arg(long)]
        fixture: Option<String>,
    },
    /// Show sccache status
    Status {
        /// Path to the fleet cargo config.toml
        #[arg(long)]
        config: Option<String>,
        /// Path to the sccache config directory
        #[arg(long)]
        sccache_config_dir: Option<String>,
    },
}

fn main() {
    sigpipe::reset();
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Install { local_build, install_dir, bin_dir } => {
            // bin_dir takes precedence over install_dir
            let dir = bin_dir.or(install_dir);
            install::install(local_build, dir.as_deref())?;
        }
        Commands::Wire { max_size, config, sccache_config_dir } => {
            config::wire_cmd(&max_size, config.as_deref(), sccache_config_dir.as_deref())?;
        }
        Commands::Unwire { config } => {
            config::unwire_cmd(config.as_deref())?;
        }
        Commands::Stats { fixture } => {
            stats::show_stats(fixture.as_deref())?;
        }
        Commands::Status { config, sccache_config_dir } => {
            status::show_status(config.as_deref(), sccache_config_dir.as_deref())?;
        }
    }
    Ok(())
}
