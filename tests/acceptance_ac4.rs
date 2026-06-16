//! AC4: `wire --max-size 20G` records a 20G cap that `status` reads back as `max_size`.

use std::process::Command;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-sccache").into()
}

const GIB_20: u64 = 20 * 1024 * 1024 * 1024;

#[test]
fn ac4_wire_max_size_readable_by_status() {
    let dir = TempDir::new().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let sccache_dir = dir.path().join("sccache-conf");

    // Wire with 20G cap
    let status = Command::new(binary())
        .args([
            "wire",
            "--config",
            config_path.to_str().expect("path"),
            "--sccache-config-dir",
            sccache_dir.to_str().expect("path"),
            "--max-size",
            "20G",
        ])
        .status()
        .expect("run wire");
    assert!(status.success(), "wire should succeed: {status}");

    // Status should read back max_size
    let out = Command::new(binary())
        .args([
            "status",
            "--config",
            config_path.to_str().expect("path"),
            "--sccache-config-dir",
            sccache_dir.to_str().expect("path"),
        ])
        .output()
        .expect("run status");
    assert!(out.status.success(), "status should exit 0: {}", out.status);

    let stdout = String::from_utf8(out.stdout).expect("UTF-8 stdout");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("status output must be valid JSON");

    let max_bytes = parsed["max_bytes"]
        .as_u64()
        .expect("max_bytes must be a u64 in status JSON");

    assert_eq!(
        max_bytes, GIB_20,
        "max_bytes should be 20GiB ({GIB_20}), got {max_bytes}"
    );
}
