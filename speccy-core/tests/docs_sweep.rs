#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]

//! Docs sweep integration tests.
//!
//! Asserts that:
//! - `docs/ARCHITECTURE.md` documents the raw XML element grammar by containing
//!   the canonical element names currently in the whitelist.
//! - `docs/ARCHITECTURE.md` pins the no-public-`speccy fmt` contract so the
//!   "What We Deliberately Don't Do" row cannot quietly vanish.

use camino::Utf8PathBuf;

fn workspace_root() -> Utf8PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let manifest = Utf8PathBuf::from(manifest_dir);
    manifest
        .parent()
        .expect("speccy-core has a parent")
        .to_path_buf()
}

#[test]
fn architecture_md_documents_xml_element_grammar() {
    let root = workspace_root();
    let arch = root.join("docs").join("ARCHITECTURE.md");
    let body = fs_err::read_to_string(arch.as_std_path()).expect("read docs/ARCHITECTURE.md");

    // ARCHITECTURE.md must teach the raw XML element grammar: every
    // element name in the live whitelist, plus the HTML5-disjointness
    // invariant.
    for needle in [
        "<requirement",
        "<scenario",
        "<decision",
        "<changelog",
        "<open-question",
        "HTML5",
    ] {
        assert!(
            body.contains(needle),
            "ARCHITECTURE.md must document the raw XML element grammar by \
             mentioning `{needle}`"
        );
    }
}

#[test]
fn architecture_md_pins_no_public_speccy_fmt() {
    let root = workspace_root();
    let arch = root.join("docs").join("ARCHITECTURE.md");
    let body = fs_err::read_to_string(arch.as_std_path()).expect("read docs/ARCHITECTURE.md");

    let has_pinning_line = body.lines().any(|line| line.contains("speccy fmt"));

    assert!(
        has_pinning_line,
        "ARCHITECTURE.md must contain at least one line that mentions \
         `speccy fmt`, pinning the no-public-formatter contract so a future \
         deletion of the \"What We Deliberately Don't Do\" row regresses loudly"
    );
}
