//! AC7: `status` reports `installed`, `wired`, `cache_bytes`, `max_bytes` and
//! exits 0 whether or not sccache is actually installed.

use std::process::Command;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-sccache").into()
}

#[test]
fn ac7_status_exits_0_with_all_fields_present() {
    let dir = TempDir::new().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let sccache_dir = dir.path().join("sccache-conf");

    // Don't create any config files — status must degrade gracefully.
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

    assert!(
        out.status.success(),
        "status must exit 0 even when sccache is not installed; got {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).expect("UTF-8 stdout");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("status output must be valid JSON");

    // All required fields must be present
    assert!(
        parsed.get("installed").is_some(),
        "status JSON missing 'installed' field"
    );
    assert!(
        parsed.get("wired").is_some(),
        "status JSON missing 'wired' field"
    );
    assert!(
        parsed.get("cache_bytes").is_some(),
        "status JSON missing 'cache_bytes' field"
    );
    assert!(
        parsed.get("max_bytes").is_some(),
        "status JSON missing 'max_bytes' field"
    );

    // On a fresh tmpdir: not wired, max_bytes is null
    assert_eq!(
        parsed["wired"].as_bool(),
        Some(false),
        "should not be wired on fresh config"
    );
    // max_bytes should be null when no sccache config exists
    assert!(
        parsed["max_bytes"].is_null(),
        "max_bytes should be null when no sccache config exists"
    );
}
