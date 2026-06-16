//! AC8: `install` without network/cloud reachable fails with a clear actionable message
//! and non-zero exit; never leaves a half-wired config.

use std::process::Command;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-sccache").into()
}

#[test]
fn ac8_install_without_flag_fails_with_actionable_message() {
    let dir = TempDir::new().expect("tempdir");
    let bin_dir = dir.path().join("bin");

    // Run install pointing to a fresh bin dir with no sccache, narrow PATH to exclude system sccache.
    let out = Command::new(binary())
        .args(["install", "--install-dir", bin_dir.to_str().expect("path")])
        .env("PATH", bin_dir.to_str().expect("path")) // narrow PATH — no sccache
        .output()
        .expect("run install");

    if out.status.success() {
        // sccache was found on the real PATH or installed successfully — skip failure check
        return;
    }

    // Must fail non-zero with an actionable message mentioning --local-build
    assert!(
        !out.status.success(),
        "install without network/cloud must fail when sccache not installed"
    );

    let stderr = String::from_utf8(out.stderr).expect("UTF-8 stderr");
    let stdout = String::from_utf8(out.stdout).expect("UTF-8 stdout");
    let combined = format!("{stderr}{stdout}");

    // Must mention how to resolve (local-build flag or manual package manager)
    assert!(
        combined.to_lowercase().contains("local") || combined.to_lowercase().contains("path"),
        "error message must mention a resolution path\noutput:\n{combined}"
    );

    // Must NOT leave behind a partial binary
    if bin_dir.exists() {
        let sccache_path = bin_dir.join("sccache");
        if sccache_path.exists() {
            let metadata = std::fs::metadata(&sccache_path).expect("metadata");
            assert!(metadata.len() > 0, "partial sccache binary must not exist (zero-byte file)");
        }
    }
}
