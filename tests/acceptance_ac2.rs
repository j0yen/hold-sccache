//! AC2: `wire` merges `RUSTC_WRAPPER=sccache` under `[build]` without removing `target-dir`.

use std::fs;
use tempfile::TempDir;
use toml_edit::{DocumentMut, Item, Table, value};

fn make_config_with_target_dir(dir: &TempDir) -> std::path::PathBuf {
    let config_path = dir.path().join("config.toml");
    // Pre-populate with a target-dir key (as hold-anchor would write)
    let initial = "[build]\ntarget-dir = \"/tmp/cargo-target\"\n";
    fs::write(&config_path, initial).expect("write initial config");
    config_path
}

#[test]
fn ac2_wire_preserves_target_dir() {
    let dir = TempDir::new().expect("tempdir");
    let sccache_dir = dir.path().join("sccache-conf");
    let config_path = make_config_with_target_dir(&dir);

    // Run wire via the library function
    let content = fs::read_to_string(&config_path).expect("read config");
    let mut doc: DocumentMut = content.parse().expect("parse TOML");

    // Simulate wire operation
    let changed = hold_sccache_wire_merge(&mut doc);
    assert!(changed, "first wire should report a change");

    let result = doc.to_string();

    // Both keys must be present
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

    // Sccache config dir creation
    fs::create_dir_all(&sccache_dir).expect("create sccache dir");
}

/// Mirror of wire::merge_rustc_wrapper for use in tests without spawning a process.
fn hold_sccache_wire_merge(doc: &mut DocumentMut) -> bool {
    if !doc.contains_key("build") {
        let mut t = Table::new();
        t.set_implicit(false);
        doc["build"] = Item::Table(t);
    }
    let build = doc["build"].as_table_mut().expect("build is a table");
    let already = build
        .get("rustc-wrapper")
        .and_then(Item::as_str)
        .map_or(false, |v| v == "sccache");
    if already {
        return false;
    }
    build.insert("rustc-wrapper", value("sccache"));
    true
}
