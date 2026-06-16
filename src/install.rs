//! `install` subcommand: ensure sccache binary is present.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Args;

/// Ensure sccache binary is present at ~/.local/bin/sccache.
#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Build sccache locally via `cargo install sccache` instead of the cloud builder.
    /// Requires a working Rust toolchain. May take a long time.
    #[arg(long)]
    pub local_build: bool,

    /// Destination directory for the sccache binary (default: ~/.local/bin).
    #[arg(long)]
    pub bin_dir: Option<PathBuf>,
}

/// Run the `install` subcommand.
///
/// # Errors
/// Returns an error if sccache cannot be installed with a clear actionable message.
pub fn run(args: InstallArgs) -> Result<()> {
    let bin_dir = match args.bin_dir {
        Some(p) => p,
        None => default_bin_dir()?,
    };
    let sccache_bin = bin_dir.join("sccache");

    // Idempotent: skip if already present and functional.
    if sccache_bin.exists() {
        let version_ok = std::process::Command::new(&sccache_bin)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if version_ok {
            #[allow(clippy::print_stdout)]
            {
                println!("sccache already installed at {}", sccache_bin.display());
            }
            return Ok(());
        }
    }

    // Also check $PATH for a system-installed sccache.
    if which_on_path("sccache") {
        #[allow(clippy::print_stdout)]
        {
            println!("sccache found on $PATH; skipping install");
        }
        return Ok(());
    }

    if args.local_build {
        run_local_build(&bin_dir)
    } else {
        bail!(
            "sccache is not installed and cloud build was not requested.\n\
             \n\
             Options:\n\
             1. Run `hold-sccache install --local-build` to build sccache locally\n\
                via `cargo install sccache` (requires Rust toolchain; ~5 min).\n\
             2. Use the cloud builder: run `cloudbuild.sh up` then\n\
                `cargo install --root ~/.local sccache` on the cloud box.\n\
             3. Install via your package manager: `pacman -S sccache` (Arch Linux).\n\
             \n\
             Note: `wire` is a separate step and has NOT been run; no config was modified."
        )
    }
}

/// Run `cargo install sccache --root <bin_dir>`.
///
/// # Errors
/// Returns an error if cargo install fails.
fn run_local_build(bin_dir: &std::path::Path) -> Result<()> {
    // Ensure destination exists
    std::fs::create_dir_all(bin_dir)
        .with_context(|| format!("creating bin dir {}", bin_dir.display()))?;

    #[allow(clippy::print_stdout)]
    {
        println!("Building sccache locally (this may take several minutes)...");
    }

    let status = std::process::Command::new("cargo")
        .args(["install", "sccache", "--root"])
        .arg(bin_dir.parent().unwrap_or(bin_dir))
        .status()
        .context("running `cargo install sccache`; is cargo on $PATH?")?;

    if !status.success() {
        bail!(
            "`cargo install sccache` failed (exit {status}).\n\
             Check that your Rust toolchain is installed and `cargo` is on $PATH."
        );
    }

    let sccache_bin = bin_dir.join("sccache");
    #[allow(clippy::print_stdout)]
    {
        println!("sccache installed to {}", sccache_bin.display());
    }
    Ok(())
}

/// Default bin directory: ~/.local/bin
///
/// # Errors
/// Returns an error if $HOME is not set.
fn default_bin_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("$HOME not set; cannot determine default bin dir")?;
    Ok(home.join(".local").join("bin"))
}

/// Returns true if `name` is found anywhere on $PATH.
fn which_on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path).any(|dir| dir.join(name).exists())
        })
        .unwrap_or(false)
}
