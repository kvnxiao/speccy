#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![allow(
    clippy::unwrap_in_result,
    reason = "test code may .expect() with descriptive messages inside TestResult fns"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! End-to-end tests for `speccy implement`.
//!
//! - `prompt_renders_*` exercises SPEC-0008 REQ-004 (CHK-004): template
//!   loading, placeholder substitution, budget trimming, stdout emission.
//! - `error_paths_and_integration_*` exercises REQ-005 (CHK-005): exit codes
//!   for `InvalidFormat` / `NotFound` / `Ambiguous` / outside-workspace, plus
//!   the happy path through the binary.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use predicates::str::contains;
use speccy_cli::implement::ImplementArgs;
use speccy_cli::implement::run;

// -- Helpers -----------------------------------------------------------------

fn write_agents(ws: &Workspace, body: &str) -> TestResult {
    fs_err::write(ws.root.join("AGENTS.md").as_std_path(), body)?;
    Ok(())
}

fn tasks_md_with(spec_id: &str, body: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n{body}",
    )
}

fn capture_stdout(ws: &Workspace, task_ref: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    run(
        &ImplementArgs {
            task_ref: task_ref.to_owned(),
        },
        &ws.root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf)?)
}

// -- CHK-004: prompt_renders ------------------------------------------------

#[test]
fn prompt_renders_succeeds_for_unique_match() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\nuse rust\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: implement signup\n  - Covers: REQ-001\n  - Suggested files: `src/auth/signup.rs`\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001")?;
    assert!(
        out.contains("Speccy: Implement `T-001` for `SPEC-0001`"),
        "output missing header: {out}"
    );
    Ok(())
}

#[test]
fn prompt_renders_substitutes_every_placeholder() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents conventions go here\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: implement signup\n  - Covers: REQ-001\n  - Suggested files: `src/auth/signup.rs`, `tests/auth/signup_spec.rs`\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001")?;
    // {{spec_id}}, {{task_id}}
    assert!(out.contains("`T-001`"), "task_id placeholder missing");
    assert!(out.contains("`SPEC-0001`"), "spec_id placeholder missing");
    // {{spec_md}}
    assert!(out.contains("Example SPEC-0001"), "spec_md content missing");
    assert!(
        out.contains("### REQ-001: First"),
        "spec REQ heading missing"
    );
    // {{task_entry}}
    assert!(
        out.contains("**T-001**: implement signup"),
        "task_entry missing"
    );
    assert!(
        out.contains("Covers: REQ-001"),
        "task sub-list bullets missing"
    );
    // {{suggested_files}}
    assert!(
        out.contains("src/auth/signup.rs, tests/auth/signup_spec.rs"),
        "suggested_files placeholder not formatted as CSV: {out}",
    );
    // {{agents}}
    assert!(
        out.contains("Agents conventions go here"),
        "agents placeholder missing: {out}",
    );
    // No raw placeholder left unsubstituted.
    assert!(
        !out.contains("{{spec_id}}"),
        "spec_id placeholder not substituted"
    );
    assert!(
        !out.contains("{{task_entry}}"),
        "task_entry placeholder not substituted"
    );
    Ok(())
}

#[test]
fn prompt_renders_task_entry_preserves_all_sublist_notes_in_order() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        concat!(
            "- [?] **T-001**: password storage\n",
            "  - Covers: REQ-001\n",
            "  - Suggested files: `src/auth/password.rs`\n",
            "  - Implementer note (session-abc): added bcrypt at cost 10.\n",
            "  - Review (business, pass): matches REQ-001 intent.\n",
            "  - Review (tests, pass): hash assertion present.\n",
            "  - Review (security, blocking): bcrypt cost 10 < required 12.\n",
            "  - Retry: address bcrypt cost.\n",
        ),
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001")?;
    let pos_implementer = out
        .find("Implementer note (session-abc)")
        .expect("implementer note line present");
    let pos_review_biz = out
        .find("Review (business, pass)")
        .expect("business review present");
    let pos_review_security = out
        .find("Review (security, blocking)")
        .expect("security review present");
    let pos_retry = out
        .find("Retry: address bcrypt")
        .expect("retry note present");
    assert!(
        pos_implementer < pos_review_biz
            && pos_review_biz < pos_review_security
            && pos_review_security < pos_retry,
        "task subtree bullets must appear in declared order",
    );
    Ok(())
}

#[test]
fn prompt_renders_with_missing_agents_md_succeeds_with_marker() -> TestResult {
    let ws = Workspace::new()?;
    // Deliberately do NOT write AGENTS.md.
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: implement signup\n  - Covers: REQ-001\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001")?;
    assert!(
        out.contains("AGENTS.md missing"),
        "missing AGENTS.md should leave the marker in the rendered prompt: {out}",
    );
    Ok(())
}

#[test]
fn prompt_renders_qualified_form_resolves_correctly() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks_a = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: in spec-1\n  - Covers: REQ-001\n",
    );
    let tasks_b = tasks_md_with(
        "SPEC-0002",
        "- [ ] **T-001**: in spec-2\n  - Covers: REQ-001\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_a),
    )?;
    write_spec(
        &ws.root,
        "0002-bar",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_b),
    )?;

    let out = capture_stdout(&ws, "SPEC-0002/T-001")?;
    assert!(
        out.contains("in spec-2"),
        "qualified must pick SPEC-0002: {out}"
    );
    assert!(
        !out.contains("in spec-1"),
        "qualified must NOT include SPEC-0001 task: {out}"
    );
    Ok(())
}

// -- CHK-005: error_paths_and_integration -----------------------------------

#[test]
fn error_paths_and_integration_valid_task_exits_zero() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [ ] **T-001**: implement signup\n  - Covers: REQ-001\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("T-001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .success()
        .stdout(contains("Speccy: Implement `T-001`"))
        .stdout(contains("**T-001**: implement signup"));
    Ok(())
}

#[test]
fn error_paths_and_integration_invalid_format_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("FOO")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("invalid task reference `FOO`"))
        .stderr(contains("T-NNN"));
    Ok(())
}

#[test]
fn error_paths_and_integration_not_found_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_with(
            "SPEC-0001",
            "- [ ] **T-001**: real task\n  - Covers: REQ-001\n",
        )),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("T-999")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("not found"))
        .stderr(contains("speccy status"));
    Ok(())
}

#[test]
fn error_paths_and_integration_ambiguous_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    write_spec(
        &ws.root,
        "0001-a",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_with(
            "SPEC-0001",
            "- [ ] **T-001**: in spec-1\n  - Covers: REQ-001\n",
        )),
    )?;
    write_spec(
        &ws.root,
        "0002-b",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_with(
            "SPEC-0002",
            "- [ ] **T-001**: in spec-2\n  - Covers: REQ-001\n",
        )),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("T-001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("T-001 is ambiguous"))
        .stderr(contains("speccy implement SPEC-0001/T-001"))
        .stderr(contains("speccy implement SPEC-0002/T-001"));
    Ok(())
}

#[test]
fn error_paths_and_integration_outside_workspace_exits_one() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("T-001")
        .current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn error_paths_and_integration_missing_positional_exits_two() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement").current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("missing required TASK-ID"));
    Ok(())
}

#[test]
fn error_paths_and_integration_unknown_flag_exits_two() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement")
        .arg("T-001")
        .arg("--bogus")
        .current_dir(ws.root.as_std_path());
    cmd.assert().failure().code(2);
    Ok(())
}

#[test]
fn error_paths_and_integration_help_succeeds() -> TestResult {
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("implement").arg("--help");
    cmd.assert()
        .success()
        .stdout(contains("Usage: speccy"))
        .stdout(contains("implement"))
        .stdout(contains("TASK_REF"));
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0019 / SPEC-0020 T-006: implementer prompt is task-scoped — only
// requirements listed in `Covers:` appear in the rendered slice. Uncovered
// requirement bodies and scenarios are excluded. The emitted markup uses
// raw XML element tags (SPEC-0020 carrier form).
// ---------------------------------------------------------------------------

/// Three-requirement marker SPEC.md with one scenario per requirement and
/// uniquely-identifiable body text so the test can substring-match.
fn marker_three_req_spec_md(spec_id: &str) -> String {
    let template = indoc::indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: in-progress
        created: 2026-05-11
        ---

        # __ID__

        <requirement id="REQ-001">
        ### REQ-001: First
        BODY_REQ_001_unique_marker.
        <scenario id="CHK-001">
        SCENARIO_CHK_001_unique_marker
        </scenario>
        </requirement>
        <requirement id="REQ-002">
        ### REQ-002: Second
        BODY_REQ_002_unique_marker.
        <scenario id="CHK-002">
        SCENARIO_CHK_002_unique_marker
        </scenario>
        </requirement>
        <requirement id="REQ-003">
        ### REQ-003: Third
        BODY_REQ_003_unique_marker.
        <scenario id="CHK-003">
        SCENARIO_CHK_003_unique_marker
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    template.replace("__ID__", spec_id)
}

#[test]
fn prompt_slices_to_covered_requirements_only() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0099",
        "- [ ] **T-001**: only req2\n  - Covers: REQ-002\n  - Suggested files: `a.rs`\n",
    );
    // No legacy spec.toml — relying on the marker tree.
    write_spec(
        &ws.root,
        "0099-slice",
        &marker_three_req_spec_md("SPEC-0099"),
        "",
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001")?;

    // Covered REQ-002: requirement element open tag, body, nested scenario
    // open tag, scenario body, scenario close tag, and requirement close
    // tag all present (raw XML element form per SPEC-0020).
    assert!(
        out.contains("<requirement id=\"REQ-002\">"),
        "covered REQ-002 element open tag must be present in slice:\n{out}",
    );
    assert!(
        out.contains("BODY_REQ_002_unique_marker."),
        "covered REQ-002 body must be present in slice:\n{out}",
    );
    assert!(
        out.contains("<scenario id=\"CHK-002\">"),
        "covered REQ-002 nested scenario open tag must be present:\n{out}",
    );
    assert!(
        out.contains("SCENARIO_CHK_002_unique_marker"),
        "covered REQ-002 scenario body must be present in slice:\n{out}",
    );
    assert!(
        out.contains("</scenario>"),
        "covered REQ-002 nested scenario close tag must be present:\n{out}",
    );
    assert!(
        out.contains("</requirement>"),
        "covered REQ-002 requirement close tag must be present:\n{out}",
    );

    // Uncovered REQ-001 and REQ-003: bodies, scenarios, and open tags all
    // absent.
    assert!(
        !out.contains("BODY_REQ_001_unique_marker."),
        "uncovered REQ-001 body must be excluded:\n{out}",
    );
    assert!(
        !out.contains("BODY_REQ_003_unique_marker."),
        "uncovered REQ-003 body must be excluded:\n{out}",
    );
    assert!(
        !out.contains("<requirement id=\"REQ-001\">"),
        "uncovered REQ-001 element open tag must be excluded:\n{out}",
    );
    assert!(
        !out.contains("<requirement id=\"REQ-003\">"),
        "uncovered REQ-003 element open tag must be excluded:\n{out}",
    );
    assert!(
        !out.contains("<scenario id=\"CHK-001\">"),
        "uncovered REQ-001 scenario open tag must be excluded:\n{out}",
    );
    assert!(
        !out.contains("<scenario id=\"CHK-003\">"),
        "uncovered REQ-003 scenario open tag must be excluded:\n{out}",
    );
    assert!(
        !out.contains("SCENARIO_CHK_001_unique_marker"),
        "uncovered REQ-001 scenario body must be excluded:\n{out}",
    );
    assert!(
        !out.contains("SCENARIO_CHK_003_unique_marker"),
        "uncovered REQ-003 scenario body must be excluded:\n{out}",
    );
    // And no legacy comment-marker carrier leaks into the rendered slice.
    assert!(
        !out.contains("<!-- speccy:"),
        "rendered slice must not emit legacy HTML-comment speccy markers:\n{out}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0020 T-006: end-to-end single-pass substitution regression — bytes
// inside a sliced scenario body that happen to contain `{{agents}}` or
// other handlebars-style placeholders are emitted as literals and NOT
// re-substituted. Pins the SPEC-0019 T-006 single-pass invariant at the
// render boundary, not just the render-helper unit test.
// ---------------------------------------------------------------------------

#[test]
fn prompt_single_pass_does_not_substitute_placeholders_inside_scenario_body() -> TestResult {
    let ws = Workspace::new()?;
    // Use a sentinel agents body so we can detect re-substitution by
    // searching the rendered output for the sentinel inside the scenario
    // body region.
    write_agents(&ws, "AGENTS_SENTINEL_VALUE_FROM_AGENTS_MD\n")?;
    // Scenario body legitimately documents `{{agents}}` and `{{task_id}}`
    // as text — for example, when the spec is teaching prompt-template
    // semantics. The renderer is single-pass: those literals must
    // survive intact in the rendered prompt, even though `{{agents}}`
    // and `{{task_id}}` are also valid template placeholders.
    let spec_md = indoc::indoc! {r#"
        ---
        id: SPEC-0099
        slug: x
        title: Single-pass fixture
        status: in-progress
        created: 2026-05-15
        ---

        # SPEC-0099

        <requirement id="REQ-001">
        ### REQ-001: Documented placeholders survive verbatim
        Body.
        <scenario id="CHK-001">
        Given a scenario body that documents `{{agents}}` and `{{task_id}}`
        as literal placeholder text,
        when the prompt is rendered,
        then these tokens survive verbatim and are not replaced by the
        actual values.
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-15 | t | init |
        </changelog>
    "#};
    let tasks = tasks_md_with(
        "SPEC-0099",
        "- [ ] **T-001**: doc the placeholders\n  - Covers: REQ-001\n",
    );
    write_spec(&ws.root, "0099-single-pass", spec_md, "", Some(&tasks))?;

    let out = capture_stdout(&ws, "T-001")?;
    // Sanity: the agents sentinel did land in the prompt via the
    // top-level `{{agents}}` placeholder substitution.
    assert!(
        out.contains("AGENTS_SENTINEL_VALUE_FROM_AGENTS_MD"),
        "agents placeholder substitution must still work at the top level:\n{out}",
    );
    // The literal `{{agents}}` text inside the scenario body must survive:
    // single-pass substitution does not re-scan substituted text.
    assert!(
        out.contains("`{{agents}}`"),
        "scenario body's literal `{{{{agents}}}}` must NOT be re-substituted:\n{out}",
    );
    assert!(
        out.contains("`{{task_id}}`"),
        "scenario body's literal `{{{{task_id}}}}` must NOT be re-substituted:\n{out}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0020 T-006: when the SpecDoc parse fails (legacy comment marker
// outside any fenced code block), the slicer must fall back to the raw
// SPEC.md bytes so the implementer prompt is never silently empty.
// ---------------------------------------------------------------------------

#[test]
fn prompt_falls_back_to_raw_spec_md_when_parse_fails() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    // Authored SPEC.md still uses a SPEC-0019 HTML-comment marker outside
    // any fenced code block: parse_spec_xml rejects it via the
    // `LegacyMarker` diagnostic, so `location.spec_doc` is `Err` and the
    // slicer-fallback path in `implement.rs` must inline the raw bytes.
    let legacy_spec_md = indoc::indoc! {r#"
        ---
        id: SPEC-0099
        slug: x
        title: Legacy fallback fixture
        status: in-progress
        created: 2026-05-15
        ---

        # SPEC-0099

        <!-- speccy:requirement id="REQ-001" -->
        ### REQ-001: First
        FALLBACK_REQ_001_unique_marker
        <!-- /speccy:requirement -->
    "#};
    let tasks = tasks_md_with(
        "SPEC-0099",
        "- [ ] **T-001**: covers req1\n  - Covers: REQ-001\n",
    );
    write_spec(&ws.root, "0099-fallback", legacy_spec_md, "", Some(&tasks))?;

    let out = capture_stdout(&ws, "T-001")?;
    // The prompt must not be silently empty: the raw SPEC.md bytes are
    // the documented fallback.
    assert!(
        out.contains("FALLBACK_REQ_001_unique_marker"),
        "fallback path must inline the raw SPEC.md body when parse fails:\n{out}",
    );
    assert!(
        out.contains("# SPEC-0099"),
        "fallback path must inline the SPEC.md heading verbatim:\n{out}",
    );
    Ok(())
}
