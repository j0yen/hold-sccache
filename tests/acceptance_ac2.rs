//! AC2: `wire` merges `RUSTC_WRAPPER=sccache` under `[build]` without removing `target-dir`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-sccache").into()
}

#[test]
fn ac2_wire_preserves_target_dir() {
    let dir = TempDir::new().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let sccache_dir = dir.path().join("sccache-conf");

    // Pre-populate with a target-dir key (as hold-anchor would write)
    fs::write(&config_path, "[build]\ntarget-dir = \"/tmp/cargo-target\"\n")
        .expect("write initial config");

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

    let result = fs::read_to_string(&config_path).expect("read after wire");

    assert!(
        result.contains("rustc-wrapper"),
        "rustc-wrapper key missing after wire\n{result}"
    );
    assert!(
        result.contains("target-dir"),
        "target-dir key was removed by wire!\n{result}"
    );
    assert!(
        result.contains("/tmp/cargo-target"),
        "target-dir value corrupted\n{result}"
    );
    assert!(
        result.contains("sccache"),
        "sccache value missing\n{result}"
    );
}
