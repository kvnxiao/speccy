#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy context`.
//!
//! Cover the command and selector contract; the JSON skeleton with spec
//! identity and the intent block populated; the task entry and covering
//! requirements via the shared core walk; the per-task journal inlined in
//! full, with an explicit empty marker when the file is absent; and the
//! navigation aids: the sibling-task index (id/state/covers only), the
//! repo-relative paths, and the suggested merge-base diff command. The
//! consistency section rides the same envelope. A property-style test pins
//! the governing size invariant: it emits a bundle, grows the spec around a
//! fixed task (one uncovered requirement, one new sibling task, one
//! foreign-task journal round, hash re-locked), re-emits, and asserts the two
//! payloads differ by exactly one sibling-index entry once the consistency
//! section is normalized.

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use common::TestResult;
use common::Workspace;
use common::sha256_hex;
use common::write_spec;
use indoc::indoc;
use predicates::str::contains;
use speccy_cli::context::ContextArgs;
use speccy_cli::context::ContextError;
use speccy_cli::context::run;
use speccy_core::task_lookup::LookupError;
use std::process::Command as StdCommand;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A SPEC.md whose intent surfaces carry distinctive marker strings, and
/// whose Summary narrative carries its own distinct marker. The Summary
/// marker must never appear in the emitted bundle.
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
/// none of the other three.
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

/// A TASKS.md with six tasks T-001..T-006, each body carrying a
/// distinctive `SIBLING_BODY_MARKER_N` string. The sibling index for any
/// one task must surface the other five as id/state/covers only — no body
/// marker may leak into the payload. Each task has a
/// distinct state so the index's state field can be asserted.
fn tasks_md_six(spec_id: &str) -> String {
    use std::fmt::Write as _;
    let states = [
        "completed",
        "completed",
        "in-progress",
        "pending",
        "pending",
        "pending",
    ];
    let mut body = format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\n\
         generated_at: 2026-06-10T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n",
    );
    for (idx, state) in states.iter().enumerate() {
        let n = idx + 1;
        write!(
            body,
            "<task id=\"T-{n:03}\" state=\"{state}\" covers=\"REQ-001\">\n\
             SIBLING_BODY_MARKER_{n}: body prose for task {n}.\n\n\
             <task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n",
        )
        .expect("writing to a String is infallible");
    }
    body
}

/// A TASKS.md whose tasks carry the given per-task states, in order from
/// `T-001`. Used by the consistency tests: a task `completed` in
/// TASKS.md with no matching `[SPEC/T-NNN]:` commit in git log surfaces a
/// `state_completed_no_commit` blocking drift, so marking several tasks
/// `completed` in a fresh repo (no per-task commits) drifts exactly those.
fn tasks_md_states(spec_id: &str, states: &[&str]) -> String {
    use std::fmt::Write as _;
    let mut body = format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\n\
         generated_at: 2026-06-10T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n",
    );
    for (idx, state) in states.iter().enumerate() {
        let n = idx + 1;
        write!(
            body,
            "<task id=\"T-{n:03}\" state=\"{state}\" covers=\"REQ-001\">\n\
             body prose for task {n}.\n\n\
             <task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n",
        )
        .expect("writing to a String is infallible");
    }
    body
}

/// Stand up a real git repo at `root` with one initial commit on `main`
/// and no `[SPEC/T-NNN]:`-prefixed commits. Returns `Ok(false)` (skip) when
/// git is unavailable, mirroring the live merge-base test's skip path.
fn init_repo_no_task_commits(root: &Utf8Path) -> TestResult<bool> {
    if run_git(root, &["init", "-q", "-b", "main"]).is_err() {
        eprintln!("git unavailable; skipping live consistency test");
        return Ok(false);
    }
    run_git(root, &["config", "user.email", "t@example.com"])?;
    run_git(root, &["config", "user.name", "t"])?;
    run_git(root, &["config", "commit.gpgsign", "false"])?;
    run_git(root, &["add", "-A"])?;
    run_git(root, &["commit", "-q", "-m", "base"])?;
    Ok(true)
}

/// Extract the consistency block from a parsed bundle: the `status` string
/// and the list of `task_id`s appearing in `drifts`.
fn consistency_of(value: &serde_json::Value) -> (String, Vec<String>) {
    let consistency = value
        .get("consistency")
        .expect("bundle carries a consistency section");
    let status = consistency
        .get("status")
        .and_then(serde_json::Value::as_str)
        .expect("consistency carries a status string")
        .to_owned();
    let drift_task_ids = consistency
        .get("drifts")
        .and_then(serde_json::Value::as_array)
        .expect("consistency carries a drifts array")
        .iter()
        .map(|d| {
            d.get("task_id")
                .and_then(serde_json::Value::as_str)
                .expect("each drift carries a task_id")
                .to_owned()
        })
        .collect();
    (status, drift_task_ids)
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
/// content.
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

/// A single-round per-task journal: one implementer plus two reviews, all
/// at round 1. Proves the latest-round slice keeps *every* block when the
/// journal has only one round — `journal_one_round` carries a lone
/// block, too thin to distinguish "all blocks" from "the first block".
fn journal_single_round(spec_id: &str) -> String {
    format!(
        "---\nspec: {spec_id}\ntask: T-001\n\
         generated_at: 2026-06-10T00:00:00Z\n---\n\n\
         <implementer date=\"2026-06-10T01:00:00Z\" model=\"m/low\" round=\"1\">\n\
         SINGLE_IMPL_MARKER\n</implementer>\n\n\
         <review date=\"2026-06-10T02:00:00Z\" model=\"m/low\" persona=\"business\" verdict=\"pass\" round=\"1\">\n\
         SINGLE_REVIEW_BUSINESS_MARKER\n</review>\n\n\
         <review date=\"2026-06-10T02:01:00Z\" model=\"m/low\" persona=\"tests\" verdict=\"pass\" round=\"1\">\n\
         SINGLE_REVIEW_TESTS_MARKER\n</review>\n",
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

/// Invoke `speccy context` in the human-readable text form (`--json` off)
/// and return its stdout. `--json` toggles representation only, so the text
/// form carries the same content the JSON form does.
fn invoke_text(root: &Utf8Path, selector: &str) -> TestResult<String> {
    let mut out: Vec<u8> = Vec::new();
    run(
        ContextArgs {
            selector: selector.to_owned(),
            json: false,
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
    // A selector failure must not write any partial bundle to stdout.
    assert!(
        out.is_empty(),
        "selector failure must produce no partial stdout; got {} bytes",
        out.len(),
    );
    err
}

// ---------------------------------------------------------------------------
// unqualified selector ambiguous across two specs → same
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
// a single spec + qualified selector → stdout parses as one JSON
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
// goals, non-goals, and both decision ids+bodies are present;
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

    // Identity: id / title / status from frontmatter.
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
    // payload.
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
// At the CLI boundary: the rendered ambiguity diagnostic class
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
// An invalid-format selector fails fast with the shared
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
// A five-requirement spec, a task covering two of them.
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
// The task's raw `<task>` body bytes appear alongside
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
// The two-round fixture journal (five round-1 blocks,
// three round-2 blocks). The bundle inlines only the round-2 blocks in full;
// no round-1 body marker survives in the serialized `blocks` array.
// ---------------------------------------------------------------------------

#[test]
fn bundle_inlines_only_latest_round_blocks() -> TestResult {
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

    // Only the three round-2 blocks survive (implementer + 2 reviews); the
    // five round-1 blocks are sliced out.
    assert_eq!(
        blocks.len(),
        3,
        "only the round-2 blocks are inlined; got {}",
        blocks.len(),
    );

    // Every surviving block is round 2 with a non-empty body.
    for block in blocks {
        assert_eq!(
            block.get("round").and_then(serde_json::Value::as_u64),
            Some(2),
            "every inlined block is from the latest round; got {block:?}",
        );
        let body = block
            .get("body")
            .and_then(serde_json::Value::as_str)
            .expect("each block carries a body");
        assert!(
            !body.trim().is_empty(),
            "block body is non-empty; got {block:?}"
        );
    }

    // The kinds match the round-2 slice: one implementer, two reviews, and no
    // blockers (the lone blockers block is round 1, so it is sliced out).
    let kinds: Vec<&str> = blocks
        .iter()
        .filter_map(|b| b.get("block").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(
        kinds.iter().filter(|k| **k == "implementer").count(),
        1,
        "one round-2 implementer block; got kinds {kinds:?}",
    );
    assert_eq!(
        kinds.iter().filter(|k| **k == "review").count(),
        2,
        "two round-2 review blocks; got kinds {kinds:?}",
    );
    assert_eq!(
        kinds.iter().filter(|k| **k == "blockers").count(),
        0,
        "the round-1 blockers block is sliced out; got kinds {kinds:?}",
    );

    // The round-2 bodies are present in full; none of the round-1 body
    // markers survive in any inlined block body (markers live only in bodies,
    // so concatenating them is a faithful read of the serialized blocks).
    let bodies: String = blocks
        .iter()
        .filter_map(|b| b.get("body").and_then(serde_json::Value::as_str))
        .collect();
    for marker in [
        "IMPL_R2_MARKER",
        "REVIEW_R2_SECURITY_MARKER",
        "REVIEW_R2_STYLE_MARKER",
    ] {
        assert!(
            bodies.contains(marker),
            "round-2 body marker {marker} must appear in the inlined blocks",
        );
    }
    for marker in [
        "IMPL_R1_MARKER",
        "REVIEW_R1_BUSINESS_MARKER",
        "REVIEW_R1_TESTS_MARKER",
        "REVIEW_R1_SECURITY_MARKER",
        "BLOCKERS_R1_MARKER",
    ] {
        assert!(
            !bodies.contains(marker),
            "round-1 body marker {marker} must NOT survive the latest-round slice",
        );
    }

    // Round-2 review metadata still projects: the style review carries its
    // persona and pass verdict.
    let style = blocks
        .iter()
        .find(|b| b.get("persona").and_then(serde_json::Value::as_str) == Some("style"))
        .expect("the round-2 style review is inlined");
    assert_eq!(
        style.get("verdict").and_then(serde_json::Value::as_str),
        Some("pass"),
        "the round-2 style review carries its verdict",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// A single-round journal fixture. Round 1 is the only —
// and therefore the latest — round, so the slice keeps every block in full.
// ---------------------------------------------------------------------------

#[test]
fn bundle_inlines_single_round_journal_in_full() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001")),
    )?;
    write_journal(&spec_dir, "T-001", &journal_single_round("SPEC-0042"))?;

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

    // All three round-1 blocks survive — the only round is the latest round.
    assert_eq!(
        blocks.len(),
        3,
        "every block of the single-round journal is inlined; got {}",
        blocks.len(),
    );
    for block in blocks {
        assert_eq!(
            block.get("round").and_then(serde_json::Value::as_u64),
            Some(1),
            "the sole round is round 1; got {block:?}",
        );
    }

    // Every block's full body is present in the inlined blocks.
    let bodies: String = blocks
        .iter()
        .filter_map(|b| b.get("body").and_then(serde_json::Value::as_str))
        .collect();
    for marker in [
        "SINGLE_IMPL_MARKER",
        "SINGLE_REVIEW_BUSINESS_MARKER",
        "SINGLE_REVIEW_TESTS_MARKER",
    ] {
        assert!(
            bodies.contains(marker),
            "single-round body marker {marker} must appear in the inlined blocks",
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// A fixture task with no journal file. The exit code is 0
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
// The two-round fixture. `journal.prior_rounds` is an
// attributes-only index of the five round-1 blocks in file order — the
// round-1 security review carries its persona and verdict, no entry
// serializes a `body` key, and no round-1 body marker survives in the
// `prior_rounds` array.
// ---------------------------------------------------------------------------

#[test]
fn prior_rounds_indexes_pre_latest_blocks_without_bodies() -> TestResult {
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
    let prior = journal
        .get("prior_rounds")
        .and_then(serde_json::Value::as_array)
        .expect("journal carries a prior_rounds array");

    // One entry per round-1 block: implementer, business review, tests review,
    // security review, blockers — five, in file order.
    let kinds: Vec<&str> = prior
        .iter()
        .filter_map(|e| e.get("block").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(
        kinds,
        vec!["implementer", "review", "review", "review", "blockers"],
        "prior_rounds lists the five round-1 blocks in file order; got {kinds:?}",
    );

    // Every entry is round 1 (strictly below the highest round, 2).
    for entry in prior {
        assert_eq!(
            entry.get("round").and_then(serde_json::Value::as_u64),
            Some(1),
            "every prior-round entry is from round 1; got {entry:?}",
        );
    }

    // No entry serializes a `body` key — the attrs shape omits it entirely
    // (not an empty string), which is what distinguishes index entries from
    // full blocks.
    for entry in prior {
        assert!(
            entry.get("body").is_none(),
            "no prior_rounds entry serializes a body key; got {entry:?}",
        );
    }

    // The round-1 security review carries its persona and verdict.
    let security = prior
        .iter()
        .find(|e| e.get("persona").and_then(serde_json::Value::as_str) == Some("security"))
        .expect("the round-1 security review is indexed");
    assert_eq!(
        security.get("verdict").and_then(serde_json::Value::as_str),
        Some("blocking"),
        "the round-1 security review carries its verdict",
    );

    // No round-1 body marker survives anywhere in the serialized prior_rounds
    // array (the whole subtree, serialized, must contain none of them).
    let prior_text = serde_json::to_string(prior)?;
    for marker in [
        "IMPL_R1_MARKER",
        "REVIEW_R1_BUSINESS_MARKER",
        "REVIEW_R1_TESTS_MARKER",
        "REVIEW_R1_SECURITY_MARKER",
        "BLOCKERS_R1_MARKER",
    ] {
        assert!(
            !prior_text.contains(marker),
            "round-1 body marker {marker} must NOT appear in the prior_rounds index",
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// A single-round journal and (separately) an absent
// journal both yield `prior_rounds: []`. A single round has nothing strictly
// below the highest round, and an absent journal has no entries at all.
// ---------------------------------------------------------------------------

#[test]
fn prior_rounds_is_empty_for_single_round_and_absent_journals() -> TestResult {
    // Single-round journal: every block is the latest round, none prior.
    let single = Workspace::new()?;
    let single_dir = write_spec(
        &single.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001")),
    )?;
    write_journal(&single_dir, "T-001", &journal_single_round("SPEC-0042"))?;
    let single_value = parse_one_json(&invoke_json(&single.root, "SPEC-0042/T-001")?);
    let single_prior = single_value
        .get("journal")
        .and_then(|j| j.get("prior_rounds"))
        .and_then(serde_json::Value::as_array)
        .expect("single-round journal carries a prior_rounds array");
    assert!(
        single_prior.is_empty(),
        "single-round journal yields prior_rounds: []; got {} entries",
        single_prior.len(),
    );

    // Absent journal: no entries, hence an empty index.
    let absent = Workspace::new()?;
    write_spec(
        &absent.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001")),
    )?;
    let absent_value = parse_one_json(&invoke_json(&absent.root, "SPEC-0042/T-001")?);
    let absent_prior = absent_value
        .get("journal")
        .and_then(|j| j.get("prior_rounds"))
        .and_then(serde_json::Value::as_array)
        .expect("absent journal carries a prior_rounds array");
    assert!(
        absent_prior.is_empty(),
        "absent journal yields prior_rounds: []; got {} entries",
        absent_prior.len(),
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// The text representation of the two-round fixture renders
// the round-2 block bodies in full, followed by a prior-rounds index naming
// each round-1 block's type, round, and persona / verdict where present —
// with no round-1 body content.
// ---------------------------------------------------------------------------

#[test]
fn text_journal_renders_latest_bodies_then_prior_rounds_index() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001")),
    )?;
    write_journal(&spec_dir, "T-001", &journal_two_rounds("SPEC-0042"))?;

    let text = invoke_text(&ws.root, "SPEC-0042/T-001")?;

    // Round-2 bodies render in full.
    for marker in [
        "IMPL_R2_MARKER",
        "REVIEW_R2_SECURITY_MARKER",
        "REVIEW_R2_STYLE_MARKER",
    ] {
        assert!(
            text.contains(marker),
            "round-2 body marker {marker} must render in the text journal",
        );
    }

    // A prior-rounds index section follows, after the round-2 bodies.
    let index_at = text
        .find("Prior rounds (index)")
        .expect("text renders a prior-rounds index header");
    let last_r2_body = text
        .find("REVIEW_R2_STYLE_MARKER")
        .expect("round-2 style body renders");
    assert!(
        index_at > last_r2_body,
        "the prior-rounds index renders after the round-2 block bodies",
    );

    // The index names each round-1 block's type and round; the security
    // review line carries its persona and verdict. No round-1 body content.
    // The index header precedes every block line, so these substrings can
    // only come from the index (no round-1 body content survives — asserted
    // below).
    assert!(
        text.contains("blockers round=1"),
        "index names the round-1 blockers block; got:\n{text}",
    );
    assert!(
        text.contains("review round=1 persona=security verdict=blocking"),
        "index names the round-1 security review's persona and verdict; got:\n{text}",
    );
    for marker in [
        "IMPL_R1_MARKER",
        "REVIEW_R1_BUSINESS_MARKER",
        "REVIEW_R1_TESTS_MARKER",
        "REVIEW_R1_SECURITY_MARKER",
        "BLOCKERS_R1_MARKER",
    ] {
        assert!(
            !text.contains(marker),
            "round-1 body marker {marker} must NOT render in the text journal",
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// At the binary boundary: a task with no journal exits 0.
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

// ---------------------------------------------------------------------------
// A six-task spec whose bodies each carry a distinctive
// marker. The bundle for T-003 carries a sibling index of the other five
// tasks (T-001, T-002, T-004, T-005, T-006) with only id/state/covers
// fields, and no sibling body marker appears anywhere in the payload.
// ---------------------------------------------------------------------------

#[test]
fn bundle_sibling_index_carries_id_state_covers_only_no_bodies() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_six("SPEC-0042")),
    )?;

    let stdout = invoke_json(&ws.root, "SPEC-0042/T-003")?;
    let value = parse_one_json(&stdout);

    let siblings = value
        .get("siblings")
        .and_then(serde_json::Value::as_array)
        .expect("bundle has a siblings array");

    // Exactly the five non-selected tasks, in TASKS.md declared order.
    let sib_ids: Vec<&str> = siblings
        .iter()
        .filter_map(|s| s.get("id").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(
        sib_ids,
        ["T-001", "T-002", "T-004", "T-005", "T-006"],
        "the sibling index excludes the selected T-003 and keeps declared order; got {sib_ids:?}",
    );

    // Each sibling entry carries exactly id, state, and covers — no body
    // field, and the state reflects the fixture's per-task state.
    let t001 = siblings.first().expect("first sibling is T-001");
    assert_eq!(
        t001.get("state").and_then(serde_json::Value::as_str),
        Some("completed"),
        "T-001 sibling state matches the fixture",
    );
    let t001_covers: Vec<&str> = t001
        .get("covers")
        .and_then(serde_json::Value::as_array)
        .expect("sibling covers array present")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert_eq!(t001_covers, ["REQ-001"], "sibling covers surfaced");
    for sib in siblings {
        let obj = sib.as_object().expect("each sibling is a JSON object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["covers", "id", "state"],
            "a sibling entry carries only id/state/covers; got {keys:?}",
        );
    }

    // No sibling task body marker leaks into the payload. The five
    // siblings of T-003 (T-001/2/4/5/6) must be body-free; the selected
    // task T-003's own body legitimately appears in the `task` entry,
    // so its marker is excluded from this check.
    for n in [1, 2, 4, 5, 6] {
        assert!(
            !stdout.contains(&format!("SIBLING_BODY_MARKER_{n}")),
            "sibling body marker for task {n} must not appear in the payload; payload: {stdout}",
        );
    }
    // Sanity: the selected task's own body marker *is* present (it is the
    // `task` entry), proving the absence above is sibling-scoped, not a
    // vacuous all-absent assertion.
    assert!(
        stdout.contains("SIBLING_BODY_MARKER_3"),
        "the selected task's own body marker is present in the task entry",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// The three repo-relative paths resolve to the actual files from
// the repo root, and the suggested diff command is in merge-base form
// against the default branch.
// ---------------------------------------------------------------------------

#[test]
fn bundle_carries_repo_relative_paths_and_diff_command() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001")),
    )?;

    let stdout = invoke_json(&ws.root, "SPEC-0042/T-001")?;
    let value = parse_one_json(&stdout);

    let paths = value.get("paths").expect("bundle has a paths section");
    assert_eq!(
        paths.get("spec_md").and_then(serde_json::Value::as_str),
        Some(".speccy/specs/0042-alpha/SPEC.md"),
        "SPEC.md path is repo-relative with forward slashes",
    );
    assert_eq!(
        paths.get("tasks_md").and_then(serde_json::Value::as_str),
        Some(".speccy/specs/0042-alpha/TASKS.md"),
        "TASKS.md path is repo-relative with forward slashes",
    );
    assert_eq!(
        paths.get("journal").and_then(serde_json::Value::as_str),
        Some(".speccy/specs/0042-alpha/journal/T-001.md"),
        "journal path is repo-relative and surfaced even when absent",
    );

    // The paths resolve to the actual files from the repo root: SPEC.md and
    // TASKS.md were just written by the fixture and must exist; the journal
    // file legitimately does not (round-1 task), so its presence is not
    // asserted.
    assert!(
        ws.root.join(".speccy/specs/0042-alpha/SPEC.md").exists(),
        "the surfaced SPEC.md path resolves to a real file",
    );
    assert!(
        ws.root.join(".speccy/specs/0042-alpha/TASKS.md").exists(),
        "the surfaced TASKS.md path resolves to a real file",
    );
    // `spec_dir` is the directory the paths are anchored to.
    assert_eq!(spec_dir, ws.root.join(".speccy/specs/0042-alpha"));

    // The diff command is in merge-base (triple-dot) form against a default
    // branch and ends at HEAD. Outside a git repo the default-branch probe
    // falls back to `main`, so the command is the runnable
    // `git diff main...HEAD`.
    let diff = value
        .get("diff_command")
        .and_then(serde_json::Value::as_str)
        .expect("bundle has a diff_command string");
    assert_eq!(
        diff, "git diff main...HEAD",
        "diff command is merge-base form against the fallback default branch",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// On a real git feature branch with an `origin/HEAD`
// pointing at the default branch, the suggested diff command names the
// resolved default branch in merge-base form and runs as-is from the repo
// root. This exercises the live default-branch + merge-base git machinery
// rather than only the fallback path.
// ---------------------------------------------------------------------------

#[test]
fn diff_command_uses_resolved_default_branch_and_runs_from_repo_root() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_covering("SPEC-0042", "REQ-001")),
    )?;

    // Stand up a real repo: an initial commit on `main`, a simulated
    // `origin/HEAD -> origin/main` remote-tracking ref, then a feature
    // branch with its own commit. If git is unavailable, skip — the
    // fallback path is covered by the unit test in `git.rs` and the
    // library test above.
    if run_git(&ws.root, &["init", "-q", "-b", "main"]).is_err() {
        eprintln!("git unavailable; skipping live merge-base test");
        return Ok(());
    }
    run_git(&ws.root, &["config", "user.email", "t@example.com"])?;
    run_git(&ws.root, &["config", "user.name", "t"])?;
    // Disable commit signing locally so the test does not depend on a
    // host's global signing config (which would fail the commit).
    run_git(&ws.root, &["config", "commit.gpgsign", "false"])?;
    run_git(&ws.root, &["add", "-A"])?;
    run_git(&ws.root, &["commit", "-q", "-m", "base"])?;
    // Create a remote-tracking ref + symbolic-ref so the default-branch
    // probe resolves `origin/main` rather than falling back.
    let head_sha = String::from_utf8(
        StdCommand::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(ws.root.as_std_path())
            .output()?
            .stdout,
    )?;
    run_git(
        &ws.root,
        &["update-ref", "refs/remotes/origin/main", head_sha.trim()],
    )?;
    run_git(
        &ws.root,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
    )?;
    run_git(&ws.root, &["checkout", "-q", "-b", "feature/example"])?;
    fs_err::write(ws.root.join("change.txt").as_std_path(), "feature change\n")?;
    run_git(&ws.root, &["add", "-A"])?;
    run_git(&ws.root, &["commit", "-q", "-m", "feature commit"])?;

    let stdout = invoke_json(&ws.root, "SPEC-0042/T-001")?;
    let value = parse_one_json(&stdout);
    let diff = value
        .get("diff_command")
        .and_then(serde_json::Value::as_str)
        .expect("bundle has a diff_command string");

    // The command names the resolved default branch in merge-base form.
    assert_eq!(
        diff, "git diff origin/main...HEAD",
        "diff command names the resolved origin/main in merge-base form; got {diff}",
    );

    // It runs as-is from the repo root and surfaces the feature change.
    let out = StdCommand::new("git")
        .args(["diff", "origin/main...HEAD"])
        .current_dir(ws.root.as_std_path())
        .output()?;
    assert!(
        out.status.success(),
        "the suggested diff command must run as-is from the repo root",
    );
    let diff_text = String::from_utf8(out.stdout)?;
    assert!(
        diff_text.contains("feature change"),
        "the diff covers the feature branch's change; got: {diff_text}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// A fixture workspace with drift affecting two tasks.
// Bundles for one drifted task and one undrifted task both exit 0, both
// carry the non-ok workspace status, the drifted task's bundle carries only
// its own drift entries, and the undrifted task's bundle carries an empty
// drift list.
// ---------------------------------------------------------------------------

#[test]
fn drifted_and_undrifted_bundles_share_status_but_scope_drifts_to_self() -> TestResult {
    let ws = Workspace::new()?;
    // Six tasks: T-001 and T-002 `completed` (no matching commit → each
    // drifts `state_completed_no_commit`); the rest `pending` (no drift).
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_states(
            "SPEC-0042",
            &[
                "completed",
                "completed",
                "pending",
                "pending",
                "pending",
                "pending",
            ],
        )),
    )?;

    if !init_repo_no_task_commits(&ws.root)? {
        return Ok(());
    }

    // Drifted task: T-001 carries its own drift only.
    let drifted = parse_one_json(&invoke_json(&ws.root, "SPEC-0042/T-001")?);
    let (drifted_status, drifted_ids) = consistency_of(&drifted);
    assert_ne!(
        drifted_status, "ok",
        "two completed tasks without commits make the workspace non-ok; got {drifted_status}",
    );
    assert_eq!(
        drifted_ids,
        vec!["T-001".to_owned()],
        "the drifted task's bundle carries only its own drift entry; T-002's must not appear",
    );

    // Undrifted task: T-003 (pending) carries the same non-ok workspace
    // status but an empty drift list.
    let undrifted = parse_one_json(&invoke_json(&ws.root, "SPEC-0042/T-003")?);
    let (undrifted_status, undrifted_ids) = consistency_of(&undrifted);
    assert_eq!(
        undrifted_status, drifted_status,
        "both bundles carry the same workspace-level status",
    );
    assert!(
        undrifted_ids.is_empty(),
        "the undrifted task's bundle carries an empty drift list; got {undrifted_ids:?}",
    );

    // Both emissions exit 0 — `speccy context` never refuses on drift. The
    // library `run` returning `Ok` (asserted by `invoke_json`'s `?`) is the
    // exit-0 contract; the binary path confirms the process exit code.
    Command::cargo_bin("speccy")?
        .args(["context", "SPEC-0042/T-001", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success();
    Command::cargo_bin("speccy")?
        .args(["context", "SPEC-0042/T-003", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success();
    Ok(())
}

// ---------------------------------------------------------------------------
// Drift affecting three tasks including T-002. The bundle
// for T-002 carries a non-ok status and exactly T-002's drift entries, with
// no other task's drift entries present.
// ---------------------------------------------------------------------------

#[test]
fn three_task_drift_surfaces_only_the_selected_tasks_entries() -> TestResult {
    let ws = Workspace::new()?;
    // T-001, T-002, T-004 `completed` (drift); T-003, T-005 `pending` (no
    // drift). Three tasks drift, one of them is the selected T-002.
    write_spec(
        &ws.root,
        "0042-alpha",
        &spec_md_five_requirements("SPEC-0042"),
        Some(&tasks_md_states(
            "SPEC-0042",
            &["completed", "completed", "pending", "completed", "pending"],
        )),
    )?;

    if !init_repo_no_task_commits(&ws.root)? {
        return Ok(());
    }

    let value = parse_one_json(&invoke_json(&ws.root, "SPEC-0042/T-002")?);
    let (status, drift_ids) = consistency_of(&value);
    assert_ne!(status, "ok", "three drifting tasks make the status non-ok");
    assert_eq!(
        drift_ids,
        vec!["T-002".to_owned()],
        "exactly T-002's drift entries appear; T-001's and T-004's must not",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// The size invariant as an executable contract. Emit a
// bundle for a fixed task, then grow the spec in three ways: one that must
// NOT enlarge the bundle (one uncovered requirement) plus the
// one that adds exactly one bounded line (one sibling task) plus one that
// must NOT enlarge it (a journal round on a *different* task). Re-lock the
// SPEC hash in TASKS.md frontmatter and re-emit. After normalizing the
// consistency section (the only field the SPEC edit can perturb), the two
// payloads must differ by exactly one added sibling-index entry and nothing
// else.
// ---------------------------------------------------------------------------

/// A SPEC.md carrying `n_requirements` requirements (REQ-001..REQ-NNN), each
/// with a body / done-when / behavior / one scenario, plus a goals,
/// non-goals, and Summary block. Used by the size-invariant test to grow
/// the spec by one uncovered requirement between emissions.
fn spec_md_n_requirements(spec_id: &str, n_requirements: u32) -> String {
    use std::fmt::Write as _;
    let mut body = format!(
        "---\nid: {spec_id}\nslug: x\ntitle: Example {spec_id}\n\
         status: in-progress\ncreated: 2026-06-10\n---\n\n# {spec_id}\n\n\
         ## Summary\n\nNarrative.\n\n\
         <goals>\n- A goal.\n</goals>\n\n\
         <non-goals>\n- A non-goal.\n</non-goals>\n\n\
         <user-stories>\n- A story.\n</user-stories>\n\n",
    );
    for n in 1..=n_requirements {
        write!(
            body,
            "<requirement id=\"REQ-{n:03}\">\n\
             ### REQ-{n:03}: Requirement {n}\n\
             Body {n}.\n\n\
             <done-when>\n- done {n}.\n</done-when>\n\n\
             <behavior>\n- behavior {n}.\n</behavior>\n\n\
             <scenario id=\"CHK-{n:03}\">\n\
             Given req {n}, when X, then Y.\n\
             </scenario>\n\
             </requirement>\n\n",
        )
        .expect("writing to a String is infallible");
    }
    body.push_str(
        "## Changelog\n\n<changelog>\n| Date | Author | Summary |\n\
         |------|--------|---------|\n| 2026-06-10 | t | init |\n</changelog>\n",
    );
    body
}

/// A TASKS.md whose frontmatter pins `spec_hash`, carrying the tasks in
/// `task_specs` as `(id, state, covers)`. Bodies carry no distinctive
/// markers — the size-invariant test asserts on structure, not body text.
fn tasks_md_with_hash(spec_id: &str, spec_hash: &str, task_specs: &[(&str, &str, &str)]) -> String {
    use std::fmt::Write as _;
    let mut body = format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: {spec_hash}\n\
         generated_at: 2026-06-10T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n",
    );
    for (id, state, covers) in task_specs {
        write!(
            body,
            "<task id=\"{id}\" state=\"{state}\" covers=\"{covers}\">\n\
             body prose for {id}.\n\n\
             <task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n",
        )
        .expect("writing to a String is infallible");
    }
    body
}

/// A minimal single-round per-task journal: one `<implementer>` block. Used
/// to add a journal round on a *foreign* task (one the selected task does
/// not read), which must not enlarge the bundle.
fn journal_one_round(spec_id: &str, task_id: &str) -> String {
    format!(
        "---\nspec: {spec_id}\ntask: {task_id}\n\
         generated_at: 2026-06-10T00:00:00Z\n---\n\n\
         <implementer date=\"2026-06-10T01:00:00Z\" model=\"m/low\" round=\"1\">\n\
         body\n</implementer>\n",
    )
}

/// Normalize a parsed bundle in place by replacing its `consistency` section
/// with a fixed sentinel. The consistency section is the only bundle field a
/// SPEC.md edit can perturb (its status reflects task-state-vs-git
/// correlation; re-locking the hash and editing requirements can shift what
/// the workspace scan reports), so the "differs only by one sibling
/// entry" claim is asserted modulo this field ("consistency fields
/// normalized").
fn normalize_consistency(value: &mut serde_json::Value) {
    if let Some(obj) = value.as_object_mut() {
        obj.insert("consistency".to_owned(), serde_json::json!("NORMALIZED"));
    }
}

#[test]
fn bundle_size_scales_with_task_not_spec() -> TestResult {
    let ws = Workspace::new()?;

    // Initial spec: three requirements, a single task T-001 covering REQ-001.
    let spec_before = spec_md_n_requirements("SPEC-0042", 3);
    let tasks_before = tasks_md_with_hash(
        "SPEC-0042",
        &sha256_hex(spec_before.as_bytes()),
        &[("T-001", "pending", "REQ-001")],
    );
    let spec_dir = write_spec(&ws.root, "0042-alpha", &spec_before, Some(&tasks_before))?;

    let mut before = parse_one_json(&invoke_json(&ws.root, "SPEC-0042/T-001")?);

    // The pre-growth bundle has zero siblings (T-001 is the only task).
    assert_eq!(
        before
            .get("siblings")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(0),
        "the single-task spec has no siblings before growth",
    );

    // Grow the spec in three ways:
    //   (1) one requirement T-001 does NOT cover (REQ-004) — adds nothing;
    //   (2) one new sibling task T-002 — adds exactly one index entry;
    //   (3) one journal round on the foreign task T-002 — adds nothing to
    //       T-001's bundle (each task reads only its own journal).
    let spec_after = spec_md_n_requirements("SPEC-0042", 4);
    let tasks_after = tasks_md_with_hash(
        "SPEC-0042",
        // Re-lock the hash to the edited SPEC.md, mirroring the reconcile
        // step. The bundle does not read the hash; relocking proves the
        // invariant is independent of the lock value.
        &sha256_hex(spec_after.as_bytes()),
        &[
            ("T-001", "pending", "REQ-001"),
            ("T-002", "pending", "REQ-002"),
        ],
    );
    fs_err::write(spec_dir.join("SPEC.md").as_std_path(), &spec_after)?;
    fs_err::write(spec_dir.join("TASKS.md").as_std_path(), &tasks_after)?;
    write_journal(&spec_dir, "T-002", &journal_one_round("SPEC-0042", "T-002"))?;

    let mut after = parse_one_json(&invoke_json(&ws.root, "SPEC-0042/T-001")?);

    // Normalize the one field a SPEC edit may perturb, then diff.
    normalize_consistency(&mut before);
    normalize_consistency(&mut after);

    // The ONLY difference is the siblings array: before has zero entries,
    // after has exactly one (the new T-002), carrying only id/state/covers.
    let after_siblings = after
        .get("siblings")
        .and_then(serde_json::Value::as_array)
        .expect("after-growth bundle has a siblings array")
        .clone();
    assert_eq!(
        after_siblings.len(),
        1,
        "growing the spec by one task adds exactly one sibling entry; got {after_siblings:?}",
    );
    let sibling = after_siblings.first().expect("the one added sibling");
    let mut keys: Vec<&str> = sibling
        .as_object()
        .expect("sibling is an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["covers", "id", "state"],
        "the added sibling carries only id/state/covers; got {keys:?}",
    );
    assert_eq!(
        sibling.get("id").and_then(serde_json::Value::as_str),
        Some("T-002"),
        "the added sibling is the new T-002",
    );

    // Every other field is byte-for-byte identical. Project both payloads
    // with the siblings array removed and assert deep equality — this is the
    // executable form of "nothing else changed": the uncovered REQ-004 and
    // the foreign T-002 journal round left no trace in T-001's bundle.
    let strip_siblings = |v: &serde_json::Value| -> serde_json::Value {
        let mut obj = v.as_object().expect("bundle is an object").clone();
        obj.remove("siblings");
        serde_json::Value::Object(obj)
    };
    assert_eq!(
        strip_siblings(&before),
        strip_siblings(&after),
        "outside the siblings index, the bundle is invariant to spec growth",
    );

    // Guard against a vacuous pass: the spec really did grow. REQ-004 was
    // added (so the uncovered-requirement growth was exercised) yet must be
    // absent from T-001's payload, and the foreign T-002 journal exists on
    // disk yet contributes nothing to T-001's journal section.
    assert!(
        spec_after.contains("REQ-004"),
        "the test actually added an uncovered REQ-004 to SPEC.md",
    );
    let after_str = serde_json::to_string(&after)?;
    assert!(
        !after_str.contains("REQ-004"),
        "the uncovered REQ-004 must not appear in T-001's bundle; payload: {after_str}",
    );
    assert!(
        spec_dir.join("journal/T-002.md").exists(),
        "the foreign T-002 journal round was written to disk",
    );
    assert_eq!(
        after
            .get("journal")
            .and_then(|j| j.get("exists"))
            .and_then(serde_json::Value::as_bool),
        Some(false),
        "T-001's own journal section stays empty despite T-002's new journal",
    );
    Ok(())
}

/// Run a git subcommand in `cwd`, returning an error when git is missing or
/// the command fails. Used by the live merge-base test to stand up a repo.
fn run_git(cwd: &Utf8Path, args: &[&str]) -> TestResult {
    let status = StdCommand::new("git")
        .args(args)
        .current_dir(cwd.as_std_path())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {args:?} failed with {status}").into())
    }
}
