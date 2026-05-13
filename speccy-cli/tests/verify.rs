#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy verify`. Covers SPEC-0012 CHK-001..CHK-007.

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

/// `Value::Null` as a static so [`field`] can hand out a borrow when a key
/// is missing without allocating per call.
static JSON_NULL: Value = Value::Null;

/// Borrow a key from a JSON object, falling back to [`JSON_NULL`] when
/// the key is missing. Keeps test code free of `value[idx]` indexing
/// (forbidden by `clippy::indexing_slicing`) without sacrificing
/// readability.
fn field<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&JSON_NULL)
}

/// Convenience: walk a chain of keys (`field(field(json, "a"), "b")`).
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

fn spec_toml_passing(check_command: &str) -> String {
    format!(
        indoc! {r#"
            schema_version = 1

            [[requirements]]
            id = "REQ-001"
            checks = ["CHK-001"]

            [[checks]]
            id = "CHK-001"
            kind = "test"
            command = {cmd:?}
            proves = "covers REQ-001"
        "#},
        cmd = check_command,
    )
}

fn spec_toml_three_outcomes() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001", "CHK-002", "CHK-003"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 0"
        proves = "first passes"

        [[checks]]
        id = "CHK-002"
        kind = "test"
        command = "exit 1"
        proves = "second fails"

        [[checks]]
        id = "CHK-003"
        kind = "test"
        command = "exit 0"
        proves = "third passes again"
    "#}
    .to_owned()
}

fn spec_toml_with_manual() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001", "CHK-002", "CHK-003"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "exit 0"
        proves = "executable pass"

        [[checks]]
        id = "CHK-002"
        kind = "manual"
        prompt = "Click signup; confirm no errors appear."
        proves = "manual claim"

        [[checks]]
        id = "CHK-003"
        kind = "test"
        command = "exit 0"
        proves = "executable pass 2"
    "#}
    .to_owned()
}

/// SPEC.md missing the required `id` frontmatter field. SPC-004 fires at
/// `Error` level, so this fixture is a clean way to inject a lint error
/// without inventing a malformed spec.toml.
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

fn last_n_lines(s: &str, n: usize) -> Vec<&str> {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines.get(start..).map_or_else(Vec::new, <[&str]>::to_vec)
}

// ---------------------------------------------------------------------------
// CHK-001: lint integration
// ---------------------------------------------------------------------------

#[test]
fn lint_integration_partitions_errors_warnings_info() -> TestResult {
    let ws = Workspace::new()?;
    // Spec A: lint error (missing frontmatter id).
    write_spec(
        &ws.root,
        "0001-bad",
        &spec_md_missing_id(),
        &spec_toml_passing("exit 0"),
        None,
    )?;
    // Spec B: clean (all passing).
    write_spec(
        &ws.root,
        "0002-good",
        &spec_md_template("SPEC-0002", "in-progress"),
        &spec_toml_passing("exit 0"),
        None,
    )?;

    let (code, out, err) = invoke(&ws.root, true)?;
    assert_eq!(code, 1, "lint error must drive exit 1; stderr:\n{err}");

    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");
    // Warnings/info buckets exist as arrays (may be empty).
    assert!(at(&json, &["lint", "warnings"]).is_array());
    assert!(at(&json, &["lint", "info"]).is_array());

    assert!(
        !errors.is_empty(),
        "expected at least one lint error; got json:\n{out}",
    );
    // Each diagnostic is a structured object (not a string).
    let first_err = errors.first().expect("at least one error");
    assert!(
        first_err.is_object(),
        "diagnostic must be structured object"
    );
    assert!(field(first_err, "code").is_string());
    assert!(field(first_err, "message").is_string());
    Ok(())
}

#[test]
fn lint_integration_runs_on_empty_workspace() -> TestResult {
    let ws = Workspace::new()?;
    let (code, out, _err) = invoke(&ws.root, true)?;
    assert_eq!(code, 0);

    let json: Value = serde_json::from_str(&out)?;
    assert!(
        at(&json, &["lint", "errors"])
            .as_array()
            .expect("errors")
            .is_empty(),
    );
    assert!(
        field(&json, "checks")
            .as_array()
            .expect("checks")
            .is_empty()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-002: check execution streams to stderr
// ---------------------------------------------------------------------------

#[test]
fn check_execution_streams_to_stderr() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-stream",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("echo hello-from-check"),
        None,
    )?;

    let (code, out, err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0, "all green => exit 0; stderr:\n{err}");

    // Headers, child output, footer all on stderr.
    assert!(err.contains("==> CHK-001 (SPEC-0001):"));
    assert!(
        err.contains("hello-from-check"),
        "child output must stream to stderr; got:\n{err}",
    );
    assert!(err.contains("<-- CHK-001 PASS"));

    // Headers and child output must NOT contaminate stdout (which carries
    // the three-line summary only).
    assert!(
        !out.contains("==> CHK-001"),
        "stdout should not carry per-check framing; got:\n{out}"
    );
    assert!(!out.contains("hello-from-check"));
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-003: run-all (not fail-fast); manual checks don't affect exit code
// ---------------------------------------------------------------------------

#[test]
fn run_all_not_fail_fast() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-mixed",
        // Implemented: the failure must gate so exit code is 1.
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_three_outcomes(),
        None,
    )?;

    let (code, _out, err) = invoke(&ws.root, false)?;
    assert_eq!(code, 1, "one failing check => exit 1");

    // All three executable checks must have run.
    assert!(err.contains("<-- CHK-001 PASS"));
    assert!(err.contains("<-- CHK-002 FAIL"));
    assert!(err.contains("<-- CHK-003 PASS"));
    Ok(())
}

#[test]
fn spec_scoped_chk_ids_duplicate_across_specs_both_run() -> TestResult {
    let ws = Workspace::new()?;
    // Two specs both define CHK-001 (legitimate scoping per SPEC-0010
    // DEC-003). Both must execute.
    write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("exit 0"),
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-beta",
        &spec_md_template("SPEC-0002", "in-progress"),
        &spec_toml_passing("exit 0"),
        None,
    )?;

    let (code, _out, err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0);
    assert!(err.contains("==> CHK-001 (SPEC-0001)"));
    assert!(err.contains("==> CHK-001 (SPEC-0002)"));
    Ok(())
}

#[test]
fn manual_check_does_not_affect_exit_code() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-manual",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_with_manual(),
        None,
    )?;

    let (code, out, err) = invoke(&ws.root, false)?;
    // Two passing + one manual + zero failing => exit 0.
    assert_eq!(code, 0);
    assert!(err.contains("==> CHK-002 (SPEC-0001, manual):"));
    assert!(err.contains("Click signup"));
    assert!(err.contains("<-- CHK-002 MANUAL (verify and proceed)"));

    // Summary on stdout reflects the manual count.
    assert!(out.contains("Checks: 2 passed, 0 failed, 0 in-flight, 1 manual"));
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-004: binary exit code
// ---------------------------------------------------------------------------

#[test]
fn binary_exit_code_clean_workspace() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-pass",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("exit 0"),
        None,
    )?;

    let (code, _out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0);
    Ok(())
}

#[test]
fn binary_exit_code_lint_only_failure() -> TestResult {
    let ws = Workspace::new()?;
    // Lint error (SPC-004 missing frontmatter id) + all passing checks.
    write_spec(
        &ws.root,
        "0001-bad",
        &spec_md_missing_id(),
        &spec_toml_passing("exit 0"),
        None,
    )?;

    let (code, _out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 1, "lint error alone fails the gate");
    Ok(())
}

#[test]
fn in_progress_spec_lint_errors_do_not_gate_verify() -> TestResult {
    let ws = Workspace::new()?;
    // SPEC.md parses cleanly with status: in-progress, but TASKS.md
    // references a REQ that doesn't exist in SPEC.md/spec.toml. That's a
    // TSK-001 Level::Error — but on an in-progress spec, it must be
    // demoted to info and not gate verify.
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        - [ ] **T-001**: covers a non-existent REQ
          - Covers: REQ-999
    "};
    write_spec(
        &ws.root,
        "0001-drafted",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("exit 0"),
        Some(tasks_md),
    )?;

    let (code, out, err) = invoke(&ws.root, true)?;
    assert_eq!(
        code, 0,
        "TSK-001 on an in-progress spec must not gate; err:\n{err}",
    );

    let json: Value = serde_json::from_str(&out)?;
    assert_eq!(
        at(&json, &["summary", "lint", "errors"]),
        &Value::from(0),
        "demoted TSK-001 must not count as an error; json:\n{out}",
    );
    assert_eq!(field(&json, "passed"), &Value::Bool(true));

    let info = at(&json, &["lint", "info"])
        .as_array()
        .expect("lint.info array");
    let demoted = info
        .iter()
        .find(|d| field(d, "code").as_str() == Some("TSK-001"))
        .expect("demoted TSK-001 must appear in info bucket");
    assert_eq!(
        field(demoted, "level"),
        &Value::from("info"),
        "demoted diagnostic must carry level=info",
    );
    Ok(())
}

#[test]
fn implemented_spec_lint_errors_still_gate_verify() -> TestResult {
    let ws = Workspace::new()?;
    // Same TSK-001 shape as the in-progress test above, but the parent
    // spec is implemented; the error must remain gating.
    let tasks_md = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        - [x] **T-001**: covers a non-existent REQ
          - Covers: REQ-999
    "};
    write_spec(
        &ws.root,
        "0001-shipped",
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_passing("exit 0"),
        Some(tasks_md),
    )?;

    let (code, _out, _err) = invoke(&ws.root, false)?;
    assert_eq!(
        code, 1,
        "TSK-001 on an implemented spec must still gate verify",
    );
    Ok(())
}

#[test]
fn binary_exit_code_check_only_failure() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-fail",
        // Implemented status: failing checks gate the exit code.
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_passing("exit 1"),
        None,
    )?;

    let (code, _out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 1, "failing check alone fails the gate");
    Ok(())
}

#[test]
fn binary_exit_code_empty_workspace_passes() -> TestResult {
    let ws = Workspace::new()?;
    let (code, _out, _err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0);
    Ok(())
}

#[test]
fn binary_exit_code_is_deterministic_across_runs() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-pass",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("exit 0"),
        None,
    )?;

    let (code1, _, _) = invoke(&ws.root, false)?;
    let (code2, _, _) = invoke(&ws.root, false)?;
    assert_eq!(code1, code2);
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-005: text mode summary output (last three lines)
// ---------------------------------------------------------------------------

#[test]
fn text_summary_output_last_three_lines() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-pass",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("echo greeting"),
        None,
    )?;

    let (_code, out, err) = invoke(&ws.root, false)?;
    let last = last_n_lines(&out, 3);
    assert_eq!(
        last,
        vec![
            "Lint: 0 errors, 0 warnings, 0 info",
            "Checks: 1 passed, 0 failed, 0 in-flight, 0 manual",
            "verify: PASS",
        ],
        "expected the three summary lines; out:\n{out}\nerr:\n{err}",
    );
    Ok(())
}

#[test]
fn text_summary_output_failure_shows_fail() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-fail",
        // Implemented: failure gates -> FAIL verdict.
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_passing("exit 1"),
        None,
    )?;

    let (_code, out, _err) = invoke(&ws.root, false)?;
    let last = last_n_lines(&out, 3);
    assert_eq!(
        last,
        vec![
            "Lint: 0 errors, 0 warnings, 0 info",
            "Checks: 0 passed, 1 failed, 0 in-flight, 0 manual",
            "verify: FAIL",
        ],
    );
    Ok(())
}

#[test]
fn text_summary_output_empty_workspace_passes() -> TestResult {
    let ws = Workspace::new()?;
    let (_code, out, _err) = invoke(&ws.root, false)?;
    let last = last_n_lines(&out, 3);
    assert_eq!(
        last,
        vec![
            "Lint: 0 errors, 0 warnings, 0 info",
            "Checks: 0 passed, 0 failed, 0 in-flight, 0 manual",
            "verify: PASS",
        ],
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-006: JSON contract
// ---------------------------------------------------------------------------

#[test]
fn json_contract_shape() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-mix",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_with_manual(),
        None,
    )?;

    let (code, out, _err) = invoke(&ws.root, true)?;
    assert_eq!(code, 0);

    let json: Value = serde_json::from_str(&out)?;
    assert_eq!(field(&json, "schema_version"), &Value::from(1));
    assert!(field(&json, "repo_sha").is_string());

    // Top-level lint block has all three arrays.
    assert!(at(&json, &["lint", "errors"]).is_array());
    assert!(at(&json, &["lint", "warnings"]).is_array());
    assert!(at(&json, &["lint", "info"]).is_array());

    // checks array carries structured per-check objects.
    let checks = field(&json, "checks").as_array().expect("checks array");
    assert_eq!(checks.len(), 3);
    let first = checks.first().expect("first check");
    assert_eq!(field(first, "spec_id"), &Value::from("SPEC-0001"));
    assert_eq!(field(first, "check_id"), &Value::from("CHK-001"));
    assert_eq!(field(first, "kind"), &Value::from("test"));
    assert_eq!(field(first, "outcome"), &Value::from("Pass"));

    // Manual entry: no exit_code field.
    let manual = checks.get(1).expect("manual check entry");
    assert_eq!(field(manual, "check_id"), &Value::from("CHK-002"));
    assert_eq!(field(manual, "outcome"), &Value::from("Manual"));
    let exit_code = manual.get("exit_code");
    assert!(
        exit_code.is_none_or(Value::is_null),
        "manual checks must not emit an exit_code; got: {manual}",
    );

    // Summary counts.
    assert_eq!(at(&json, &["summary", "lint", "errors"]), &Value::from(0));
    assert_eq!(at(&json, &["summary", "checks", "passed"]), &Value::from(2));
    assert_eq!(at(&json, &["summary", "checks", "failed"]), &Value::from(0));
    assert_eq!(
        at(&json, &["summary", "checks", "in_flight"]),
        &Value::from(0),
        "JSON summary must expose the in-flight bucket",
    );
    assert_eq!(at(&json, &["summary", "checks", "manual"]), &Value::from(1));

    // Each check entry now carries spec_status for harness filtering.
    assert_eq!(
        field(first, "spec_status"),
        &Value::from("in-progress"),
        "per-check JSON entries must include spec_status",
    );

    // passed bool mirrors exit code.
    assert_eq!(field(&json, "passed"), &Value::Bool(true));
    Ok(())
}

#[test]
fn json_contract_is_pretty_printed_with_trailing_newline() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-pretty",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("exit 0"),
        None,
    )?;

    let (_code, out, _err) = invoke(&ws.root, true)?;
    assert!(
        out.ends_with('\n'),
        "JSON output must end with a trailing newline",
    );
    assert!(
        out.contains("\n  \"schema_version\": 1,") || out.contains("\n  \"schema_version\":1,"),
        "JSON must be pretty-printed; got:\n{out}",
    );
    Ok(())
}

#[test]
fn json_contract_is_byte_identical_across_runs() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-deterministic",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("exit 0"),
        None,
    )?;

    let (_c1, out1, _e1) = invoke(&ws.root, true)?;
    let (_c2, out2, _e2) = invoke(&ws.root, true)?;
    assert_eq!(
        out1, out2,
        "verify --json must be byte-identical across runs with the same state",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-007: passed field mirrors exit code
// ---------------------------------------------------------------------------

#[test]
fn json_passed_field_mirrors_exit_clean() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-pass",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("exit 0"),
        None,
    )?;
    let (code, out, _err) = invoke(&ws.root, true)?;
    let json: Value = serde_json::from_str(&out)?;
    assert_eq!(code, 0);
    assert_eq!(field(&json, "passed"), &Value::Bool(true));
    Ok(())
}

#[test]
fn json_passed_field_mirrors_exit_lint_only() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-lint-only",
        &spec_md_missing_id(),
        &spec_toml_passing("exit 0"),
        None,
    )?;
    let (code, out, _err) = invoke(&ws.root, true)?;
    let json: Value = serde_json::from_str(&out)?;
    assert_eq!(code, 1);
    assert_eq!(field(&json, "passed"), &Value::Bool(false));
    Ok(())
}

#[test]
fn json_passed_field_mirrors_exit_check_only() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-check-only",
        // Implemented: failing check must gate (exit 1, passed=false).
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_passing("exit 1"),
        None,
    )?;
    let (code, out, _err) = invoke(&ws.root, true)?;
    let json: Value = serde_json::from_str(&out)?;
    assert_eq!(code, 1);
    assert_eq!(field(&json, "passed"), &Value::Bool(false));
    Ok(())
}

#[test]
fn json_passed_field_mirrors_exit_both_failing() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-both",
        &spec_md_missing_id(),
        &spec_toml_passing("exit 1"),
        None,
    )?;
    let (code, out, _err) = invoke(&ws.root, true)?;
    let json: Value = serde_json::from_str(&out)?;
    assert_eq!(code, 1);
    assert_eq!(field(&json, "passed"), &Value::Bool(false));
    Ok(())
}

// ---------------------------------------------------------------------------
// Binary dispatcher: outside workspace + flag handling
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
fn binary_propagates_exit_one_on_failure() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-fail",
        // Implemented: failing check must gate the exit code.
        &spec_md_template("SPEC-0001", "implemented"),
        &spec_toml_passing("exit 1"),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("verify").current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(1);
    Ok(())
}

#[test]
fn in_progress_spec_failures_do_not_gate_verify() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-flight",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("exit 1"),
        None,
    )?;

    let (code, out, err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0, "in-progress failures must not gate; err:\n{err}");
    assert!(
        err.contains("IN-FLIGHT"),
        "footer must use IN-FLIGHT wording: {err}",
    );
    let last = last_n_lines(&out, 3);
    assert_eq!(
        last,
        vec![
            "Lint: 0 errors, 0 warnings, 0 info",
            "Checks: 0 passed, 0 failed, 1 in-flight, 0 manual",
            "verify: PASS",
        ],
    );
    Ok(())
}

#[test]
fn dropped_spec_checks_are_skipped_in_verify() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-dropped",
        &spec_md_template("SPEC-0001", "dropped"),
        &spec_toml_passing("exit 1"),
        None,
    )?;

    let (code, out, err) = invoke(&ws.root, false)?;
    assert_eq!(code, 0, "dropped spec checks must not run");
    assert!(
        !err.contains("CHK-001"),
        "no per-check framing for dropped specs: {err}",
    );
    let last = last_n_lines(&out, 3);
    assert_eq!(
        last,
        vec![
            "Lint: 0 errors, 0 warnings, 0 info",
            "Checks: 0 passed, 0 failed, 0 in-flight, 0 manual",
            "verify: PASS",
        ],
    );
    Ok(())
}

#[test]
fn binary_propagates_exit_zero_on_pass() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-pass",
        &spec_md_template("SPEC-0001", "in-progress"),
        &spec_toml_passing("exit 0"),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("verify").current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .code(0)
        .stdout(contains("verify: PASS"));
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
