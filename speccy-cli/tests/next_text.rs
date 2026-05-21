#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Text-output tests for `speccy next` (no `--json`).
//!
//! Covers SPEC-0007 CHK-009 (one line per active spec, exit code 0) and
//! SPEC-0033 REQ-004 (derived action kinds; workspace and per-spec text
//! format).

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::write_spec;
use predicates::str::contains;
use speccy_cli::next::NextArgs;
use speccy_cli::next::run;

fn tasks_md_xml(spec_id: &str, tasks_xml: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n\n\n{tasks_xml}\n\n",
    )
}

fn task_xml(id: &str, state: &str) -> String {
    format!(
        "<task id=\"{id}\" state=\"{state}\" covers=\"REQ-001\">\ndo the thing\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n",
    )
}

fn render_text(ws: &Workspace) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = Vec::new();
    run(
        &NextArgs {
            spec_id: None,
            json: false,
        },
        &ws.root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf)?)
}

// -- CHK-009 ----------------------------------------------------------------

#[test]
fn one_line_per_active_spec() -> TestResult {
    // implement: pending task.
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    let text = render_text(&ws)?;
    assert_eq!(text.lines().count(), 1, "expected 1 line: {text:?}");
    let first_line = text.lines().next().expect("checked count above");
    assert!(
        first_line.contains("SPEC-0001") && first_line.contains("implement"),
        "line must contain SPEC-0001 and implement: {text:?}",
    );
    assert!(
        first_line.contains("T-001"),
        "line must include the task id T-001: {text:?}",
    );

    // review: in-review task.
    let ws2 = Workspace::new()?;
    let tasks_xml2 = task_xml("T-002", "in-review");
    write_spec(
        &ws2.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml2)),
    )?;
    let text2 = render_text(&ws2)?;
    assert_eq!(text2.lines().count(), 1, "expected 1 line: {text2:?}");
    let line2 = text2.lines().next().expect("checked count above");
    assert!(
        line2.contains("SPEC-0001") && line2.contains("review"),
        "line must contain SPEC-0001 and review: {text2:?}",
    );

    // decompose: no TASKS.md.
    let ws3 = Workspace::new()?;
    write_spec(
        &ws3.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        None,
    )?;
    let text3 = render_text(&ws3)?;
    assert_eq!(text3.lines().count(), 1, "expected 1 line: {text3:?}");
    let line3 = text3.lines().next().expect("checked count above");
    assert!(
        line3.contains("SPEC-0001") && line3.contains("decompose"),
        "line must contain SPEC-0001 and decompose: {text3:?}",
    );

    // ship: all done + no REPORT.md.
    let ws4 = Workspace::new()?;
    let tasks_xml4 = task_xml("T-001", "completed");
    write_spec(
        &ws4.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml4)),
    )?;
    let text4 = render_text(&ws4)?;
    assert_eq!(text4.lines().count(), 1, "expected 1 line: {text4:?}");
    let line4 = text4.lines().next().expect("checked count above");
    assert!(
        line4.contains("SPEC-0001") && line4.contains("ship"),
        "line must contain SPEC-0001 and ship: {text4:?}",
    );

    // completed: all done + REPORT.md → omitted (zero lines).
    let ws5 = Workspace::new()?;
    let tasks_xml5 = task_xml("T-001", "completed");
    let spec_dir = write_spec(
        &ws5.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml5)),
    )?;
    fs_err::write(spec_dir.join("REPORT.md").as_std_path(), "# Report\n")?;
    let text5 = render_text(&ws5)?;
    assert!(
        text5.is_empty(),
        "completed spec must be omitted (empty output): {text5:?}",
    );
    Ok(())
}

#[test]
fn exit_code_is_zero_for_all_kinds() -> TestResult {
    // implement.
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    Command::cargo_bin("speccy")?
        .arg("next")
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .stdout(contains("implement"));

    // Empty workspace still exits 0 (no active specs → empty output).
    let ws_empty = Workspace::new()?;
    Command::cargo_bin("speccy")?
        .arg("next")
        .current_dir(ws_empty.root.as_std_path())
        .assert()
        .success();

    Ok(())
}

#[test]
fn per_spec_form_text_output() -> TestResult {
    // pending → "implement T-001".
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0001"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .stdout(contains("SPEC-0001"))
        .stdout(contains("implement"))
        .stdout(contains("T-001"));

    // no TASKS.md → "decompose".
    let ws2 = Workspace::new()?;
    write_spec(
        &ws2.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        None,
    )?;
    Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0001"])
        .current_dir(ws2.root.as_std_path())
        .assert()
        .success()
        .stdout(contains("SPEC-0001"))
        .stdout(contains("decompose"));

    Ok(())
}
