#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for `speccy_core::prompt::load_agents_md`.
//! Covers SPEC-0005 REQ-004 via the public API.

use camino::Utf8PathBuf;
use speccy_core::prompt::load_agents_md;
use tempfile::TempDir;

fn make_tmp_root() -> (TempDir, Utf8PathBuf) {
    let dir = tempfile::tempdir().expect("tmpdir creation must succeed");
    let path =
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("tempdir path must be UTF-8");
    (dir, path)
}

#[test]
fn returns_file_content_verbatim_when_present() {
    let (_tmp, root) = make_tmp_root();
    fs_err::write(
        root.join("AGENTS.md").as_std_path(),
        "# Agents\nproject conventions go here\n",
    )
    .expect("AGENTS.md write must succeed");
    let out = load_agents_md(&root);
    assert_eq!(out, "# Agents\nproject conventions go here\n");
}

#[test]
fn missing_file_returns_marker_string() {
    let (_tmp, root) = make_tmp_root();
    let out = load_agents_md(&root);
    assert!(
        out.contains("AGENTS.md missing"),
        "missing marker should mention `AGENTS.md missing`, got: {out}",
    );
    assert!(
        out.contains("conventions not loaded"),
        "missing marker should mention conventions not loaded, got: {out}",
    );
}
