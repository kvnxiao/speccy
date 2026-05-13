#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Text-output tests for `speccy next` (no `--json`). Covers SPEC-0007
//! CHK-009: one line per kind variant, exit code 0 for all kinds
//! (blocked is not an error).

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use predicates::str::contains;
use speccy_cli::next::NextArgs;
use speccy_cli::next::run;

fn tasks_md(spec_id: &str, body: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n{body}",
    )
}

fn render_text(ws: &Workspace) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = Vec::new();
    run(
        NextArgs {
            kind: None,
            json: false,
        },
        &ws.root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf)?)
}

// -- CHK-009 ----------------------------------------------------------------

#[test]
fn one_line_per_kind() -> TestResult {
    // implement
    let ws = Workspace::new()?;
    let body = "- [ ] **T-001**: do the thing\n  - Covers: REQ-001\n";
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md("SPEC-0001", body)),
    )?;
    let text = render_text(&ws)?;
    assert_eq!(
        text, "next: implement T-001 (SPEC-0001) -- do the thing\n",
        "unexpected implement text: {text:?}",
    );

    // review
    let ws2 = Workspace::new()?;
    let body2 = "- [?] **T-002**: review the thing\n  - Covers: REQ-001\n";
    write_spec(
        &ws2.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md("SPEC-0001", body2)),
    )?;
    let text2 = render_text(&ws2)?;
    assert_eq!(
        text2, "next: review T-002 (SPEC-0001) -- personas: business, tests, security, style\n",
        "unexpected review text: {text2:?}",
    );

    // report
    let ws3 = Workspace::new()?;
    let body3 = "- [x] **T-001**: done\n  - Covers: REQ-001\n";
    write_spec(
        &ws3.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md("SPEC-0001", body3)),
    )?;
    let text3 = render_text(&ws3)?;
    assert_eq!(
        text3, "next: report SPEC-0001 -- all tasks complete\n",
        "unexpected report text: {text3:?}",
    );

    // blocked
    let ws4 = Workspace::new()?;
    let text4 = render_text(&ws4)?;
    assert_eq!(
        text4, "next: blocked -- no specs in workspace\n",
        "unexpected blocked text: {text4:?}",
    );
    Ok(())
}

#[test]
fn exit_code_is_zero_for_all_kinds() -> TestResult {
    // implement
    let ws = Workspace::new()?;
    let body = "- [ ] **T-001**: do it\n  - Covers: REQ-001\n";
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md("SPEC-0001", body)),
    )?;
    Command::cargo_bin("speccy")?
        .arg("next")
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .stdout(contains("implement"));

    // blocked: empty workspace still exits 0.
    let ws_empty = Workspace::new()?;
    Command::cargo_bin("speccy")?
        .arg("next")
        .current_dir(ws_empty.root.as_std_path())
        .assert()
        .success()
        .stdout(contains("blocked"));

    Ok(())
}

#[test]
fn integration_kind_and_json_flags() -> TestResult {
    let ws = Workspace::new()?;
    let body = "- [ ] **T-001**: a\n  - Covers: REQ-001\n- [?] **T-002**: b\n  - Covers: REQ-001\n";
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md("SPEC-0001", body)),
    )?;

    Command::cargo_bin("speccy")?
        .args(["next", "--kind", "implement"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .stdout(contains("implement T-001"));

    Command::cargo_bin("speccy")?
        .args(["next", "--kind", "review", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .stdout(contains("\"kind\": \"review\""));
    Ok(())
}
