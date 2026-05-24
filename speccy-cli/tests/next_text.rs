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
use common::sha256_hex;
use common::spec_md_template;
use common::task_xml;
use common::tasks_md_xml;
use common::write_spec;
use predicates::str::contains;
use speccy_cli::next::NextArgs;
use speccy_cli::next::run;

fn render_text(ws: &Workspace) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    run(
        &NextArgs {
            spec_id: None,
            json: false,
        },
        &ws.root,
        &mut buf,
        &mut err,
    )?;
    Ok(String::from_utf8(buf)?)
}

// -- CHK-009 ----------------------------------------------------------------

#[test]
fn one_line_per_active_spec() -> TestResult {
    // work: pending task.
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
        first_line.contains("SPEC-0001") && first_line.contains("work"),
        "line must contain SPEC-0001 and work: {text:?}",
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

    // vet: all done + no VET.md (the new lifecycle step between
    // completed tasks and ship — SPEC-0041 REQ-001/REQ-002).
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
        line4.contains("SPEC-0001") && line4.contains("vet"),
        "line must contain SPEC-0001 and vet (all-completed + no VET.md): {text4:?}",
    );

    // completed: all done + fresh-pass VET.md + REPORT.md → omitted
    // (zero lines).
    let ws5 = Workspace::new()?;
    let tasks_xml5 = task_xml("T-001", "completed");
    let tasks_md5 = tasks_md_xml("SPEC-0001", &tasks_xml5);
    let spec_dir = write_spec(
        &ws5.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md5),
    )?;
    // Compute SHA-256 of TASKS.md for the gate freshness signal.
    let hash = sha256_hex(tasks_md5.as_bytes());
    let journal = spec_dir.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    let vet = format!(
        "## Invocation 1\n\n<gate verdict=\"passed\" tasks_hash=\"{hash}\" date=\"2026-05-22T00:00:00Z\">\nstub.\n</gate>\n",
    );
    fs_err::write(journal.join("VET.md").as_std_path(), vet)?;
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
    // work.
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
        .stdout(contains("work"));

    Ok(())
}

#[test]
fn empty_workspace_exits_2_with_stderr_advisory() -> TestResult {
    // No specs at all → workspace-level terminal: exit 2, empty stdout,
    // friendly stderr line so an AI harness sees the loop-stop signal.
    let ws_empty = Workspace::new()?;
    Command::cargo_bin("speccy")?
        .arg("next")
        .current_dir(ws_empty.root.as_std_path())
        .assert()
        .code(2)
        .stdout(predicates::str::is_empty())
        .stderr(contains("no active specs"))
        .stderr(contains("no_active_specs"))
        .stderr(contains("speccy plan"));
    Ok(())
}

#[test]
fn per_spec_form_text_output() -> TestResult {
    // pending → "work T-001".
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
        .stdout(contains("work"))
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

// ---------------------------------------------------------------------------
// SPEC-0043 REQ-003: CLI exit code 2 for terminal per-spec resolutions.
// ---------------------------------------------------------------------------

#[test]
fn per_spec_terminal_completed_cli_exits_2() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "completed");
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    fs_err::write(
        spec_dir.join("REPORT.md").as_std_path(),
        "# Report\n\nstub.\n",
    )?;
    Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0001", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .code(2)
        .stderr(contains("SPEC-0001 is completed"))
        .stderr(contains("speccy archive SPEC-0001"))
        .stdout(contains("\"reason\":\"completed\""));
    Ok(())
}

#[test]
fn per_spec_non_terminal_cli_exits_0() -> TestResult {
    let ws = Workspace::new()?;
    let tasks_xml = task_xml("T-001", "pending");
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &tasks_xml)),
    )?;
    Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0001", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .code(0);
    Ok(())
}
