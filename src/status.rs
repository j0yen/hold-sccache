//! `status` subcommand: report installation and wiring status.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config;
use crate::install;

/// Status output structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    /// Whether sccache binary is found on `$PATH`.
    pub installed: bool,
    /// Whether `RUSTC_WRAPPER=sccache` is in the fleet cargo config.
    pub wired: bool,
    /// Current cache size in bytes (0 if unknown or not installed).
    pub cache_bytes: u64,
    /// Maximum cache size in bytes (None if not configured).
    pub max_bytes: Option<u64>,
    /// Path to the sccache binary (None if not found).
    pub binary_path: Option<String>,
}

fn home_dir() -> Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .context("HOME not set")
}

/// Show status, optionally overriding config paths.
///
/// # Errors
/// Returns an error if JSON cannot be serialized.
pub fn show_status(cargo_config: Option<&str>, sccache_config_dir: Option<&str>) -> Result<()> {
    let cargo_path: PathBuf = match cargo_config {
        Some(p) => PathBuf::from(p),
        None => home_dir()?.join("wintermute").join(".cargo").join("config.toml"),
    };
    let sccache_dir: PathBuf = match sccache_config_dir {
        Some(p) => PathBuf::from(p),
        None => home_dir()?.join(".config").join("sccache"),
    };

    let installed = install::is_installed();
    let binary_path = install::which_sccache().map(|p| p.display().to_string());

    let wired = config::is_wired_at(&cargo_path).unwrap_or(false);
    let max_bytes = config::get_max_size_dir(&sccache_dir).unwrap_or(None);

    let cache_bytes = if installed {
        get_cache_bytes().unwrap_or(0)
    } else {
        0
    };

    let status = Status {
        installed,
        wired,
        cache_bytes,
        max_bytes,
        binary_path,
    };

    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

fn get_cache_bytes() -> Result<u64> {
    use crate::stats;
    let output = std::process::Command::new("sccache")
        .arg("--show-stats")
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let parsed = stats::parse_stats(&text);
            Ok(parsed.cache_size)
        }
        _ => Ok(0),
    }
}
