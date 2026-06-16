//! AC1: `hold-sccache --help` lists `install`, `wire`, `unwire`, `stats`, `status`; exits 0.

use std::process::Command;

fn binary() -> std::path::PathBuf {
    env!("CARGO_BIN_EXE_hold-sccache").into()
}

#[test]
fn ac1_help_lists_all_subcommands() {
    let out = Command::new(binary())
        .arg("--help")
        .output()
        .expect("failed to run hold-sccache --help");

    assert!(out.status.success(), "exit code must be 0; got {}", out.status);

    let stdout = String::from_utf8(out.stdout).expect("stdout must be UTF-8");
    let lower = stdout.to_lowercase();

    for sub in &["install", "wire", "unwire", "stats", "status"] {
        assert!(
            lower.contains(sub),
            "--help output missing subcommand '{sub}'\nstdout:\n{stdout}"
        );
    }
}
