use anyhow::{bail, Context, Result};
use std::path::PathBuf;

pub(crate) fn install(local_build: bool, install_dir: Option<&str>) -> Result<()> {
    let dir = resolve_install_dir(install_dir)?;
    std::fs::create_dir_all(&dir).context("Failed to create install directory")?;

    let dest = dir.join("sccache");

    if dest.exists() {
        println!("sccache already installed at {}", dest.display());
        return Ok(());
    }

    if local_build {
        install_local(&dest)?;
    } else {
        install_prebuilt(&dest)?;
    }

    println!("sccache installed at {}", dest.display());
    println!("Make sure {} is on your PATH", dir.display());
    Ok(())
}

fn resolve_install_dir(install_dir: Option<&str>) -> Result<PathBuf> {
    if let Some(d) = install_dir {
        return Ok(PathBuf::from(d));
    }
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".local").join("bin"))
}

fn install_prebuilt(dest: &std::path::Path) -> Result<()> {
    let status = std::process::Command::new("cargo")
        .args(["install", "sccache", "--root", "/tmp/sccache-install"])
        .status();

    match status {
        Ok(s) if s.success() => {
            let src = std::path::Path::new("/tmp/sccache-install/bin/sccache");
            if src.exists() {
                std::fs::copy(src, dest).context("Failed to copy sccache binary")?;
                Ok(())
            } else {
                bail!("cargo install succeeded but binary not found at expected path")
            }
        }
        Ok(s) => {
            bail!("cargo install sccache failed with exit code: {}", s.code().unwrap_or(-1))
        }
        Err(e) => {
            bail!(
                "Failed to run cargo install: {e}\n\
                 Hint: use --local-build to build sccache via cargo install, \
                 or install sccache manually and ensure it's on PATH"
            )
        }
    }
}

fn install_local(dest: &std::path::Path) -> Result<()> {
    let status = std::process::Command::new("cargo")
        .args(["install", "sccache", "--root", "/tmp/sccache-local-build"])
        .status()
        .context("Failed to run cargo install")?;

    if !status.success() {
        bail!("cargo install sccache failed");
    }

    let src = std::path::Path::new("/tmp/sccache-local-build/bin/sccache");
    std::fs::copy(src, dest).context("Failed to copy sccache binary")?;
    Ok(())
}

pub(crate) fn is_installed() -> bool {
    which_sccache().is_some()
}

pub(crate) fn which_sccache() -> Option<PathBuf> {
    if let Ok(paths) = std::env::var("PATH") {
        for dir in paths.split(':') {
            let candidate = PathBuf::from(dir).join("sccache");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
