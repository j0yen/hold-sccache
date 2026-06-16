//! AC3: `wire` is idempotent — a second run leaves config.toml byte-identical.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-sccache").into()
}

#[test]
fn ac3_wire_is_idempotent() {
    let dir = TempDir::new().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let sccache_dir = dir.path().join("sccache-conf");

    let wire_args = [
        "wire",
        "--config",
        config_path.to_str().expect("path"),
        "--sccache-config-dir",
        sccache_dir.to_str().expect("path"),
        "--max-size",
        "20G",
    ];

    // First wire
    let status = Command::new(binary())
        .args(wire_args)
        .status()
        .expect("run wire (first)");
    assert!(status.success(), "first wire should succeed: {status}");

    let content_after_first = fs::read_to_string(&config_path).expect("read after first wire");

    // Second wire — must produce byte-identical output
    let status = Command::new(binary())
        .args(wire_args)
        .status()
        .expect("run wire (second)");
    assert!(status.success(), "second wire should succeed: {status}");

    let content_after_second = fs::read_to_string(&config_path).expect("read after second wire");

    assert_eq!(
        content_after_first, content_after_second,
        "second wire must leave config.toml byte-identical"
    );
}
