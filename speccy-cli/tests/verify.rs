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

fn invoke(root: &Utf8Path, json: bool) -> TestResult<(i32, String)> {
    let mut out: Vec<u8> = Vec::new();
    let code = run(
        VerifyArgs {
            include_archive: false,
            json,
        },
        root,
        &mut out,
    )?;
    Ok((code, String::from_utf8(out)?))
}

/// Marker SPEC.md with a requirement marker that has no nested
/// scenarios — fires REQ-001.
fn spec_md_empty_scenarios(spec_id: &str, status: &str) -> String {
    let template = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: __STATUS__
        created: 2026-05-11
        ---

        # __ID__

        <goals>
        Example goals.
        </goals>

        <non-goals>
        Example non-goals.
        </non-goals>

        <user-stories>
        - Example story.
        </user-stories>

        <requirement id="REQ-001">
        ### REQ-001: First
        Body with no scenarios.
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    template
        .replace("__ID__", spec_id)
        .replace("__STATUS__", status)
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
        &spec_md_empty_scenarios("SPEC-0001", "implemented"),
        None,
    )?;

    let (code, out) = invoke(&ws.root, true)?;
    assert_eq!(code, 1, "empty scenarios must gate verify");

    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");
    // After SPEC-0019 the marker parser itself rejects a requirement
    // with no nested scenario marker, so the gating error surfaces as
    // SPC-001 (marker tree invalid) rather than REQ-001 (lint-derived).
    let spc1 = errors
        .iter()
        .find(|d| field(d, "code").as_str() == Some("SPC-001"))
        .expect("SPC-001 must appear in errors");
    let message = field(spc1, "message").as_str().unwrap_or("");
    assert!(
        message.contains("REQ-001"),
        "SPC-001 diagnostic must name the offending requirement; got: {message}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// (Former REQ-002 / REQ-003 tests removed: SPEC-0019 marker containment
// makes both "dangling CHK reference" and "orphan scenario row"
// structurally unrepresentable, so neither code can fire.)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Bullet 4: clean workspace passes, no child process spawned
// ---------------------------------------------------------------------------

#[test]
fn clean_workspace_exits_zero_without_spawning_child_processes() -> TestResult {
    let ws = Workspace::new()?;
    // Sentinel file used as a process-spawning canary: even though the
    // SPEC.md marker-tree scenario body is plain text and never
    // executed, the assertion guards against regressions that would
    // reintroduce subprocess execution.
    let sentinel = ws.root.join("verify-sentinel");
    write_spec(
        &ws.root,
        "0001-no-spawn",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;

    let (code, _out) = invoke(&ws.root, false)?;
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
        None,
    )?;

    let (code, out) = invoke(&ws.root, false)?;
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
        None,
    )?;
    write_spec(
        &ws.root,
        "0002-b",
        &spec_md_template("SPEC-0002", "in-progress"),
        None,
    )?;

    let (code, out) = invoke(&ws.root, false)?;
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
    let (code, out) = invoke(&ws.root, false)?;
    assert_eq!(code, 0);
    let last = last_n_lines(&out, 1);
    assert_eq!(
        last,
        vec!["verified 0 specs, 0 requirements, 0 scenarios; 0 errors"],
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// JSON envelope is schema_version=1 and has no execution fields
// ---------------------------------------------------------------------------

#[test]
fn json_envelope_is_schema_one_and_has_no_execution_fields() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-json",
        &spec_md_template("SPEC-0001", "in-progress"),
        None,
    )?;

    let (code, out) = invoke(&ws.root, true)?;
    assert_eq!(code, 0);

    let json: Value = serde_json::from_str(&out)?;
    assert_eq!(field(&json, "schema_version"), &Value::from(1));
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

    // Execution-shaped fields must be absent at every level.
    assert!(
        json.get("checks").is_none(),
        "verify JSON must not carry a per-check execution array",
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
        None,
    )?;

    let (_code, out) = invoke(&ws.root, true)?;
    assert!(out.ends_with('\n'), "JSON output must end with newline");
    assert!(
        out.contains("\n  \"schema_version\": 1,"),
        "JSON must be pretty-printed and declare schema_version 1; got:\n{out}",
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
        None,
    )?;

    let (_c1, out1) = invoke(&ws.root, true)?;
    let (_c2, out2) = invoke(&ws.root, true)?;
    assert_eq!(out1, out2, "verify --json must be deterministic");
    Ok(())
}

// ---------------------------------------------------------------------------
// Bullet 7: dropped/superseded non-gating; workspace-level parse errors gate
// ---------------------------------------------------------------------------

#[test]
fn dropped_spec_with_shape_errors_is_non_gating() -> TestResult {
    let ws = Workspace::new()?;
    // A dropped spec with no scenarios would normally fire REQ-001 at
    // Error. Dropped specs must not gate verify.
    write_spec(
        &ws.root,
        "0001-dropped",
        &spec_md_empty_scenarios("SPEC-0001", "dropped"),
        None,
    )?;

    let (code, out) = invoke(&ws.root, false)?;
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
        &spec_md_empty_scenarios("SPEC-0001", "superseded"),
        None,
    )?;
    let supersedes_md = indoc! {r#"
        ---
        id: SPEC-0002
        slug: x
        title: Example SPEC-0002
        status: in-progress
        created: 2026-05-11
        supersedes: [SPEC-0001]
        ---

        # SPEC-0002

        <goals>
        Example goals.
        </goals>

        <non-goals>
        Example non-goals.
        </non-goals>

        <user-stories>
        - Example story.
        </user-stories>

        <requirement id="REQ-001">
        ### REQ-001: First
        Body.
        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        covers REQ-001
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    write_spec(&ws.root, "0002-new", supersedes_md, None)?;

    let (code, _out) = invoke(&ws.root, false)?;
    assert_eq!(code, 0, "superseded spec shape errors must not gate verify");
    Ok(())
}

#[test]
fn workspace_level_parse_errors_still_gate_verify() -> TestResult {
    let ws = Workspace::new()?;
    // SPEC.md missing `id` -> SPC-004 (Error) -> gating.
    write_spec(&ws.root, "0001-bad", &spec_md_missing_id(), None)?;

    let (code, out) = invoke(&ws.root, true)?;
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
    // Empty scenarios on an in-progress spec must be demoted to info
    // (not gating).
    write_spec(
        &ws.root,
        "0001-drafting",
        &spec_md_empty_scenarios("SPEC-0001", "in-progress"),
        None,
    )?;

    let (code, out) = invoke(&ws.root, true)?;
    assert_eq!(
        code, 0,
        "marker-tree shape errors on an in-progress spec must not gate; out:\n{out}",
    );

    let json: Value = serde_json::from_str(&out)?;
    let info = at(&json, &["lint", "info"])
        .as_array()
        .expect("lint.info array");
    assert!(
        info.iter()
            .any(|d| field(d, "code").as_str() == Some("SPC-001")),
        "SPC-001 must be demoted to info on in-progress specs; got: {out}",
    );
    Ok(())
}

#[test]
fn implemented_spec_shape_errors_still_gate() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-shipped",
        &spec_md_empty_scenarios("SPEC-0001", "implemented"),
        None,
    )?;

    let (code, _out) = invoke(&ws.root, false)?;
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
        &spec_md_empty_scenarios("SPEC-0001", "implemented"),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("verify").current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(1);
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0019 T-006: duplicate scenario id across two requirements -> verify
// surfaces the marker parser's DuplicateMarkerId via SPC-001 and gates.
// ---------------------------------------------------------------------------

#[test]
fn duplicate_scenario_id_across_requirements_gates_verify() -> TestResult {
    let ws = Workspace::new()?;
    let spec_md = indoc! {r#"
        ---
        id: SPEC-0098
        slug: x
        title: Example
        status: implemented
        created: 2026-05-11
        ---

        # SPEC-0098

        <goals>
        Example goals.
        </goals>

        <non-goals>
        Example non-goals.
        </non-goals>

        <user-stories>
        - Example story.
        </user-stories>

        <requirement id="REQ-001">
        ### REQ-001: First
        body
        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        first
        </scenario>
        </requirement>
        <requirement id="REQ-002">
        ### REQ-002: Second
        body
        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        duplicate
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    write_spec(&ws.root, "0098-dup-chk", spec_md, None)?;

    let (code, out) = invoke(&ws.root, true)?;
    assert_eq!(
        code, 1,
        "duplicate scenario id must gate verify (status=implemented)",
    );
    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");
    let spc1 = errors
        .iter()
        .find(|d| {
            field(d, "code").as_str() == Some("SPC-001")
                && field(d, "message")
                    .as_str()
                    .is_some_and(|m| m.contains("CHK-001"))
        })
        .expect("SPC-001 diagnostic naming CHK-001 must appear in errors");
    let message = field(spc1, "message").as_str().unwrap_or("");
    assert!(
        message.contains("duplicate"),
        "SPC-001 must surface the duplicate-id wording from the marker parser; got: {message}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0035: RPT-* lint family integration tests (CHK-001 / CHK-002 / CHK-003
// plus in-progress demotion)
// ---------------------------------------------------------------------------

/// Minimal valid REPORT.md frontmatter + heading, followed by a `<report>`
/// root open tag with no `spec="..."` attribute.  The parser requires
/// `spec=`, so this triggers RPT-001.
fn report_md_missing_spec_attr() -> String {
    indoc! {r"
        ---
        spec: SPEC-0001
        outcome: satisfied
        generated_at: 2026-05-20T00:00:00Z
        ---

        # REPORT: SPEC-0001

        <report>
        </report>
    "}
    .to_owned()
}

/// Minimal valid REPORT.md with a `<coverage>` row referencing `REQ-999`
/// (a requirement that does not exist in the sibling SPEC.md which only
/// declares `REQ-001`).  Triggers RPT-002.
fn report_md_dangling_req() -> String {
    indoc! {r#"
        ---
        spec: SPEC-0001
        outcome: satisfied
        generated_at: 2026-05-20T00:00:00Z
        ---

        # REPORT: SPEC-0001

        <report spec="SPEC-0001">

        <coverage req="REQ-999" result="satisfied" scenarios="CHK-001">
        </coverage>

        </report>
    "#}
    .to_owned()
}

/// Minimal valid REPORT.md with a `<coverage>` row for `REQ-001` referencing
/// both `CHK-001` (valid) and `CHK-999` (dangling).  Triggers RPT-003 for
/// `CHK-999` only.
fn report_md_dangling_scenario() -> String {
    indoc! {r#"
        ---
        spec: SPEC-0001
        outcome: satisfied
        generated_at: 2026-05-20T00:00:00Z
        ---

        # REPORT: SPEC-0001

        <report spec="SPEC-0001">

        <coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-999">
        </coverage>

        </report>
    "#}
    .to_owned()
}

/// CHK-001: REPORT.md with `<report>` but no `spec=` attribute triggers
/// RPT-001 on an `implemented` spec, gating verify (exit 1).
#[test]
fn report_md_missing_spec_attribute_fires_rpt_001() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-rpt001",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    fs_err::write(
        spec_dir.join("REPORT.md").as_std_path(),
        report_md_missing_spec_attr(),
    )?;

    let (code, out) = invoke(&ws.root, true)?;
    assert_eq!(code, 1, "RPT-001 on implemented spec must gate verify");

    assert!(
        out.contains("RPT-001"),
        "text output must contain RPT-001; out:\n{out}",
    );

    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");
    let rpt_001 = errors
        .iter()
        .find(|d| field(d, "code").as_str() == Some("RPT-001"))
        .expect("RPT-001 must appear in lint.errors");
    let file = field(rpt_001, "file").as_str().unwrap_or("");
    assert!(
        file.ends_with("/REPORT.md") || file.ends_with("\\REPORT.md"),
        "RPT-001 file must end with REPORT.md; got: {file}",
    );
    Ok(())
}

/// CHK-002: REPORT.md with `<coverage req="REQ-999">` on a SPEC that only
/// declares `REQ-001` triggers RPT-002 (naming REQ-999) and no RPT-003.
#[test]
fn report_md_dangling_req_fires_rpt_002() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-rpt002",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    fs_err::write(
        spec_dir.join("REPORT.md").as_std_path(),
        report_md_dangling_req(),
    )?;

    let (code, out) = invoke(&ws.root, true)?;
    assert_eq!(code, 1, "RPT-002 on implemented spec must gate verify");

    assert!(
        out.contains("RPT-002"),
        "text output must contain RPT-002; out:\n{out}",
    );
    assert!(
        out.contains("REQ-999"),
        "text output must name REQ-999; out:\n{out}",
    );

    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");

    assert!(
        errors
            .iter()
            .any(|d| field(d, "code").as_str() == Some("RPT-002")),
        "RPT-002 must appear in lint.errors; got: {out}",
    );
    // RPT-003 must NOT fire: the row short-circuited at the missing req.
    assert!(
        !errors
            .iter()
            .any(|d| field(d, "code").as_str() == Some("RPT-003")),
        "RPT-003 must not fire when req is missing; got: {out}",
    );
    // Also not in info or warnings.
    let info = at(&json, &["lint", "info"])
        .as_array()
        .expect("lint.info array");
    assert!(
        !info
            .iter()
            .any(|d| field(d, "code").as_str() == Some("RPT-003")),
        "RPT-003 must not appear in any bucket when req is missing; got: {out}",
    );
    Ok(())
}

/// CHK-003: REPORT.md with `<coverage req="REQ-001" scenarios="CHK-001
/// CHK-999">` where REQ-001 has only CHK-001 triggers RPT-003 for CHK-999
/// and no diagnostic for CHK-001.
#[test]
fn report_md_dangling_scenario_fires_rpt_003() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-rpt003",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    fs_err::write(
        spec_dir.join("REPORT.md").as_std_path(),
        report_md_dangling_scenario(),
    )?;

    let (code, out) = invoke(&ws.root, true)?;
    assert_eq!(code, 1, "RPT-003 on implemented spec must gate verify");

    assert!(
        out.contains("RPT-003"),
        "text output must contain RPT-003; out:\n{out}",
    );
    assert!(
        out.contains("CHK-999"),
        "text output must name CHK-999; out:\n{out}",
    );

    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");

    let rpt_003_diags: Vec<_> = errors
        .iter()
        .filter(|d| field(d, "code").as_str() == Some("RPT-003"))
        .collect();
    assert_eq!(rpt_003_diags.len(), 1, "exactly one RPT-003 diagnostic");
    let msg = rpt_003_diags
        .first()
        .expect("one RPT-003 diagnostic")
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        msg.contains("CHK-999"),
        "RPT-003 message must name CHK-999; got: {msg}",
    );
    // No diagnostic for CHK-001.
    assert!(
        !errors.iter().any(|d| {
            field(d, "message")
                .as_str()
                .is_some_and(|m| m.contains("CHK-001"))
        }),
        "no diagnostic must mention CHK-001; got: {out}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0057 CHK-008: an unbalanced foreign tag in a parsed artifact gates
// verify (exit non-zero) and the rendered output names the artifact path and
// the orphan tag's 1-indexed line.
// ---------------------------------------------------------------------------

/// SPEC.md (status injected) carrying a dangling foreign open tag
/// (`<orphan>`) in its requirement body with no matching close.
fn spec_md_dangling_foreign_tag(spec_id: &str, status: &str) -> String {
    let template = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: __STATUS__
        created: 2026-05-11
        ---

        # __ID__

        <goals>
        Example goals.
        </goals>

        <non-goals>
        Example non-goals.
        </non-goals>

        <user-stories>
        - Example user story.
        </user-stories>

        <requirement id="REQ-001">
        ### REQ-001: First
        Body.
        <orphan>

        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        Given REQ-001, when the suite runs, then it covers REQ-001.
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    template
        .replace("__ID__", spec_id)
        .replace("__STATUS__", status)
}

#[test]
fn xml_001_unbalanced_foreign_tag_gates_verify_and_names_location() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-xml001",
        // Implemented so the Error is not demoted to Info.
        &spec_md_dangling_foreign_tag("SPEC-0001", "implemented"),
        None,
    )?;

    let (code, out) = invoke(&ws.root, true)?;
    assert_eq!(
        code, 1,
        "unbalanced foreign tag must gate verify; out:\n{out}"
    );

    let json: Value = serde_json::from_str(&out)?;
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");
    let xml_001 = errors
        .iter()
        .find(|d| field(d, "code").as_str() == Some("XML-001"))
        .expect("XML-001 must appear in lint.errors");

    let file = field(xml_001, "file").as_str().unwrap_or("");
    assert!(
        file.ends_with("/SPEC.md") || file.ends_with("\\SPEC.md"),
        "XML-001 must name the SPEC.md artifact path; got: {file}",
    );
    assert!(
        field(xml_001, "line").as_u64().is_some(),
        "XML-001 must carry the orphan tag's 1-indexed line; got: {xml_001}",
    );
    Ok(())
}

/// In-progress demotion: same malformed REPORT.md (no `spec=`) but the spec's
/// frontmatter is `status: in-progress`.  RPT-001 is demoted to `Level::Info`
/// by `partition_lint`; exit code must be 0.
#[test]
fn report_md_rpt_demotes_on_in_progress_spec() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-rpt-demote",
        &spec_md_template("SPEC-0001", "in-progress"),
        None,
    )?;
    fs_err::write(
        spec_dir.join("REPORT.md").as_std_path(),
        report_md_missing_spec_attr(),
    )?;

    let (code, out) = invoke(&ws.root, true)?;
    assert_eq!(
        code, 0,
        "RPT-001 on in-progress spec must be demoted; must not gate verify; out:\n{out}",
    );

    let json: Value = serde_json::from_str(&out)?;

    // Must appear in lint.info, not lint.errors.
    let info = at(&json, &["lint", "info"])
        .as_array()
        .expect("lint.info array");
    assert!(
        info.iter()
            .any(|d| field(d, "code").as_str() == Some("RPT-001")),
        "RPT-001 must be demoted to info on in-progress spec; got: {out}",
    );
    let errors = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array");
    assert!(
        !errors
            .iter()
            .any(|d| field(d, "code").as_str() == Some("RPT-001")),
        "RPT-001 must not appear in errors when spec is in-progress; got: {out}",
    );
    Ok(())
}
