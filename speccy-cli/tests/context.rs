#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy context` (SPEC-0056).
//!
//! T-002 establishes the command, the selector contract, and the JSON
//! skeleton with spec identity (REQ-001 / REQ-002) and the intent block
//! (REQ-002) populated. Later tasks (T-003..T-006) extend the same
//! envelope; tests for those sections land with their tasks.

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use common::TestResult;
use common::Workspace;
use common::write_spec;
use indoc::indoc;
use predicates::str::contains;
use speccy_cli::context::ContextArgs;
use speccy_cli::context::ContextError;
use speccy_cli::context::run;
use speccy_core::task_lookup::LookupError;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A SPEC.md whose intent surfaces carry distinctive marker strings, and
/// whose Summary narrative carries its own distinct marker. The Summary
/// marker must never appear in the emitted bundle (REQ-002 / CHK-003).
///
/// `__ID__` substitutes the spec id; the slug/title are fixed.
fn spec_md_intent_markers(spec_id: &str) -> String {
    let template = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: in-progress
        created: 2026-06-10
        ---

        # __ID__

        ## Summary

        This summary contains the marker SUMMARY_NARRATIVE_MARKER which must
        never leak into the task-scoped bundle payload.

        <goals>
        - GOALS_MARKER_ALPHA: first goal bullet.
        - GOALS_MARKER_BETA: second goal bullet.
        </goals>

        <non-goals>
        - NONGOALS_MARKER: the one and only non-goal.
        </non-goals>

        <user-stories>
        - USER_STORY_MARKER: a story that is not bundled.
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
        Given REQ-001, when the suite runs, then it covers REQ-001.
        </scenario>
        </requirement>

        <decision id="DEC-001">
        DECISION_MARKER_ONE: the first decision body.
        </decision>

        <decision id="DEC-002">
        DECISION_MARKER_TWO: the second decision body.
        </decision>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-06-10 | t | init |
        </changelog>
    "#};
    template.replace("__ID__", spec_id)
}

/// A minimal TASKS.md with a single task T-001 covering REQ-001.
fn tasks_md_single(spec_id: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\n\
         generated_at: 2026-06-10T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n\n\n\
         <task id=\"T-001\" state=\"pending\" covers=\"REQ-001\">\nstub\n\n\
         <task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n",
    )
}

fn invoke_json(root: &Utf8Path, selector: &str) -> TestResult<String> {
    let mut out: Vec<u8> = Vec::new();
    run(
        ContextArgs {
            selector: selector.to_owned(),
            json: true,
        },
        root,
        &mut out,
    )?;
    Ok(String::from_utf8(out)?)
}

/// Parse `stdout` as exactly one JSON document. Kept in a non-`Result`
/// helper so the `.expect()` lives outside the `-> TestResult` test
/// bodies (matching the project's `unwrap_in_result` clippy posture).
fn parse_one_json(stdout: &str) -> serde_json::Value {
    serde_json::from_str(stdout.trim_end()).expect("stdout must be one JSON document")
}

fn invoke_json_err(root: &Utf8Path, selector: &str) -> ContextError {
    let mut out: Vec<u8> = Vec::new();
    let err = run(
        ContextArgs {
            selector: selector.to_owned(),
            json: true,
        },
        root,
        &mut out,
    )
    .expect_err("expected ContextError");
    // A selector failure must not write any partial bundle to stdout
    // (REQ-001 done-when: selector failures exit non-zero without partial
    // output).
    assert!(
        out.is_empty(),
        "selector failure must produce no partial stdout; got {} bytes",
        out.len(),
    );
    err
}

// ---------------------------------------------------------------------------
// CHK-001: unqualified selector ambiguous across two specs → same
// diagnostic class `speccy check` produces (an `Ambiguous` LookupError).
// ---------------------------------------------------------------------------

#[test]
fn ambiguous_unqualified_selector_surfaces_lookup_ambiguity() -> TestResult {
    let ws = Workspace::new()?;
    // Two specs that both contain a task T-001.
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_intent_markers("SPEC-0042"),
        Some(&tasks_md_single("SPEC-0042")),
    )?;
    write_spec(
        &ws.root,
        "0043-beta",
        &spec_md_intent_markers("SPEC-0043"),
        Some(&tasks_md_single("SPEC-0043")),
    )?;

    let err = invoke_json_err(&ws.root, "T-001");
    // The selector failure is the shared `LookupError::Ambiguous` — the
    // exact same class `speccy check T-001` produces against the same
    // workspace. The dispatcher renders both through `report_lookup_error`,
    // so naming both candidate specs is part of the contract.
    let ContextError::TaskLookup(LookupError::Ambiguous {
        task_id,
        candidate_specs,
    }) = &err
    else {
        return Err(format!("expected Ambiguous LookupError, got {err:?}").into());
    };
    assert_eq!(task_id, "T-001");
    assert!(
        candidate_specs.contains(&"SPEC-0042".to_owned())
            && candidate_specs.contains(&"SPEC-0043".to_owned()),
        "ambiguity must name both candidate specs; got {candidate_specs:?}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-002: a single spec + qualified selector → stdout parses as one JSON
// document whose first serialized field is `schema_version` = 1.
// ---------------------------------------------------------------------------

#[test]
fn qualified_selector_emits_json_with_schema_version_first() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_intent_markers("SPEC-0042"),
        Some(&tasks_md_single("SPEC-0042")),
    )?;

    let stdout = invoke_json(&ws.root, "SPEC-0042/T-001")?;

    // (1) Parses as exactly one JSON document.
    let value = parse_one_json(&stdout);
    assert_eq!(
        value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64),
        Some(1),
        "schema_version must be 1; payload: {stdout}",
    );

    // (2) `schema_version` is the FIRST serialized field. `serde_json::Value`
    // does not preserve key order, so assert against the raw serialized
    // prefix — the struct field order is the contract.
    let trimmed = stdout.trim_start();
    assert!(
        trimmed.starts_with("{\"schema_version\":1"),
        "schema_version must be the first serialized field; got prefix: {}",
        trimmed.chars().take(40).collect::<String>(),
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-003: goals, non-goals, and both decision ids+bodies are present;
// the Summary narrative marker is absent from the payload.
// ---------------------------------------------------------------------------

#[test]
fn bundle_carries_intent_and_excludes_summary_marker() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_intent_markers("SPEC-0042"),
        Some(&tasks_md_single("SPEC-0042")),
    )?;

    let stdout = invoke_json(&ws.root, "SPEC-0042/T-001")?;
    let value = parse_one_json(&stdout);

    // Identity (REQ-002): id / title / status from frontmatter.
    let spec = value.get("spec").expect("bundle has spec identity");
    assert_eq!(
        spec.get("id").and_then(serde_json::Value::as_str),
        Some("SPEC-0042"),
    );
    assert_eq!(
        spec.get("title").and_then(serde_json::Value::as_str),
        Some("Example SPEC-0042"),
    );
    assert_eq!(
        spec.get("status").and_then(serde_json::Value::as_str),
        Some("in-progress"),
    );

    let intent = value.get("intent").expect("bundle has intent block");

    // Goals + non-goals bullet text present.
    let goals = intent
        .get("goals")
        .and_then(serde_json::Value::as_str)
        .expect("goals body present");
    assert!(
        goals.contains("GOALS_MARKER_ALPHA") && goals.contains("GOALS_MARKER_BETA"),
        "both goals bullets must appear; goals: {goals}",
    );
    let non_goals = intent
        .get("non_goals")
        .and_then(serde_json::Value::as_str)
        .expect("non_goals body present");
    assert!(
        non_goals.contains("NONGOALS_MARKER"),
        "non-goal bullet must appear; non_goals: {non_goals}",
    );

    // Both decisions present with id + body.
    let decisions = intent
        .get("decisions")
        .and_then(serde_json::Value::as_array)
        .expect("decisions array present");
    assert_eq!(
        decisions.len(),
        2,
        "exactly two decisions; got {decisions:?}"
    );
    let dec_ids: Vec<&str> = decisions
        .iter()
        .filter_map(|d| d.get("id").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(
        dec_ids,
        ["DEC-001", "DEC-002"],
        "decision ids in declared order"
    );
    let dec_one = decisions
        .first()
        .and_then(|d| d.get("body"))
        .and_then(serde_json::Value::as_str)
        .expect("DEC-001 body present");
    assert!(
        dec_one.contains("DECISION_MARKER_ONE"),
        "DEC-001 body must appear; got: {dec_one}",
    );

    // The Summary narrative marker, user-story marker, and the
    // (uncovered-here) nothing-else surfaces must be absent from the whole
    // payload (REQ-002: Summary / user stories excluded).
    assert!(
        !stdout.contains("SUMMARY_NARRATIVE_MARKER"),
        "Summary narrative must be excluded from the bundle; payload: {stdout}",
    );
    assert!(
        !stdout.contains("USER_STORY_MARKER"),
        "user stories must be excluded from the bundle; payload: {stdout}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-001 at the CLI boundary: the rendered ambiguity diagnostic class
// matches `speccy check`'s. Both commands route the same `LookupError`
// through `report_lookup_error`, so the binary exits non-zero and emits
// the shared "is ambiguous; matches in N specs" wording plus a
// disambiguation line per candidate spec — the same class
// `speccy check T-001` produces.
// ---------------------------------------------------------------------------

#[test]
fn binary_ambiguous_selector_matches_check_diagnostic_class() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_intent_markers("SPEC-0042"),
        Some(&tasks_md_single("SPEC-0042")),
    )?;
    write_spec(
        &ws.root,
        "0043-beta",
        &spec_md_intent_markers("SPEC-0043"),
        Some(&tasks_md_single("SPEC-0043")),
    )?;

    // `speccy context T-001` — ambiguous: must fail non-zero with the
    // shared diagnostic, naming both candidate specs in disambiguation
    // lines.
    let mut ctx_cmd = Command::cargo_bin("speccy")?;
    ctx_cmd
        .args(["context", "T-001", "--json"])
        .current_dir(ws.root.as_std_path());
    ctx_cmd
        .assert()
        .failure()
        .stdout(predicates::str::is_empty())
        .stderr(contains("T-001 is ambiguous; matches in 2 specs."))
        .stderr(contains("speccy context SPEC-0042/T-001"))
        .stderr(contains("speccy context SPEC-0043/T-001"));

    // `speccy check T-001` — same class: same ambiguity headline (the
    // command name in the disambiguation lines differs by design, but the
    // diagnostic CLASS — ambiguity, candidate enumeration — is identical).
    let mut check_cmd = Command::cargo_bin("speccy")?;
    check_cmd
        .args(["check", "T-001"])
        .current_dir(ws.root.as_std_path());
    check_cmd
        .assert()
        .failure()
        .stderr(contains("T-001 is ambiguous; matches in 2 specs."));
    Ok(())
}

// ---------------------------------------------------------------------------
// REQ-001 done-when: an invalid-format selector fails fast with the shared
// InvalidFormat diagnostic and writes no partial stdout.
// ---------------------------------------------------------------------------

#[test]
fn invalid_selector_format_surfaces_lookup_invalid_format() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_intent_markers("SPEC-0042"),
        Some(&tasks_md_single("SPEC-0042")),
    )?;

    let err = invoke_json_err(&ws.root, "NOT-A-SELECTOR");
    assert!(
        matches!(
            &err,
            ContextError::TaskLookup(LookupError::InvalidFormat { arg }) if arg == "NOT-A-SELECTOR",
        ),
        "expected InvalidFormat carrying the raw selector; got {err:?}",
    );
    Ok(())
}
