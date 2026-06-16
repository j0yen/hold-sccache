//! `wire` and `unwire` subcommands: manage RUSTC_WRAPPER in fleet config.toml.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use toml_edit::{DocumentMut, Item, Table, value};

/// Wire RUSTC_WRAPPER=sccache into the fleet cargo config.
#[derive(Debug, Args)]
pub struct WireArgs {
    /// Maximum sccache cache size (e.g. "20G", "10G").
    #[arg(long, default_value = "20G")]
    pub max_size: String,

    /// Path to the fleet cargo config.toml (default: ~/wintermute/.cargo/config.toml).
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Path to the sccache config directory (default: ~/.config/sccache).
    #[arg(long)]
    pub sccache_config_dir: Option<PathBuf>,
}

/// Unwire RUSTC_WRAPPER from the fleet cargo config.
#[derive(Debug, Args)]
pub struct UnwireArgs {
    /// Path to the fleet cargo config.toml (default: ~/wintermute/.cargo/config.toml).
    #[arg(long)]
    pub config: Option<PathBuf>,
}

/// Default path to the fleet cargo config.
///
/// # Errors
/// Returns an error if the home directory cannot be determined.
pub fn default_fleet_config() -> Result<PathBuf> {
    let home = home_dir().context("cannot determine home directory")?;
    Ok(home.join("wintermute").join(".cargo").join("config.toml"))
}

/// Default path to the sccache config directory.
///
/// # Errors
/// Returns an error if the home directory cannot be determined.
pub fn default_sccache_config_dir() -> Result<PathBuf> {
    let home = home_dir().context("cannot determine home directory")?;
    Ok(home.join(".config").join("sccache"))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Load or create a TOML document from a file path.
///
/// # Errors
/// Returns an error if the file exists but cannot be read or parsed.
pub fn load_or_create_doc(path: &Path) -> Result<DocumentMut> {
    if path.exists() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        content
            .parse::<DocumentMut>()
            .with_context(|| format!("parsing TOML at {}", path.display()))
    } else {
        Ok(DocumentMut::new())
    }
}

/// Write a sccache disk cache config.
///
/// # Errors
/// Returns an error if the config directory cannot be created or the file cannot be written.
pub fn write_sccache_config(config_dir: &Path, max_size: &str) -> Result<()> {
    fs::create_dir_all(config_dir)
        .with_context(|| format!("creating sccache config dir {}", config_dir.display()))?;

    let config_path = config_dir.join("config");
    // sccache TOML config format
    let content = format!(
        "# sccache configuration — managed by hold-sccache\n\
         [cache.disk]\n\
         dir = \"{}\"\n\
         size = \"{max_size}\"\n",
        config_dir
            .parent()
            .unwrap_or(config_dir)
            .join("sccache")
            .display()
    );

    fs::write(&config_path, &content)
        .with_context(|| format!("writing sccache config to {}", config_path.display()))?;

    Ok(())
}

/// Merge `RUSTC_WRAPPER = "sccache"` into the `[build]` section without clobbering other keys.
///
/// # Errors
/// Returns an error if the config file cannot be read, parsed, or written.
pub fn merge_rustc_wrapper(doc: &mut DocumentMut) -> bool {
    // Ensure [build] table exists
    if !doc.contains_key("build") {
        let mut t = Table::new();
        t.set_implicit(false);
        doc["build"] = Item::Table(t);
    }

    let build = doc["build"]
        .as_table_mut()
        .expect("build is a table by construction");

    let already_set = build
        .get("rustc-wrapper")
        .and_then(Item::as_str)
        .map_or(false, |v| v == "sccache");

    if already_set {
        return false; // no change needed
    }

    build.insert("rustc-wrapper", value("sccache"));
    true
}

/// Run the `wire` subcommand.
///
/// # Errors
/// Returns an error if config files cannot be read or written.
pub fn run_wire(args: WireArgs) -> Result<()> {
    let config_path = match args.config {
        Some(p) => p,
        None => default_fleet_config()?,
    };
    let sccache_config_dir = match args.sccache_config_dir {
        Some(p) => p,
        None => default_sccache_config_dir()?,
    };

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating parent dir {}", parent.display()))?;
    }

    let mut doc = load_or_create_doc(&config_path)?;
    let changed = merge_rustc_wrapper(&mut doc);

    if changed {
        fs::write(&config_path, doc.to_string())
            .with_context(|| format!("writing {}", config_path.display()))?;
        #[allow(clippy::print_stdout)]
        {
            println!("wired: RUSTC_WRAPPER=sccache added to {}", config_path.display());
        }
    } else {
        #[allow(clippy::print_stdout)]
        {
            println!("already wired: no change to {}", config_path.display());
        }
    }

    // Write sccache size config
    write_sccache_config(&sccache_config_dir, &args.max_size)?;
    #[allow(clippy::print_stdout)]
    {
        println!(
            "sccache config written to {} (max_size={})",
            sccache_config_dir.join("config").display(),
            args.max_size
        );
    }

    Ok(())
}

/// Remove `rustc-wrapper` from the `[build]` section without touching other keys.
///
/// # Errors
/// Returns an error if the config file cannot be read, parsed, or written.
pub fn run_unwire(args: UnwireArgs) -> Result<()> {
    let config_path = match args.config {
        Some(p) => p,
        None => default_fleet_config()?,
    };

    if !config_path.exists() {
        #[allow(clippy::print_stdout)]
        {
            println!("not wired: {} does not exist", config_path.display());
        }
        return Ok(());
    }

    let mut doc = load_or_create_doc(&config_path)?;

    let removed = if let Some(build) = doc.get_mut("build").and_then(Item::as_table_mut) {
        build.remove("rustc-wrapper").is_some()
    } else {
        false
    };

    if removed {
        fs::write(&config_path, doc.to_string())
            .with_context(|| format!("writing {}", config_path.display()))?;
        #[allow(clippy::print_stdout)]
        {
            println!("unwired: rustc-wrapper removed from {}", config_path.display());
        }
    } else {
        #[allow(clippy::print_stdout)]
        {
            println!("already unwired: rustc-wrapper not present in {}", config_path.display());
        }
    }

    Ok(())
}
