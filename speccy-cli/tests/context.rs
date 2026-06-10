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
//! (REQ-002) populated. T-003 adds the task entry and the covering
//! requirements via the shared core walk (REQ-003). T-004 inlines the
//! per-task journal in full, with an explicit empty marker when the file is
//! absent (REQ-004). Later tasks (T-005..T-006) extend the same envelope;
//! tests for those sections land with their tasks.

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

/// A SPEC.md with five requirements, each carrying a distinctive marker
/// in its body, done-when, behavior, and one scenario. The task covers
/// only two of them, so the bundle must surface exactly those two and
/// none of the other three (REQ-003 / CHK-004).
///
/// `__ID__` substitutes the spec id.
fn spec_md_five_requirements(spec_id: &str) -> String {
    use std::fmt::Write as _;
    let mut body = String::from(indoc! {r"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: in-progress
        created: 2026-06-10
        ---

        # __ID__

        <goals>
        - A goal.
        </goals>

        <non-goals>
        - A non-goal.
        </non-goals>

        <user-stories>
        - A story.
        </user-stories>

    "});
    for n in 1..=5 {
        write!(
            body,
            "<requirement id=\"REQ-{n:03}\">\n\
             ### REQ-{n:03}: Requirement {n}\n\
             REQ{n:03}_BODY_MARKER: requirement {n} prose body.\n\
             \n\
             <done-when>\n- REQ{n:03}_DONEWHEN_MARKER.\n</done-when>\n\
             \n\
             <behavior>\n- REQ{n:03}_BEHAVIOR_MARKER.\n</behavior>\n\
             \n\
             <scenario id=\"CHK-{n:03}\">\n\
             REQ{n:03}_SCENARIO_MARKER: given req {n}, when X, then Y.\n\
             </scenario>\n\
             </requirement>\n\n",
        )
        .expect("writing to a String is infallible");
    }
    body.push_str(indoc! {r"
        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-06-10 | t | init |
        </changelog>
    "});
    body.replace("__ID__", spec_id)
}

/// A TASKS.md with a single task T-001 covering the given requirement ids.
fn tasks_md_covering(spec_id: &str, covers: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\n\
         generated_at: 2026-06-10T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n\n\n\
         <task id=\"T-001\" state=\"pending\" covers=\"{covers}\">\n\
         TASK_BODY_MARKER: the task body prose.\n\n\
         <task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n",
    )
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

/// A per-task journal for `T-001` with rounds 1–2: two `<implementer>`
/// blocks, five `<review>` blocks, and one `<blockers>` block (eight total),
/// laid out in a round-monotonic file order the `journal_xml` parser
/// accepts (round 1 first, then round 2). Each block carries a distinctive
/// body marker so the bundle's projection can be asserted against file
/// content (REQ-004 / CHK-006).
fn journal_two_rounds(spec_id: &str) -> String {
    format!(
        "---\nspec: {spec_id}\ntask: T-001\n\
         generated_at: 2026-06-10T00:00:00Z\n---\n\n\
         <implementer date=\"2026-06-10T01:00:00Z\" model=\"m/low\" round=\"1\">\n\
         IMPL_R1_MARKER\n</implementer>\n\n\
         <review date=\"2026-06-10T02:00:00Z\" model=\"m/low\" persona=\"business\" verdict=\"pass\" round=\"1\">\n\
         REVIEW_R1_BUSINESS_MARKER\n</review>\n\n\
         <review date=\"2026-06-10T02:01:00Z\" model=\"m/low\" persona=\"tests\" verdict=\"pass\" round=\"1\">\n\
         REVIEW_R1_TESTS_MARKER\n</review>\n\n\
         <review date=\"2026-06-10T02:02:00Z\" model=\"m/low\" persona=\"security\" verdict=\"blocking\" round=\"1\">\n\
         REVIEW_R1_SECURITY_MARKER\n</review>\n\n\
         <blockers date=\"2026-06-10T03:00:00Z\" round=\"1\">\n\
         BLOCKERS_R1_MARKER\n</blockers>\n\n\
         <implementer date=\"2026-06-10T04:00:00Z\" model=\"m/low\" round=\"2\">\n\
         IMPL_R2_MARKER\n</implementer>\n\n\
         <review date=\"2026-06-10T05:00:00Z\" model=\"m/low\" persona=\"security\" verdict=\"pass\" round=\"2\">\n\
         REVIEW_R2_SECURITY_MARKER\n</review>\n\n\
         <review date=\"2026-06-10T05:01:00Z\" model=\"m/low\" persona=\"style\" verdict=\"pass\" round=\"2\">\n\
         REVIEW_R2_STYLE_MARKER\n</review>\n",
    )
}

/// Write a per-task journal at `<spec-dir>/journal/<task-id>.md`.
fn write_journal(spec_dir: &Utf8Path, task_id: &str, body: &str) -> TestResult {
    let journal = spec_dir.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    fs_err::write(journal.join(format!("{task_id}.md")).as_std_path(), body)?;
    Ok(())
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

// ---------------------------------------------------------------------------
// CHK-004 (REQ-003): a five-requirement spec, a task covering two of them.
// The two covered requirements appear with done-when, behavior, and
// scenario content; none of the other three requirement ids appear
// anywhere in the payload.
// ---------------------------------------------------------------------------

#[test]
fn bundle_carries_only_the_two_covered_requirements_with_full_content() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001 REQ-003")),
    )?;

    let stdout = invoke_json(&ws.root, "SPEC-0042/T-001")?;
    let value = parse_one_json(&stdout);

    let requirements = value
        .get("requirements")
        .and_then(serde_json::Value::as_array)
        .expect("bundle has requirements array");

    // Exactly the two covered requirements, in covers-list order.
    let req_ids: Vec<&str> = requirements
        .iter()
        .filter_map(|r| r.get("id").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(
        req_ids,
        ["REQ-001", "REQ-003"],
        "covered requirements resolve in covers-list order; got {req_ids:?}",
    );

    // Each covered requirement carries its done-when, behavior, and
    // scenario content.
    for (req, n) in requirements.iter().zip([1_u32, 3]) {
        let done_when = req
            .get("done_when")
            .and_then(serde_json::Value::as_str)
            .expect("done_when present");
        assert!(
            done_when.contains(&format!("REQ{n:03}_DONEWHEN_MARKER")),
            "REQ-{n:03} done-when content must appear; got: {done_when}",
        );
        let behavior = req
            .get("behavior")
            .and_then(serde_json::Value::as_str)
            .expect("behavior present");
        assert!(
            behavior.contains(&format!("REQ{n:03}_BEHAVIOR_MARKER")),
            "REQ-{n:03} behavior content must appear; got: {behavior}",
        );
        let scenarios = req
            .get("scenarios")
            .and_then(serde_json::Value::as_array)
            .expect("scenarios array present");
        let scenario_ids: Vec<&str> = scenarios
            .iter()
            .filter_map(|s| s.get("id").and_then(serde_json::Value::as_str))
            .collect();
        assert_eq!(
            scenario_ids,
            [format!("CHK-{n:03}")],
            "REQ-{n:03} carries its own scenario; got {scenario_ids:?}",
        );
        let scenario_body = scenarios
            .first()
            .and_then(|s| s.get("body"))
            .and_then(serde_json::Value::as_str)
            .expect("scenario body present");
        assert!(
            scenario_body.contains(&format!("REQ{n:03}_SCENARIO_MARKER")),
            "REQ-{n:03} scenario content must appear; got: {scenario_body}",
        );
    }

    // None of the three uncovered requirements' ids or content appear
    // anywhere in the payload.
    for n in [2_u32, 4, 5] {
        assert!(
            !stdout.contains(&format!("REQ-{n:03}")),
            "uncovered REQ-{n:03} id must be absent from the payload; payload: {stdout}",
        );
        assert!(
            !stdout.contains(&format!("REQ{n:03}_BODY_MARKER")),
            "uncovered REQ-{n:03} body must be absent from the payload",
        );
        assert!(
            !stdout.contains(&format!("CHK-{n:03}")),
            "uncovered CHK-{n:03} must be absent from the payload",
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// REQ-003 behavior: the task's raw `<task>` body bytes appear alongside
// the parsed id, state, and covers.
// ---------------------------------------------------------------------------

#[test]
fn bundle_carries_task_entry_with_raw_body_and_parsed_fields() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001 REQ-003")),
    )?;

    let stdout = invoke_json(&ws.root, "SPEC-0042/T-001")?;
    let value = parse_one_json(&stdout);

    let task = value.get("task").expect("bundle has task entry");
    assert_eq!(
        task.get("id").and_then(serde_json::Value::as_str),
        Some("T-001"),
    );
    assert_eq!(
        task.get("state").and_then(serde_json::Value::as_str),
        Some("pending"),
    );
    let covers: Vec<&str> = task
        .get("covers")
        .and_then(serde_json::Value::as_array)
        .expect("covers array present")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert_eq!(
        covers,
        ["REQ-001", "REQ-003"],
        "parsed covers in source order; got {covers:?}",
    );
    // The raw `<task>` body bytes appear in the entry.
    let body = task
        .get("body")
        .and_then(serde_json::Value::as_str)
        .expect("task body present");
    assert!(
        body.contains("TASK_BODY_MARKER"),
        "raw task body bytes must appear in the entry; got: {body}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-006 (REQ-004): a fixture journal with rounds 1–2, five review blocks,
// and one blockers block. The bundle's journal section contains all eight
// blocks (2 implementer + 5 review + 1 blockers) with round attributes
// matching the file.
// ---------------------------------------------------------------------------

#[test]
fn bundle_inlines_full_journal_with_all_blocks_and_rounds() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001")),
    )?;
    write_journal(&spec_dir, "T-001", &journal_two_rounds("SPEC-0042"))?;

    let stdout = invoke_json(&ws.root, "SPEC-0042/T-001")?;
    let value = parse_one_json(&stdout);

    let journal = value.get("journal").expect("bundle has journal section");
    assert_eq!(
        journal.get("exists").and_then(serde_json::Value::as_bool),
        Some(true),
        "journal section marks the file present",
    );

    let blocks = journal
        .get("blocks")
        .and_then(serde_json::Value::as_array)
        .expect("journal carries a blocks array");
    assert_eq!(
        blocks.len(),
        8,
        "all eight blocks (2 implementer + 5 review + 1 blockers) are inlined; got {}",
        blocks.len(),
    );

    // The block kinds and their counts match the file.
    let kinds: Vec<&str> = blocks
        .iter()
        .filter_map(|b| b.get("block").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(
        kinds.iter().filter(|k| **k == "implementer").count(),
        2,
        "two implementer blocks; got kinds {kinds:?}",
    );
    assert_eq!(
        kinds.iter().filter(|k| **k == "review").count(),
        5,
        "five review blocks; got kinds {kinds:?}",
    );
    assert_eq!(
        kinds.iter().filter(|k| **k == "blockers").count(),
        1,
        "one blockers block; got kinds {kinds:?}",
    );

    // Round attributes match the file: the first five blocks are round 1
    // (implementer + 3 reviews + blockers), the last three are round 2
    // (implementer + 2 reviews), in file order.
    let rounds: Vec<u64> = blocks
        .iter()
        .filter_map(|b| b.get("round").and_then(serde_json::Value::as_u64))
        .collect();
    assert_eq!(
        rounds,
        [1, 1, 1, 1, 1, 2, 2, 2],
        "round attributes match the file in order; got {rounds:?}",
    );

    // The blocks-in-file-order projection preserves each block's body
    // verbatim, so the retry context (prior handoffs, verdicts, blockers
    // directives) is all present.
    for marker in [
        "IMPL_R1_MARKER",
        "REVIEW_R1_SECURITY_MARKER",
        "BLOCKERS_R1_MARKER",
        "IMPL_R2_MARKER",
        "REVIEW_R2_STYLE_MARKER",
    ] {
        assert!(
            stdout.contains(marker),
            "journal body marker {marker} must appear in the payload",
        );
    }

    // A blocking review block carries its persona and verdict (sufficient
    // for retry context); the security round-1 review blocked.
    let blocking = blocks
        .iter()
        .find(|b| b.get("verdict").and_then(serde_json::Value::as_str) == Some("blocking"))
        .expect("the round-1 security review is a blocking verdict");
    assert_eq!(
        blocking.get("persona").and_then(serde_json::Value::as_str),
        Some("security"),
        "the blocking review carries its persona",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// CHK-007 (REQ-004): a fixture task with no journal file. The exit code is 0
// and the journal section carries an explicit absence marker with zero
// blocks.
// ---------------------------------------------------------------------------

#[test]
fn bundle_journal_absent_yields_explicit_empty_marker_and_success() -> TestResult {
    let ws = Workspace::new()?;
    // No journal file is written for this task.
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001")),
    )?;

    // The emission itself succeeds (the `?` would propagate any error),
    // standing in for the exit-0 contract at the library boundary.
    let stdout = invoke_json(&ws.root, "SPEC-0042/T-001")?;
    let value = parse_one_json(&stdout);

    let journal = value.get("journal").expect("bundle has journal section");
    assert_eq!(
        journal.get("exists").and_then(serde_json::Value::as_bool),
        Some(false),
        "absent journal is marked exists: false",
    );
    let blocks = journal
        .get("blocks")
        .and_then(serde_json::Value::as_array)
        .expect("journal carries a blocks array even when absent");
    assert!(
        blocks.is_empty(),
        "an absent journal carries zero blocks; got {}",
        blocks.len(),
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// REQ-004 / CHK-007 at the binary boundary: a task with no journal exits 0.
// The library-level test proves the empty marker; this proves the process
// exit code is genuinely 0 (not just an absence of a Rust error).
// ---------------------------------------------------------------------------

#[test]
fn binary_journal_absent_exits_zero() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001")),
    )?;

    Command::cargo_bin("speccy")?
        .args(["context", "SPEC-0042/T-001", "--json"])
        .current_dir(&ws.root)
        .assert()
        .success()
        .stdout(contains("\"exists\":false"));
    Ok(())
}
