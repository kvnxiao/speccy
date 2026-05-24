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
//! - `docs/ARCHITECTURE.md` mentions every shipped CLI subcommand, so a new
//!   subcommand cannot land without a doc update.
//! - `docs/ARCHITECTURE.md` pins the full `speccy next` priority ordering
//!   (`review > work > vet > ship`) and the workspace-form `no_active_specs`
//!   terminal signal, so an addition or rename in the priority chain forces
//!   a doc edit.

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

#[test]
fn architecture_md_lists_every_cli_subcommand() {
    let root = workspace_root();
    let arch = root.join("docs").join("ARCHITECTURE.md");
    let body = fs_err::read_to_string(arch.as_std_path()).expect("read docs/ARCHITECTURE.md");

    // Hardcoded mirror of the clap `Commands` enum in
    // `speccy-cli/src/main.rs`. When the CLI surface changes, update this
    // list AND the doc the test pins. The point is to force the second edit.
    let verbs = [
        "speccy init",
        "speccy status",
        "speccy next",
        "speccy check",
        "speccy verify",
        "speccy lock",
        "speccy vacancy",
        "speccy archive",
    ];

    for verb in verbs {
        assert!(
            body.contains(verb),
            "ARCHITECTURE.md must mention `{verb}` to keep the documented \
             CLI surface in sync with the shipped one. If the verb was \
             intentionally removed, update both `Commands` in \
             `speccy-cli/src/main.rs` and this test's verb list."
        );
    }
}

#[test]
fn architecture_md_pins_speccy_next_priority_ordering() {
    let root = workspace_root();
    let arch = root.join("docs").join("ARCHITECTURE.md");
    let body = fs_err::read_to_string(arch.as_std_path()).expect("read docs/ARCHITECTURE.md");

    // The live priority lives in `speccy-core::next::compute_for_spec`:
    // `review > work > vet > ship`, with `decompose` when TASKS.md is
    // absent. The doc must spell that out at least once, in order, so an
    // addition or rename in the chain forces a doc edit.
    assert!(
        body.contains("review > work > vet > ship"),
        "ARCHITECTURE.md must document the full `speccy next` priority \
         ordering as `review > work > vet > ship`. If the priority changed, \
         update `speccy-core/src/next.rs`, the doc, and this assertion."
    );
}

#[test]
fn architecture_md_pins_no_active_specs_terminal_signal() {
    let root = workspace_root();
    let arch = root.join("docs").join("ARCHITECTURE.md");
    let body = fs_err::read_to_string(arch.as_std_path()).expect("read docs/ARCHITECTURE.md");

    // Mirrors `WORKSPACE_TERMINAL_REASON` in
    // `speccy-cli/src/next_output.rs`. The workspace-form `speccy next`
    // exit-2 contract is the loop-stop signal that skills key off; the
    // doc must name the slug so harness authors can find it.
    assert!(
        body.contains("no_active_specs"),
        "ARCHITECTURE.md must mention the `no_active_specs` slug used by \
         `speccy next` (workspace form) as the workspace-level terminal \
         signal. If the slug changed, update \
         `speccy-cli/src/next_output.rs::WORKSPACE_TERMINAL_REASON`, the \
         doc, and this assertion."
    );
}
