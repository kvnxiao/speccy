#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy verify`.
//!
//! SPEC-0018 REQ-003 contract: verify is shape-only. It does not
//! execute scenarios, does not spawn `speccy check`, and does not call
//! into the (now-deleted-by-T-004) execution layer. Coverage is mapped
//! one-to-one to the bullets under T-003 "Tests to write".

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::write_spec;
use indoc::indoc;
use predicates::str::contains;
use serde_json::Value;
use speccy_cli::verify::VerifyArgs;
use speccy_cli::verify::run;

static JSON_NULL: Value = Value::Null;

fn field<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&JSON_NULL)
}

fn at<'a>(v: &'a Value, keys: &[&str]) -> &'a Value {
    keys.iter().fold(v, |acc, k| field(acc, k))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn invoke(root: &Utf8Path, json: bool) -> TestResult<(i32, String, String)> {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(VerifyArgs { json }, root, &mut out, &mut err)?;
    Ok((code, String::from_utf8(out)?, String::from_utf8(err)?))
}

/// A well-formed spec.toml referencing one scenario. Uses the new
/// `scenario` field so REQ-003 (unreferenced scenario) cannot fire.
fn spec_toml_one_scenario() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        scenario = "Given a workspace, when verify runs, then it exits 0."
    "#}
    .to_owned()
}

/// REQ-001 trigger: requirement with an empty `checks` array.
fn spec_toml_empty_checks_array() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = []
    "#}
    .to_owned()
}

/// REQ-002 trigger: requirement references CHK-099 with no matching row.
fn spec_toml_unknown_check_reference() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-099"]
    "#}
    .to_owned()
}

/// REQ-003 trigger: a scenario row exists but no requirement references it.
fn spec_toml_unreferenced_scenario() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        scenario = "covers REQ-001"

        [[checks]]
        id = "CHK-077"
        scenario = "orphaned scenario nobody references"
    "#}
    .to_owned()
}

/// SPC-004 trigger: SPEC.md missing required `id` frontmatter field. Used
/// to inject a workspace-level / spec-level lint error.
fn spec_md_missing_id() -> String {
    indoc! {r"
        ---
        slug: x
        title: Missing id
        status: in-progress
        created: 2026-05-11
        ---

        # Missing id
    "}
    .to_owned()
}

fn assert_no_execution_keys(v: &Value) {
    if let Some(map) = v.as_object() {
        for forbidden in ["outcome", "exit_code", "duration_ms"] {
            assert!(
                !map.contains_key(forbidden),
                "execution-shaped field `{forbidden}` must not appear in verify --json",
            );
        }
        for child in map.values() {
            assert_no_execution_keys(child);
        }
    } else if let Some(arr) = v.as_array() {
        for child in arr {
            assert_no_execution_keys(child);
        }
    }
}

fn last_n_lines(s: &str, n: usize) -> Vec<&str> {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines.get(start..).map_or_else(Vec::new, <[&str]>::to_vec)
}

// ---------------------------------------------------------------------------
// Bullet 1: requirement with empty `checks` array
// ---------------------------------------------------------------------------

#[test]
fn requirement_with_empty_checks_array_exits_one_and_names_requirement() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-empty",
        // Implemented so REQ-001 stays at Error (no in-progress demotion).
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_empty_checks_array(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, true)?;
    assert_eq!(code, 1, "REQ-001 (empty checks) must gate verify");

    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");
    let req1 = errors
        .iter()
        .find(|d| field(d, "code").as_str() == Some("REQ-001"))
        .expect("REQ-001 must appear in errors");
    let message = field(req1, "message").as_str().unwrap_or("");
    assert!(
        message.contains("REQ-001"),
        "REQ-001 diagnostic must name the requirement; got: {message}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Bullet 2: requirement references CHK-099 with no matching row
// ---------------------------------------------------------------------------

#[test]
fn requirement_referencing_unknown_scenario_exits_one_and_names_both_ids() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-unknown-ref",
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_unknown_check_reference(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, true)?;
    assert_eq!(code, 1);

    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");
    let req2 = errors
        .iter()
        .find(|d| field(d, "code").as_str() == Some("REQ-002"))
        .expect("REQ-002 must appear in errors");
    let message = field(req2, "message").as_str().unwrap_or("");
    assert!(
        message.contains("REQ-001") && message.contains("CHK-099"),
        "REQ-002 diagnostic must name both REQ and CHK ids; got: {message}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Bullet 3: scenario unreferenced by any requirement
// ---------------------------------------------------------------------------

#[test]
fn unreferenced_scenario_reports_shape_error() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-orphan",
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_unreferenced_scenario(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, true)?;
    assert_eq!(code, 1, "unreferenced scenario must gate verify");

    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");
    let req3 = errors
        .iter()
        .find(|d| field(d, "code").as_str() == Some("REQ-003"))
        .expect("REQ-003 must appear in errors");
    let message = field(req3, "message").as_str().unwrap_or("");
    assert!(
        message.contains("CHK-077"),
        "REQ-003 diagnostic must name the orphaned scenario id; got: {message}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Bullet 4: clean workspace passes, no child process spawned
// ---------------------------------------------------------------------------

#[test]
fn clean_workspace_exits_zero_without_spawning_child_processes() -> TestResult {
    let ws = Workspace::new()?;
    // The scenario carries a string that, were it ever executed as a
    // shell command, would create a sentinel file. After verify runs,
    // that file must not exist — proving no child ran.
    let sentinel = ws.root.join("verify-sentinel");
    let sentinel_str = sentinel.as_str();
    let toml = format!(
        indoc! {r#"
            schema_version = 1

            [[requirements]]
            id = "REQ-001"
            checks = ["CHK-001"]

            [[checks]]
            id = "CHK-001"
            scenario = "touch {sentinel}"
        "#},
        sentinel = sentinel_str,
    );
    write_spec(
        &ws.root,
        "0001-no-spawn",
        &spec_md_template("SPEC-0001", "implemented"),
        &toml,
        None,
    )?;

    let (code, _out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0, "clean workspace must exit 0");
    assert!(
        !sentinel.as_std_path().exists(),
        "verify must not spawn child processes; sentinel file exists",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Bullet 5: text output last line shape
// ---------------------------------------------------------------------------

#[test]
fn text_output_ends_with_shape_summary_line() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-one",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_one_scenario(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0);
    let last = last_n_lines(&out, 1);
    assert_eq!(
        last,
        vec!["verified 1 specs, 1 requirements, 1 scenarios; 0 errors"],
        "text output must end with the shape summary; out:\n{out}",
    );
    Ok(())
}

#[test]
fn text_output_summary_counts_aggregate_across_specs() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-a",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_one_scenario(),
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-b",
        &spec_md_template("SPEC-0002", "in-progress"),
        &spec_toml_one_scenario(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0);
    let last = last_n_lines(&out, 1);
    assert_eq!(
        last,
        vec!["verified 2 specs, 2 requirements, 2 scenarios; 0 errors"],
    );
    Ok(())
}

#[test]
fn text_output_summary_on_empty_workspace() -> TestResult {
    let ws = Workspace::new()?;
    let (code, out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0);
    let last = last_n_lines(&out, 1);
    assert_eq!(
        last,
        vec!["verified 0 specs, 0 requirements, 0 scenarios; 0 errors"],
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Bullet 6: JSON envelope is schema_version=2 and has no execution fields
// ---------------------------------------------------------------------------

#[test]
fn json_envelope_bumps_schema_to_two_and_drops_execution_fields() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-json",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_one_scenario(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, true)?;
    assert_eq!(code, 0);

    let json: Value = serde_json::from_str(&out)?;
    assert_eq!(field(&json, "schema_version"), &Value::from(2));
    assert!(field(&json, "repo_sha").is_string());

    // Structural counts must be present and match the text summary line.
    assert_eq!(at(&json, &["summary", "shape", "specs"]), &Value::from(1));
    assert_eq!(
        at(&json, &["summary", "shape", "requirements"]),
        &Value::from(1)
    );
    assert_eq!(
        at(&json, &["summary", "shape", "scenarios"]),
        &Value::from(1)
    );
    assert_eq!(at(&json, &["summary", "shape", "errors"]), &Value::from(0));

    // Execution-shaped fields must be absent at every level the legacy
    // schema exposed them.
    assert!(
        json.get("checks").is_none(),
        "schema_version=2 must not carry a per-check execution array",
    );
    assert_no_execution_keys(&json);

    assert_eq!(field(&json, "passed"), &Value::Bool(true));
    Ok(())
}

#[test]
fn json_envelope_is_pretty_printed_with_trailing_newline() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-pretty",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_one_scenario(),
        None,
    )?;

    let (_code, out, _err) = invoke(&ws.root, true)?;
    assert!(out.ends_with('\n'), "JSON output must end with newline");
    assert!(
        out.contains("\n  \"schema_version\": 2,"),
        "JSON must be pretty-printed and declare schema_version 2; got:\n{out}",
    );
    Ok(())
}

#[test]
fn json_envelope_is_byte_identical_across_runs() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-deterministic",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_one_scenario(),
        None,
    )?;

    let (_c1, out1, _e1) = invoke(&ws.root, true)?;
    let (_c2, out2, _e2) = invoke(&ws.root, true)?;
    assert_eq!(out1, out2, "verify --json must be deterministic");
    Ok(())
}

// ---------------------------------------------------------------------------
// Bullet 7: dropped/superseded non-gating; workspace-level parse errors gate
// ---------------------------------------------------------------------------

#[test]
fn dropped_spec_with_shape_errors_is_non_gating() -> TestResult {
    let ws = Workspace::new()?;
    // A dropped spec with an empty `checks` array would normally fire
    // REQ-001 at Error. Dropped specs must not gate verify.
    write_spec(
        &ws.root,
        "0001-dropped",
        &spec_md_template("SPEC-0001", "dropped"),
        &spec_toml_empty_checks_array(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0, "dropped spec shape errors must not gate verify");
    let last = last_n_lines(&out, 1);
    // The dropped spec still counts in specs_total (one walked) but its
    // requirements/scenarios are excluded from the shape totals.
    assert_eq!(
        last,
        vec!["verified 1 specs, 0 requirements, 0 scenarios; 0 errors"],
    );
    Ok(())
}

#[test]
fn superseded_spec_with_shape_errors_is_non_gating() -> TestResult {
    let ws = Workspace::new()?;
    // Two specs: SPEC-0001 is superseded by SPEC-0002 (so SPC-006 is
    // happy), and SPEC-0001 has an empty `checks` array that would
    // otherwise gate.
    write_spec(
        &ws.root,
        "0001-old",
        &spec_md_template("SPEC-0001", "superseded"),
        &spec_toml_empty_checks_array(),
        None,
    )?;
    let supersedes_md = indoc! {r"
        ---
        id: SPEC-0002
        slug: x
        title: Example SPEC-0002
        status: in-progress
        created: 2026-05-11
        supersedes: [SPEC-0001]
        ---

        # SPEC-0002

        ### REQ-001: First
        Body.
    "};
    write_spec(
        &ws.root,
        "0002-new",
        supersedes_md,
        &spec_toml_one_scenario(),
        None,
    )?;

    let (code, _out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0, "superseded spec shape errors must not gate verify");
    Ok(())
}

#[test]
fn workspace_level_parse_errors_still_gate_verify() -> TestResult {
    let ws = Workspace::new()?;
    // SPEC.md missing `id` -> SPC-004 (Error) -> gating.
    write_spec(
        &ws.root,
        "0001-bad",
        &spec_md_missing_id(),
        &spec_toml_one_scenario(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, true)?;
    assert_eq!(code, 1, "SPC-004 parse error must gate verify");

    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");
    assert!(
        errors
            .iter()
            .any(|d| field(d, "code").as_str() == Some("SPC-004")),
        "SPC-004 must appear in lint.errors; got: {out}",
    );
    Ok(())
}

#[test]
fn in_progress_spec_shape_errors_are_demoted_not_gating() -> TestResult {
    let ws = Workspace::new()?;
    // Empty `checks` array on an in-progress spec must be demoted to
    // info (not gating).
    write_spec(
        &ws.root,
        "0001-drafting",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_empty_checks_array(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, true)?;
    assert_eq!(
        code, 0,
        "REQ-001 on an in-progress spec must not gate; out:\n{out}",
    );

    let json: Value = serde_json::from_str(&out)?;
    let info = at(&json, &["lint", "info"])
        .as_array()
        .expect("lint.info array");
    assert!(
        info.iter()
            .any(|d| field(d, "code").as_str() == Some("REQ-001")),
        "REQ-001 must be demoted to info on in-progress specs; got: {out}",
    );
    Ok(())
}

#[test]
fn implemented_spec_shape_errors_still_gate() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-shipped",
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_empty_checks_array(),
        None,
    )?;

    let (code, _out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 1, "REQ-001 on an implemented spec must gate verify");
    Ok(())
}

// ---------------------------------------------------------------------------
// Binary dispatcher: outside workspace + flag handling (unchanged contract)
// ---------------------------------------------------------------------------

#[test]
fn verify_outside_workspace_fails() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("verify").current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn binary_rejects_unknown_flag() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("verify")
        .arg("--bogus")
        .current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(2);
    Ok(())
}

#[test]
fn binary_propagates_exit_zero_on_pass() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-pass",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_one_scenario(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("verify").current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .code(0)
        .stdout(contains("verified 1 specs, 1 requirements, 1 scenarios"));
    Ok(())
}

#[test]
fn binary_propagates_exit_one_on_shape_failure() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-fail",
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_empty_checks_array(),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("verify").current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(1);
    Ok(())
}
