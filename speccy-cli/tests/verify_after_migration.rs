#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]

//! SPEC-0020 T-005 regression: `speccy verify` exits 0 against the
//! migrated workspace.
//!
//! T-004 migrated every in-tree `SPEC.md` to raw XML element form.
//! T-005 rewired `speccy-core/src/workspace.rs` to call
//! [`speccy_core::parse::parse_spec_xml`], so the workspace loader,
//! lint engine, and `speccy verify` now consume the post-SPEC-0020
//! element tree directly. The structural guarantee that every spec
//! parses cleanly lives in `speccy-core/tests/in_tree_specs.rs`
//! (`every_in_tree_spec_md_parses_with_xml_parser_and_matches_snapshot`);
//! this test exercises the live `speccy verify` binary against the
//! same workspace and asserts the gate reports `0 errors`.

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
