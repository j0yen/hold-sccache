//! `status` subcommand: report installation and wiring state.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use crate::wire::{default_fleet_config, default_sccache_config_dir, load_or_create_doc};

/// Report sccache installation and wiring status.
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Path to the fleet cargo config.toml (default: ~/wintermute/.cargo/config.toml).
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Path to the sccache config directory (default: ~/.config/sccache).
    #[arg(long)]
    pub sccache_config_dir: Option<PathBuf>,
}

/// Status output structure.
#[derive(Debug, Serialize)]
pub struct StatusOutput {
    /// Whether sccache binary is found on $PATH.
    pub installed: bool,
    /// Whether RUSTC_WRAPPER=sccache is present in the fleet config.
    pub wired: bool,
    /// Current cache size in bytes (None if unknown or not installed).
    pub cache_bytes: Option<u64>,
    /// Maximum cache size in bytes (None if not configured).
    pub max_bytes: Option<u64>,
    /// Path to the fleet config file checked.
    pub config_path: String,
    /// Path to the sccache config file checked.
    pub sccache_config_path: String,
}

/// Parse the sccache config to read max_size as bytes.
fn read_max_size(config_dir: &Path) -> Option<u64> {
    let config_path = config_dir.join("config");
    let text = std::fs::read_to_string(&config_path).ok()?;
    // Parse TOML manually for the size field under [cache.disk]
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("size") {
            // "size = \"20G\""
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == '=').trim();
            let size_str = rest.trim_matches('"');
            return parse_size_str(size_str);
        }
    }
    None
}

/// Parse a size string like "20G", "10G", "512M" into bytes.
fn parse_size_str(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    let (num_part, unit) = s.split_at(s.len() - 1);
    let n: u64 = num_part.parse().ok()?;
    #[allow(clippy::as_conversions)]
    match unit.to_uppercase().as_str() {
        "G" => Some(n * 1_024 * 1_024 * 1_024),
        "M" => Some(n * 1_024 * 1_024),
        "K" => Some(n * 1_024),
        "B" => Some(n),
        _ => None,
    }
}

/// Run the `status` subcommand.
///
/// # Errors
/// Returns an error if JSON cannot be serialized or written to stdout.
pub fn run(args: StatusArgs) -> Result<()> {
    let config_path = match args.config {
        Some(p) => p,
        None => default_fleet_config().unwrap_or_else(|_| PathBuf::from("~/.cargo/config.toml")),
    };
    let sccache_config_dir = match args.sccache_config_dir {
        Some(p) => p,
        None => default_sccache_config_dir()
            .unwrap_or_else(|_| PathBuf::from("~/.config/sccache")),
    };

    // Check if sccache binary is on $PATH
    let installed = which_sccache();

    // Check if RUSTC_WRAPPER is wired
    let wired = check_wired(&config_path);

    // Read max_size from sccache config
    let max_bytes = read_max_size(&sccache_config_dir);

    // For cache_bytes we'd need to run sccache --show-stats, but we degrade gracefully
    // if sccache is not installed — just report None.
    let cache_bytes = if installed {
        read_cache_bytes()
    } else {
        None
    };

    let status = StatusOutput {
        installed,
        wired,
        cache_bytes,
        max_bytes,
        config_path: config_path.display().to_string(),
        sccache_config_path: sccache_config_dir.join("config").display().to_string(),
    };

    let json = serde_json::to_string_pretty(&status).context("serializing status to JSON")?;
    #[allow(clippy::print_stdout)]
    {
        println!("{json}");
    }
    Ok(())
}

/// Returns true if `sccache` binary is found on $PATH.
fn which_sccache() -> bool {
    std::process::Command::new("sccache")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Returns true if `rustc-wrapper = "sccache"` is present under `[build]` in the config.
fn check_wired(config_path: &Path) -> bool {
    if !config_path.exists() {
        return false;
    }
    load_or_create_doc(config_path)
        .ok()
        .and_then(|doc| {
            doc.get("build")
                .and_then(toml_edit::Item::as_table)
                .and_then(|b| b.get("rustc-wrapper"))
                .and_then(toml_edit::Item::as_str)
                .map(|v| v == "sccache")
        })
        .unwrap_or(false)
}

/// Try to read cache bytes from live sccache stats. Returns None on any failure.
fn read_cache_bytes() -> Option<u64> {
    let output = std::process::Command::new("sccache")
        .arg("--show-stats")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    crate::stats::parse_stats(&text).ok()?.cache_size
}
