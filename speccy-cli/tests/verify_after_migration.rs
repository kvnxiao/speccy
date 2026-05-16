#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]

//! SPEC-0019 T-004: `speccy verify` exits 0 against the migrated
//! workspace. This pins REQ-004's "Given the migrated workspace,
//! `speccy verify` exits 0" behavior bullet.
//!
//! Runs against the actual in-tree `.speccy/` after the T-004 migration
//! has removed every per-spec `spec.toml` and converted SPEC.md to the
//! marker-structured carrier.

use assert_cmd::Command;
use camino::Utf8PathBuf;

fn workspace_root() -> Utf8PathBuf {
    let manifest = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("speccy-cli has a parent")
        .to_path_buf()
}

#[test]
fn speccy_verify_exits_zero_on_migrated_in_tree_workspace() {
    let root = workspace_root();
    let assert = Command::cargo_bin("speccy")
        .expect("speccy binary should build")
        .arg("verify")
        .current_dir(root.as_std_path())
        .assert()
        .success();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("0 errors"),
        "expected `speccy verify` to report `0 errors`, got stdout: {stdout}",
    );
}
