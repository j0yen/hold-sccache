//! AC5: `unwire` removes only `RUSTC_WRAPPER`, is idempotent, does not delete cache.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-sccache").into()
}

#[test]
fn ac5_unwire_only_removes_rustc_wrapper() {
    let dir = TempDir::new().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let sccache_dir = dir.path().join("sccache-conf");

    // Start with a config that has both target-dir and rustc-wrapper
    let initial = "[build]\ntarget-dir = \"/tmp/cargo-target\"\nrustc-wrapper = \"sccache\"\n";
    fs::write(&config_path, initial).expect("write initial config");
    // Create fake sccache cache dir to verify it's not deleted
    fs::create_dir_all(&sccache_dir).expect("create sccache dir");
    fs::write(sccache_dir.join("config"), "# sccache config").expect("write sccache config");

    // Unwire
    let status = Command::new(binary())
        .args([
            "unwire",
            "--config",
            config_path.to_str().expect("path"),
        ])
        .status()
        .expect("run unwire");
    assert!(status.success(), "unwire should succeed: {status}");

    let result = fs::read_to_string(&config_path).expect("read after unwire");
    assert!(
        !result.contains("rustc-wrapper"),
        "rustc-wrapper should be removed\n{result}"
    );
    assert!(
        result.contains("target-dir"),
        "target-dir must still be present after unwire\n{result}"
    );
    assert!(
        result.contains("/tmp/cargo-target"),
        "target-dir value must be preserved\n{result}"
    );

    // Cache dir must not be deleted
    assert!(sccache_dir.exists(), "sccache config dir must not be deleted by unwire");
    assert!(sccache_dir.join("config").exists(), "sccache config file must not be deleted");

    // Idempotent: second unwire should succeed too
    let status = Command::new(binary())
        .args([
            "unwire",
            "--config",
            config_path.to_str().expect("path"),
        ])
        .status()
        .expect("run unwire (second)");
    assert!(status.success(), "second unwire should succeed: {status}");

    let result2 = fs::read_to_string(&config_path).expect("read after second unwire");
    assert_eq!(result, result2, "second unwire must leave config byte-identical");
}
