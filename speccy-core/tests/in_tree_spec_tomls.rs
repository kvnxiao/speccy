#![expect(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic,
    reason = "tests panic to surface fixture parse failures"
)]

//! Workspace-scope assertion: every in-tree `.speccy/specs/**/spec.toml`
//! parses under the post-SPEC-0018 schema and every `[[checks]]` row
//! carries exactly `id` and `scenario`. The `deny_unknown_fields`
//! contract on `RawCheck` means a successful parse already implies the
//! absence of legacy `kind`, `command`, `prompt`, or `proves` fields,
//! but we also assert each scenario is non-empty as a belt-and-braces
//! check.

use camino::Utf8PathBuf;
use speccy_core::parse::spec_toml;
use std::path::Path;

fn workspace_root() -> Utf8PathBuf {
    // Cargo runs integration tests with CWD set to the crate dir.
    let crate_dir = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize_utf8()
        .expect("speccy-core crate manifest dir should canonicalize");
    crate_dir
        .parent()
        .expect("speccy-core sits one level under the workspace root")
        .to_path_buf()
}

#[test]
fn every_in_tree_spec_toml_parses_under_new_schema() {
    let root = workspace_root();
    let specs_dir = root.join(".speccy").join("specs");
    let mut found: u32 = 0;
    for entry in std::fs::read_dir(Path::new(specs_dir.as_str())).expect("specs dir should exist") {
        let entry = entry.expect("dir entry should read");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let spec_toml_path = path.join("spec.toml");
        if !spec_toml_path.exists() {
            continue;
        }
        let utf8 =
            Utf8PathBuf::from_path_buf(spec_toml_path).expect("spec.toml path should be UTF-8");
        let parsed =
            spec_toml(&utf8).unwrap_or_else(|e| panic!("{utf8} must parse under new schema: {e}"));
        for check in &parsed.checks {
            assert!(
                !check.scenario.trim().is_empty(),
                "{utf8}: {} has empty scenario",
                check.id,
            );
        }
        found = found.saturating_add(1);
    }
    assert!(
        found >= 1,
        "expected at least one in-tree spec.toml under {specs_dir}",
    );
}
