//! `stats` subcommand: parse `sccache --show-stats` output into JSON.

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

/// Parse sccache stats into JSON.
#[derive(Debug, Args)]
pub struct StatsArgs {
    /// Read sccache stats from a file instead of running `sccache --show-stats`.
    #[arg(long)]
    pub fixture: Option<PathBuf>,

    /// Read sccache stats from stdin.
    #[arg(long)]
    pub stdin: bool,
}

/// Parsed sccache statistics.
#[derive(Debug, Serialize)]
pub struct SccacheStats {
    /// Number of cache hits.
    pub cache_hits: u64,
    /// Number of cache misses.
    pub cache_misses: u64,
    /// Hit rate in [0.0, 1.0].
    pub hit_rate: f64,
    /// Current cache size in bytes (may be None if not reported).
    pub cache_size: Option<u64>,
    /// Maximum cache size in bytes (may be None if not reported).
    pub max_size: Option<u64>,
    /// Raw stats text for diagnostics.
    pub raw_lines: Vec<String>,
}

/// Parse `sccache --show-stats` text output.
///
/// # Errors
/// Returns an error if the text cannot be parsed into valid stats.
pub fn parse_stats(text: &str) -> Result<SccacheStats> {
    let mut cache_hits: u64 = 0;
    let mut cache_misses: u64 = 0;
    let mut cache_size: Option<u64> = None;
    let mut max_size: Option<u64> = None;
    let raw_lines: Vec<String> = text.lines().map(str::to_owned).collect();

    for line in text.lines() {
        let line = line.trim();

        // Match "Cache hits    42" or "Cache hits (C/C++)    42"
        if let Some(rest) = line.strip_prefix("Cache hits") {
            // Skip sub-categories like "Cache hits (C/C++)"
            if !rest.trim_start().starts_with('(') {
                if let Some(n) = parse_trailing_number(rest) {
                    cache_hits = n;
                }
            }
        } else if let Some(rest) = line.strip_prefix("Cache misses") {
            if !rest.trim_start().starts_with('(') {
                if let Some(n) = parse_trailing_number(rest) {
                    cache_misses = n;
                }
            }
        } else if line.starts_with("Cache size") && !line.contains("max") {
            // "Cache size                          1.00 GiB"
            cache_size = parse_size_bytes(line);
        } else if line.starts_with("Max cache size") {
            max_size = parse_size_bytes(line);
        }
    }

    let total = cache_hits + cache_misses;
    #[allow(clippy::float_arithmetic)]
    let hit_rate = if total == 0 {
        0.0_f64
    } else {
        cache_hits as f64 / total as f64
    };

    Ok(SccacheStats {
        cache_hits,
        cache_misses,
        hit_rate,
        cache_size,
        max_size,
        raw_lines,
    })
}

fn parse_trailing_number(s: &str) -> Option<u64> {
    s.split_whitespace().last()?.parse().ok()
}

/// Parse size strings like "1.00 GiB", "512 MiB", "20 GiB" into bytes.
fn parse_size_bytes(line: &str) -> Option<u64> {
    // Find the last two tokens: number + unit
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let unit = *parts.last()?;
    let num_str = parts.get(parts.len() - 2)?;
    let num: f64 = num_str.parse().ok()?;

    #[allow(clippy::float_arithmetic)]
    let bytes = match unit {
        "B" => num as u64,
        "KiB" => (num * 1024.0) as u64,
        "MiB" => (num * 1024.0 * 1024.0) as u64,
        "GiB" => (num * 1024.0 * 1024.0 * 1024.0) as u64,
        // Plain G/M/K suffixes (less common in sccache but handle gracefully)
        "G" => (num * 1_000_000_000.0) as u64,
        "M" => (num * 1_000_000.0) as u64,
        "K" => (num * 1_000.0) as u64,
        _ => return None,
    };
    Some(bytes)
}

/// Run the `stats` subcommand.
///
/// # Errors
/// Returns an error if sccache is not installed, stats cannot be parsed, or JSON cannot be emitted.
pub fn run(args: StatsArgs) -> Result<()> {
    let text = if let Some(fixture_path) = args.fixture {
        std::fs::read_to_string(&fixture_path)
            .with_context(|| format!("reading fixture {}", fixture_path.display()))?
    } else if args.stdin {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("reading stdin")?;
        buf
    } else {
        // Run live sccache
        let output = Command::new("sccache")
            .arg("--show-stats")
            .output()
            .context("running `sccache --show-stats`; is sccache installed and on $PATH?")?;

        if !output.status.success() {
            bail!(
                "`sccache --show-stats` exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        String::from_utf8(output.stdout).context("sccache output is not valid UTF-8")?
    };

    let stats = parse_stats(&text)?;
    let json = serde_json::to_string_pretty(&stats).context("serializing stats to JSON")?;
    #[allow(clippy::print_stdout)]
    {
        println!("{json}");
    }
    Ok(())
}
