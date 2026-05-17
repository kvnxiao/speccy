#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for the `speccy status` positional `SPEC-NNNN` selector,
//! `--all` flag, hidden-count footer, and per-spec JSON. Covers
//! SPEC-0024 REQ-004 / CHK-004.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::write_spec;
use speccy_cli::status::StatusArgs;
use speccy_cli::status::run;

fn render_text(root: &camino::Utf8Path, args: &StatusArgs) -> TestResult<String> {
    let mut buf: Vec<u8> = Vec::new();
    run(args, root, &mut buf)?;
    Ok(String::from_utf8(buf)?)
}

fn render_json(root: &camino::Utf8Path, args: &StatusArgs) -> TestResult<serde_json::Value> {
    let mut buf: Vec<u8> = Vec::new();
    run(args, root, &mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

fn args_default() -> StatusArgs {
    StatusArgs {
        selector: None,
        all: false,
        json: false,
    }
}

fn args_selector(id: &str) -> StatusArgs {
    StatusArgs {
        selector: Some(id.to_owned()),
        all: false,
        json: false,
    }
}

fn args_all() -> StatusArgs {
    StatusArgs {
        selector: None,
        all: true,
        json: false,
    }
}

#[test]
fn parsing_rejects_both_positional_and_all_flag() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("status")
        .arg("--all")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    // clap exits with code 2 on argument conflicts.
    let output = cmd.assert().failure().code(2).get_output().clone();
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("--all") && stderr.contains("SELECTOR"),
        "clap conflict error must name both flags, got:\n{stderr}",
    );
    Ok(())
}

#[test]
fn unknown_spec_id_errors_without_writing_stdout() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        "",
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("status")
        .arg("SPEC-9999")
        .current_dir(ws.root.as_std_path());
    let output = cmd.assert().failure().get_output().clone();
    assert!(
        output.stdout.is_empty(),
        "expected no stdout output for unknown-spec error, got {:?}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("SPEC-9999"),
        "stderr must name the missing ID, got:\n{stderr}",
    );
    Ok(())
}

#[test]
fn positional_renders_one_spec_text_even_if_it_would_be_filtered() -> TestResult {
    let ws = Workspace::new()?;
    // SPEC-0001 in-progress (would always be shown).
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        "",
        None,
    )?;
    // SPEC-0002 clean implemented (would normally be filtered).
    write_spec(
        &ws.root,
        "0002-done",
        &spec_md_template("SPEC-0002", "implemented"),
        "",
        None,
    )?;

    let text = render_text(&ws.root, &args_selector("SPEC-0002"))?;
    assert!(
        text.contains("SPEC-0002"),
        "selected spec block must render, got:\n{text}",
    );
    assert!(
        !text.contains("SPEC-0001"),
        "non-selected spec must NOT render, got:\n{text}",
    );
    assert!(
        !text.contains("specs hidden"),
        "selector path must not emit the hidden-count footer, got:\n{text}",
    );
    Ok(())
}

#[test]
fn positional_renders_one_spec_json() -> TestResult {
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

    let parsed = render_json(
        &ws.root,
        &StatusArgs {
            selector: Some("SPEC-0002".to_owned()),
            all: false,
            json: true,
        },
    )?;
    assert_eq!(parsed.get("schema_version"), Some(&serde_json::json!(1)));
    let specs = parsed
        .get("specs")
        .and_then(|v| v.as_array())
        .expect("specs must be an array");
    assert_eq!(
        specs.len(),
        1,
        "selector-mode JSON must return exactly one spec, got: {specs:?}",
    );
    let only = specs.first().expect("specs[0] must exist after len==1");
    assert_eq!(only.get("id").and_then(|v| v.as_str()), Some("SPEC-0002"));
    Ok(())
}

#[test]
fn all_flag_renders_every_spec_text() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        "",
        None,
    )?;
    // Clean implemented would normally be filtered.
    write_spec(
        &ws.root,
        "0002-done",
        &spec_md_template("SPEC-0002", "implemented"),
        "",
        None,
    )?;

    let text = render_text(&ws.root, &args_all())?;
    assert!(text.contains("SPEC-0001"), "got:\n{text}");
    assert!(
        text.contains("SPEC-0002"),
        "--all must include the filtered spec, got:\n{text}",
    );
    assert!(
        !text.contains("specs hidden"),
        "--all path must not emit the hidden-count footer, got:\n{text}",
    );
    Ok(())
}

#[test]
fn all_flag_on_empty_workspace_prints_empty_message() -> TestResult {
    let ws = Workspace::new()?;
    let text = render_text(&ws.root, &args_all())?;
    assert!(
        text.contains("No specs in workspace."),
        "empty workspace must print the empty message, got:\n{text}",
    );
    Ok(())
}

#[test]
fn all_flag_json_matches_default_json_shape() -> TestResult {
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

    let default_json = render_json(
        &ws.root,
        &StatusArgs {
            selector: None,
            all: false,
            json: true,
        },
    )?;
    let all_json = render_json(
        &ws.root,
        &StatusArgs {
            selector: None,
            all: true,
            json: true,
        },
    )?;
    // --all --json and --json (no flags) must be byte-equivalent in
    // shape: both contain every spec, no filter applied.
    assert_eq!(default_json, all_json);
    Ok(())
}

#[test]
fn footer_appended_when_default_filter_hides_specs() -> TestResult {
    let ws = Workspace::new()?;
    // Visible: in-progress.
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        "",
        None,
    )?;
    // Hidden: clean implemented (no TASKS.md → not stale).
    write_spec(
        &ws.root,
        "0002-done",
        &spec_md_template("SPEC-0002", "implemented"),
        "",
        None,
    )?;

    let text = render_text(&ws.root, &args_default())?;
    assert!(text.contains("SPEC-0001"), "got:\n{text}");
    assert!(
        !text.contains("SPEC-0002"),
        "filtered spec must not render, got:\n{text}",
    );
    assert!(
        text.contains("1 specs hidden; pass --all to see them"),
        "footer must be appended, got:\n{text}",
    );
    Ok(())
}

#[test]
fn footer_suppressed_when_nothing_is_hidden() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        "",
        None,
    )?;

    let text = render_text(&ws.root, &args_default())?;
    assert!(text.contains("SPEC-0001"), "got:\n{text}");
    assert!(
        !text.contains("specs hidden"),
        "footer must be suppressed when nothing was filtered, got:\n{text}",
    );
    Ok(())
}

#[test]
fn footer_appended_when_attention_list_empty_but_filter_hides_specs() -> TestResult {
    let ws = Workspace::new()?;
    // Two clean implemented specs — both get filtered, attention list
    // is empty, footer still appears.
    write_spec(
        &ws.root,
        "0001-done",
        &spec_md_template("SPEC-0001", "implemented"),
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

    let text = render_text(&ws.root, &args_default())?;
    assert!(
        text.contains("No in-progress specs need attention."),
        "got:\n{text}",
    );
    assert!(
        text.contains("2 specs hidden; pass --all to see them"),
        "footer must follow the no-attention message, got:\n{text}",
    );
    Ok(())
}

#[test]
fn help_describes_selector_and_all_flag() -> TestResult {
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("status").arg("--help");
    let output = cmd.assert().success().get_output().clone();
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("SELECTOR") || stdout.contains("SPEC-NNNN"),
        "help must describe the positional, got:\n{stdout}",
    );
    assert!(
        stdout.contains("--all"),
        "help must describe --all, got:\n{stdout}",
    );
    Ok(())
}
