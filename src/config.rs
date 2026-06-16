use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Table, value};

fn home_dir() -> Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .context("HOME not set")
}

fn default_cargo_config_path() -> Result<PathBuf> {
    Ok(home_dir()?.join("wintermute").join(".cargo").join("config.toml"))
}

fn default_sccache_config_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".config").join("sccache"))
}

fn load_or_create_doc(path: &Path) -> Result<DocumentMut> {
    if path.exists() {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        content
            .parse::<DocumentMut>()
            .with_context(|| format!("parsing TOML at {}", path.display()))
    } else {
        Ok(DocumentMut::new())
    }
}

pub fn parse_max_size(s: &str) -> Result<u64> {
    if s.is_empty() {
        anyhow::bail!("empty size string");
    }
    let (num_part, unit) = s.split_at(s.len() - 1);
    let n: u64 = num_part
        .parse()
        .with_context(|| format!("invalid number in size string: {s}"))?;
    match unit.to_uppercase().as_str() {
        "G" => Ok(n * 1024 * 1024 * 1024),
        "M" => Ok(n * 1024 * 1024),
        "K" => Ok(n * 1024),
        "B" => Ok(n),
        _ => anyhow::bail!("unknown size unit in '{s}'; use G, M, K, or B"),
    }
}

/// Wire command with optional path overrides.
pub fn wire_cmd(max_size: &str, cargo_config: Option<&str>, sccache_config_dir: Option<&str>) -> Result<()> {
    let cargo_path = match cargo_config {
        Some(p) => PathBuf::from(p),
        None => default_cargo_config_path()?,
    };
    let sccache_dir = match sccache_config_dir {
        Some(p) => PathBuf::from(p),
        None => default_sccache_config_dir()?,
    };
    let sccache_path = sccache_dir.join("config");
    wire_to(&cargo_path, &sccache_path, max_size)
}

/// Unwire command with optional path override.
pub fn unwire_cmd(cargo_config: Option<&str>) -> Result<()> {
    let cargo_path = match cargo_config {
        Some(p) => PathBuf::from(p),
        None => default_cargo_config_path()?,
    };
    unwire_from(&cargo_path)
}

/// Check if wired using default paths.
#[allow(dead_code)]
pub fn is_wired() -> Result<bool> {
    let cargo_path = default_cargo_config_path()?;
    is_wired_at(&cargo_path)
}

/// Check if wired at a specific cargo config path.
pub fn is_wired_at(cargo_path: &Path) -> Result<bool> {
    if !cargo_path.exists() {
        return Ok(false);
    }
    let doc = load_or_create_doc(cargo_path)?;
    Ok(doc
        .get("build")
        .and_then(Item::as_table)
        .and_then(|b| b.get("rustc-wrapper"))
        .and_then(Item::as_str) == Some("sccache"))
}

/// Get max size from default sccache config path.
#[allow(dead_code)]
pub fn get_max_size() -> Result<Option<u64>> {
    let sccache_dir = default_sccache_config_dir()?;
    let sccache_path = sccache_dir.join("config");
    get_max_size_from(&sccache_path)
}

/// Get max size from a specific sccache config directory.
pub fn get_max_size_dir(sccache_config_dir: &Path) -> Result<Option<u64>> {
    let sccache_path = sccache_config_dir.join("config");
    get_max_size_from(&sccache_path)
}

fn wire_impl(cargo_path: &Path, sccache_path: &Path, max_size: &str) -> Result<()> {
    // Validate max_size
    let size_bytes = parse_max_size(max_size)?;

    // Ensure cargo config parent dir exists
    if let Some(parent) = cargo_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    // Load/create and update cargo config
    let mut doc = load_or_create_doc(cargo_path)?;
    if !doc.contains_key("build") {
        let mut t = Table::new();
        t.set_implicit(false);
        doc["build"] = Item::Table(t);
    }
    let build = doc["build"]
        .as_table_mut()
        .context("build is not a table")?;
    let already_set = build
        .get("rustc-wrapper")
        .and_then(Item::as_str) == Some("sccache");
    if !already_set {
        build.insert("rustc-wrapper", value("sccache"));
    }
    std::fs::write(cargo_path, doc.to_string())
        .with_context(|| format!("writing {}", cargo_path.display()))?;

    // Write sccache config with size
    if let Some(parent) = sccache_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    std::fs::write(sccache_path, size_bytes.to_string())
        .with_context(|| format!("writing sccache config to {}", sccache_path.display()))?;

    Ok(())
}

fn unwire_impl(cargo_path: &Path) -> Result<()> {
    if !cargo_path.exists() {
        return Ok(());
    }
    let mut doc = load_or_create_doc(cargo_path)?;
    if let Some(build) = doc.get_mut("build").and_then(Item::as_table_mut) {
        build.remove("rustc-wrapper");
    }
    std::fs::write(cargo_path, doc.to_string())
        .with_context(|| format!("writing {}", cargo_path.display()))?;
    Ok(())
}

fn get_max_size_impl(sccache_path: &Path) -> Result<Option<u64>> {
    if !sccache_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(sccache_path)
        .with_context(|| format!("reading {}", sccache_path.display()))?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let bytes: u64 = trimmed
        .parse()
        .with_context(|| format!("parsing size from {}", sccache_path.display()))?;
    Ok(Some(bytes))
}

#[cfg(test)]
pub fn wire_to(cargo_path: &Path, sccache_path: &Path, max_size: &str) -> Result<()> {
    wire_impl(cargo_path, sccache_path, max_size)
}

#[cfg(not(test))]
pub fn wire_to(cargo_path: &Path, sccache_path: &Path, max_size: &str) -> Result<()> {
    wire_impl(cargo_path, sccache_path, max_size)
}

#[cfg(test)]
pub fn unwire_from(cargo_path: &Path) -> Result<()> {
    unwire_impl(cargo_path)
}

#[cfg(not(test))]
pub fn unwire_from(cargo_path: &Path) -> Result<()> {
    unwire_impl(cargo_path)
}

#[cfg(test)]
pub fn get_max_size_from(sccache_path: &Path) -> Result<Option<u64>> {
    get_max_size_impl(sccache_path)
}

#[cfg(not(test))]
pub fn get_max_size_from(sccache_path: &Path) -> Result<Option<u64>> {
    get_max_size_impl(sccache_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_wire_empty_config() {
        let dir = TempDir::new().unwrap();
        let cargo_path = dir.path().join("config.toml");
        let sccache_path = dir.path().join("sccache_config");
        wire_to(&cargo_path, &sccache_path, "20G").unwrap();
        let content = std::fs::read_to_string(&cargo_path).unwrap();
        assert!(content.contains("sccache"));
    }

    #[test]
    fn test_wire_preserves_existing_keys() {
        let dir = TempDir::new().unwrap();
        let cargo_path = dir.path().join("config.toml");
        let sccache_path = dir.path().join("sccache_config");
        // Pre-populate with target-dir
        std::fs::write(&cargo_path, "[build]\ntarget-dir = \"/tmp/target\"\n").unwrap();
        wire_to(&cargo_path, &sccache_path, "20G").unwrap();
        let content = std::fs::read_to_string(&cargo_path).unwrap();
        assert!(content.contains("target-dir"));
        assert!(content.contains("sccache"));
    }

    #[test]
    fn test_wire_idempotent() {
        let dir = TempDir::new().unwrap();
        let cargo_path = dir.path().join("config.toml");
        let sccache_path = dir.path().join("sccache_config");
        wire_to(&cargo_path, &sccache_path, "20G").unwrap();
        let content1 = std::fs::read_to_string(&cargo_path).unwrap();
        wire_to(&cargo_path, &sccache_path, "20G").unwrap();
        let content2 = std::fs::read_to_string(&cargo_path).unwrap();
        assert_eq!(content1, content2);
    }

    #[test]
    fn test_unwire_removes_only_rustc_wrapper() {
        let dir = TempDir::new().unwrap();
        let cargo_path = dir.path().join("config.toml");
        let sccache_path = dir.path().join("sccache_config");
        std::fs::write(&cargo_path, "[build]\ntarget-dir = \"/tmp/target\"\n").unwrap();
        wire_to(&cargo_path, &sccache_path, "20G").unwrap();
        unwire_from(&cargo_path).unwrap();
        let content = std::fs::read_to_string(&cargo_path).unwrap();
        assert!(!content.contains("rustc-wrapper"));
        assert!(content.contains("target-dir"));
    }

    #[test]
    fn test_unwire_idempotent() {
        let dir = TempDir::new().unwrap();
        let cargo_path = dir.path().join("config.toml");
        let sccache_path = dir.path().join("sccache_config");
        wire_to(&cargo_path, &sccache_path, "20G").unwrap();
        unwire_from(&cargo_path).unwrap();
        let content1 = std::fs::read_to_string(&cargo_path).unwrap();
        unwire_from(&cargo_path).unwrap();
        let content2 = std::fs::read_to_string(&cargo_path).unwrap();
        assert_eq!(content1, content2);
    }

    #[test]
    fn test_wire_max_size_stored_and_readable() {
        let dir = TempDir::new().unwrap();
        let cargo_path = dir.path().join("config.toml");
        let sccache_path = dir.path().join("sccache_config");
        wire_to(&cargo_path, &sccache_path, "20G").unwrap();
        let max = get_max_size_from(&sccache_path).unwrap().unwrap();
        assert_eq!(max, 20 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_parse_size_g() {
        assert_eq!(parse_max_size("20G").unwrap(), 20 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_parse_size_m() {
        assert_eq!(parse_max_size("512M").unwrap(), 512 * 1024 * 1024);
    }
}
