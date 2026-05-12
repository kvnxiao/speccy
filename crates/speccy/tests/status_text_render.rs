#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for the text renderer: empty workspace and a single-spec
//! workspace. Covers SPEC-0004 CHK-008.

mod common;

use common::TestResult;
use common::Workspace;
use common::bootstrap_tasks_md;
use common::spec_md_with_open_questions;
use common::valid_spec_toml;
use common::write_spec;
use speccy::status::StatusArgs;
use speccy::status::run;

fn render_text(root: &camino::Utf8Path) -> TestResult<String> {
    let mut buf: Vec<u8> = Vec::new();
    run(StatusArgs { json: false }, root, &mut buf)?;
    Ok(String::from_utf8(buf)?)
}

#[test]
fn empty_and_single_spec() -> TestResult {
    // Empty workspace prints the empty banner.
    let ws = Workspace::new()?;
    let text = render_text(&ws.root)?;
    assert_eq!(text, "No specs in workspace.\n");

    // Single-spec workspace renders header plus per-line summaries.
    let ws2 = Workspace::new()?;
    write_spec(
        &ws2.root,
        "0001-foo",
        &spec_md_with_open_questions("SPEC-0001", "in-progress", 2),
        &valid_spec_toml(),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;

    let text = render_text(&ws2.root)?;
    assert!(
        text.contains("SPEC-0001 in-progress: Example SPEC-0001"),
        "expected one-line header, got:\n{text}",
    );
    assert!(text.contains("tasks: 1 open"));
    assert!(text.contains("open questions: 2"));
    assert!(text.contains("stale:"));
    assert!(text.contains("bootstrap-pending"));
    Ok(())
}

#[test]
fn workspace_lint_block_appears_at_end_when_present() -> TestResult {
    let ws = Workspace::new()?;
    let spec_md = "---\nid: SPEC-0001\nslug: x\ntitle: y\nstatus: in-progress\ncreated: 2026-05-11\nsupersedes: [\"SPEC-9999\"]\n---\n\n# SPEC-0001\n\n### REQ-001: First\n".to_owned();
    write_spec(&ws.root, "0001-x", &spec_md, &valid_spec_toml(), None)?;

    let text = render_text(&ws.root)?;
    assert!(text.contains("Workspace lint:"));
    assert!(text.contains("SPEC-9999"));
    Ok(())
}
