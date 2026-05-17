#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for the default text view's "in-progress + broken" filter.
//! Covers SPEC-0004 CHK-007.

mod common;

use common::TestResult;
use common::Workspace;
use common::bootstrap_tasks_md;
use common::spec_md_template;
use common::write_spec;
use speccy_cli::status::StatusArgs;
use speccy_cli::status::run;

fn render_text(root: &camino::Utf8Path) -> TestResult<String> {
    let mut buf: Vec<u8> = Vec::new();
    run(
        &StatusArgs {
            selector: None,
            all: false,
            json: false,
        },
        root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf)?)
}

#[test]
fn in_progress_specs_are_always_shown() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        "",
        None,
    )?;

    let text = render_text(&ws.root)?;
    assert!(text.contains("SPEC-0001"));
    assert!(text.contains("in-progress"));
    Ok(())
}

#[test]
fn clean_implemented_specs_are_hidden() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        "",
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-done",
        &spec_md_template("SPEC-0002", "implemented"),
        "",
        None,
    )?;

    let text = render_text(&ws.root)?;
    assert!(text.contains("SPEC-0001"));
    assert!(
        !text.contains("SPEC-0002"),
        "implemented + clean spec should be hidden, got:\n{text}",
    );
    Ok(())
}

#[test]
fn stale_implemented_spec_is_shown() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-done",
        &spec_md_template("SPEC-0001", "implemented"),
        "",
        // bootstrap-pending makes it stale.
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;

    let text = render_text(&ws.root)?;
    assert!(
        text.contains("SPEC-0001"),
        "stale implemented spec should be shown, got:\n{text}",
    );
    assert!(
        text.contains("bootstrap-pending"),
        "stale reason should appear, got:\n{text}",
    );
    Ok(())
}

#[test]
fn implemented_with_lint_error_is_shown() -> TestResult {
    let ws = Workspace::new()?;
    // SPEC-0019: a stray per-spec spec.toml fires SPC-001.
    let dir = ws.root.join(".speccy").join("specs").join("0001-broken");
    fs_err::create_dir_all(dir.as_std_path())?;
    fs_err::write(
        dir.join("SPEC.md").as_std_path(),
        spec_md_template("SPEC-0001", "implemented"),
    )?;
    fs_err::write(dir.join("spec.toml").as_std_path(), "schema_version = 1\n")?;

    let text = render_text(&ws.root)?;
    assert!(
        text.contains("SPEC-0001"),
        "spec with lint errors must be shown, got:\n{text}",
    );
    Ok(())
}
