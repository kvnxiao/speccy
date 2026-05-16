#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]

//! T-004 integration test: every `.speccy/specs/*/SPEC.md` parses with the
//! marker parser, and no `spec.toml` files remain in the workspace.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::parse::spec_markers;

fn workspace_root() -> Utf8PathBuf {
    // CARGO_MANIFEST_DIR is `speccy-core`; parent is the workspace root.
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let manifest = Utf8PathBuf::from(manifest_dir);
    manifest
        .parent()
        .expect("speccy-core has a parent")
        .to_path_buf()
}

fn spec_dirs(root: &Utf8Path) -> Vec<Utf8PathBuf> {
    let specs_dir = root.join(".speccy").join("specs");
    let mut out = Vec::new();
    for entry in fs_err::read_dir(specs_dir.as_std_path()).expect("read .speccy/specs") {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        let utf8 =
            Utf8PathBuf::from_path_buf(path).expect("non-utf8 spec dir name should not exist");
        if utf8.is_dir() && utf8.join("SPEC.md").is_file() {
            out.push(utf8);
        }
    }
    out.sort();
    out
}

#[test]
fn every_in_tree_spec_md_parses_with_marker_parser() {
    let root = workspace_root();
    let dirs = spec_dirs(&root);
    assert!(
        !dirs.is_empty(),
        "expected at least one spec under .speccy/specs/",
    );
    let mut failed: Vec<String> = Vec::new();
    for d in &dirs {
        let spec_md_path = d.join("SPEC.md");
        let source = fs_err::read_to_string(spec_md_path.as_std_path())
            .expect("reading SPEC.md should succeed");
        if let Err(e) = spec_markers::parse(&source, &spec_md_path) {
            failed.push(format!("{spec_md_path}: {e}"));
        }
    }
    assert!(
        failed.is_empty(),
        "SPEC.md files failed to parse:\n{}",
        failed.join("\n"),
    );
}

#[test]
fn no_spec_toml_files_remain_under_speccy_specs() {
    let root = workspace_root();
    let dirs = spec_dirs(&root);
    let mut stray: Vec<Utf8PathBuf> = Vec::new();
    for d in &dirs {
        let candidate = d.join("spec.toml");
        if candidate.exists() {
            stray.push(candidate);
        }
    }
    assert!(
        stray.is_empty(),
        "stray spec.toml files remain after migration: {stray:?}",
    );
}
