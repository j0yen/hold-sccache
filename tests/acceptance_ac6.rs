//! AC6: `stats` parses a fixture `sccache --show-stats` output into JSON with
//! `cache_hits`, `cache_misses`, and a computed `hit_rate` in [0,1].

use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn binary() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-sccache").into()
}

/// A representative sample of `sccache --show-stats` output.
const FIXTURE: &str = "\
Compile requests                      100
Compile requests executed              98
Cache hits                             80
Cache misses                           18
Cache hits (C/C++)                      0
Cache hits (Rust)                      80
Cache misses (Rust)                    18
Cache timeouts                          0
Cache read errors                       0
Forced recaches                         0
Cache write errors                      0
Compilations                            0
Errors                                  0
Cache location                       Local disk: \"/home/user/.cache/sccache\"
Cache size                            2.1 GiB
Max cache size                       20.0 GiB
";

#[test]
fn ac6_stats_from_fixture_has_required_fields() {
    let mut fixture_file = NamedTempFile::new().expect("tempfile");
    write!(fixture_file, "{FIXTURE}").expect("write fixture");

    let out = Command::new(binary())
        .args([
            "stats",
            "--fixture",
            fixture_file.path().to_str().expect("path"),
        ])
        .output()
        .expect("run stats");

    assert!(
        out.status.success(),
        "stats should exit 0; got {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).expect("UTF-8 stdout");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stats output must be valid JSON");

    let hits = parsed["cache_hits"].as_u64().expect("cache_hits must be u64");
    let misses = parsed["cache_misses"].as_u64().expect("cache_misses must be u64");
    let hit_rate = parsed["hit_rate"].as_f64().expect("hit_rate must be f64");

    assert_eq!(hits, 80, "cache_hits should be 80");
    assert_eq!(misses, 18, "cache_misses should be 18");

    // hit_rate must be in [0, 1]
    assert!(
        (0.0..=1.0).contains(&hit_rate),
        "hit_rate must be in [0.0, 1.0], got {hit_rate}"
    );

    // Expected: 80 / 98 ≈ 0.816
    let expected = 80.0_f64 / 98.0_f64;
    assert!(
        (hit_rate - expected).abs() < 1e-6,
        "hit_rate should be {expected:.6}, got {hit_rate:.6}"
    );
}

#[test]
fn ac6_stats_zero_requests_gives_zero_hit_rate() {
    let fixture = "Cache hits    0\nCache misses    0\n";
    let mut f = NamedTempFile::new().expect("tempfile");
    write!(f, "{fixture}").expect("write");

    let out = Command::new(binary())
        .args(["stats", "--fixture", f.path().to_str().expect("path")])
        .output()
        .expect("run stats");

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).expect("UTF-8");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("JSON");
    let hit_rate = parsed["hit_rate"].as_f64().expect("hit_rate");
    assert_eq!(hit_rate, 0.0, "zero requests → hit_rate = 0.0");
}
