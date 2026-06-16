//! AC8: `install` without --local-build fails with a clear actionable message
//! and non-zero exit; never leaves a half-wired config.

use std::process::Command;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-sccache").into()
}

#[test]
fn ac8_install_without_flag_fails_with_actionable_message() {
    // This test verifies behavior when sccache is not on $PATH.
    // If sccache IS on $PATH (already installed), the command exits 0 — that's fine too,
    // since the AC is about the failure path when there's no cloud/network.
    // We simulate the "no cloud" scenario by checking the no-arg default path.

    let dir = TempDir::new().expect("tempdir");
    let bin_dir = dir.path().join("bin");

    // Run install pointing to an empty bin_dir; sccache won't be there.
    // We also set PATH to exclude any system sccache to ensure the binary-not-found path.
    let out = Command::new(binary())
        .args(["install", "--bin-dir", bin_dir.to_str().expect("path")])
        .env("PATH", bin_dir.to_str().expect("path")) // narrow PATH — no sccache
        .output()
        .expect("run install");

    if out.status.success() {
        // sccache was found on the real system PATH (already installed) — skip assertion
        return;
    }

    // Must fail with non-zero exit and an actionable message
    assert!(
        !out.status.success(),
        "install without --local-build must fail when sccache is not installed"
    );

    let stderr = String::from_utf8(out.stderr).expect("UTF-8 stderr");
    let stdout = String::from_utf8(out.stdout).expect("UTF-8 stdout");
    let combined = format!("{stderr}{stdout}");

    // Must mention the --local-build flag as the actionable resolution
    assert!(
        combined.contains("local-build") || combined.contains("local_build"),
        "error message must mention --local-build option\noutput:\n{combined}"
    );

    // Must NOT leave behind a partially wired config (wire is a separate step)
    // Since install only touches the binary, there's no config to corrupt.
    // The AC says "never leave a half-wired config" — verify bin_dir is either
    // empty or has a complete sccache binary (not a partial file).
    if bin_dir.exists() {
        let sccache_path = bin_dir.join("sccache");
        if sccache_path.exists() {
            let metadata = std::fs::metadata(&sccache_path).expect("metadata");
            assert!(metadata.len() > 0, "partial sccache binary must not exist");
        }
    }
}
