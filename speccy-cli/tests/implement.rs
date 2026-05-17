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
    let body = convert_legacy_to_xml(spec_id, body);
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n{body}",
    )
}

/// Translate a legacy bullet-checkbox TASKS.md body into the XML grammar.
/// Used by test fixtures that pre-date SPEC-0022's loader switch.
#[expect(
    clippy::format_push_string,
    reason = "narrow test-only legacy-to-XML transform; flattening hurts readability"
)]
fn convert_legacy_to_xml(spec_id: &str, body: &str) -> String {
    let mut out = format!("<tasks spec=\"{spec_id}\">\n\n");
    let mut current: Option<(String, String, String, Vec<String>)> = None;
    let push = |out: &mut String, cur: (String, String, String, Vec<String>)| {
        let (id, state, title, notes) = cur;
        let covers = notes
            .iter()
            .find_map(|n| n.strip_prefix("Covers:").map(|c| c.trim().to_owned()))
            .unwrap_or_else(|| "REQ-001".to_owned());
        out.push_str(&format!(
            "<task id=\"{id}\" state=\"{state}\" covers=\"{covers}\">\n{title}\n"
        ));
        for note in &notes {
            out.push_str("- ");
            out.push_str(note);
            out.push('\n');
        }
        out.push_str("\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n");
    };
    for line in body.lines() {
        let trimmed_start = line.trim_start();
        if let Some(rest) = trimmed_start.strip_prefix("- [")
            && let Some((glyph, after)) = rest.split_once("] ")
            && let Some(after) = after.strip_prefix("**")
            && let Some((id, title)) = after.split_once("**")
        {
            let title = title.trim_start_matches(':').trim().to_owned();
            let state = match glyph {
                "~" => "in-progress",
                "?" => "in-review",
                "x" => "completed",
                _ => "pending",
            }
            .to_owned();
            if let Some(cur) = current.take() {
                push(&mut out, cur);
            }
            current = Some((id.to_owned(), state, title, Vec::new()));
            continue;
        }
        if let Some(rest) = trimmed_start.strip_prefix("- ")
            && let Some(ref mut cur) = current
        {
            cur.3.push(rest.to_owned());
            continue;
        }
        if current.is_none() && !line.is_empty() {
            out.push_str(line);
            out.push('\n');
        }
    }
    if let Some(cur) = current.take() {
        push(&mut out, cur);
    }
    out.push_str("</tasks>\n");
    out
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
    // SPEC-0023 REQ-005: AGENTS.md is no longer inlined into the rendered
    // prompt; modern AI coding harnesses auto-load it themselves. Writing
    // AGENTS.md here would only confirm the renderer ignores it; the
    // negative assertion below pins that explicitly.
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
    // SPEC-0023 REQ-006: `{{spec_md}}` is retired. The rendered prompt
    // names the SPEC.md repo-relative path; the body is not inlined.
    assert!(
        out.contains(".speccy/specs/0001-foo/SPEC.md"),
        "rendered prompt must name the SPEC.md repo-relative path: {out}",
    );
    assert!(
        !out.contains("Example SPEC-0001"),
        "SPEC.md body must not be inlined into the rendered prompt: {out}",
    );
    assert!(
        !out.contains("### REQ-001: First"),
        "SPEC.md REQ heading must not appear inline: {out}",
    );
    // {{task_entry}}
    assert!(
        out.contains("<task id=\"T-001\""),
        "task_entry missing the <task> element"
    );
    assert!(
        out.contains("covers=\"REQ-001\""),
        "task covers attribute missing"
    );
    // {{suggested_files}}
    assert!(
        out.contains("src/auth/signup.rs, tests/auth/signup_spec.rs"),
        "suggested_files placeholder not formatted as CSV: {out}",
    );
    // SPEC-0023 REQ-005: `{{agents}}` is retired. The AGENTS.md body must
    // not appear in the rendered prompt; the host auto-loads it.
    assert!(
        !out.contains("Agents conventions go here"),
        "AGENTS.md body must not be inlined into the rendered prompt: {out}",
    );
    assert!(
        !out.contains("{{agents}}"),
        "retired `{{{{agents}}}}` placeholder must not appear in rendered output: {out}",
    );
    // No raw placeholder left unsubstituted (including the retired
    // `{{spec_md}}`).
    for raw in [
        "{{spec_id}}",
        "{{spec_md}}",
        "{{spec_md_path}}",
        "{{task_id}}",
        "{{task_entry}}",
        "{{suggested_files}}",
    ] {
        assert!(
            !out.contains(raw),
            "placeholder `{raw}` not substituted: {out}"
        );
    }
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
        .stdout(contains("<task id=\"T-001\""));
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
        BODY_REQ_001_unique_marker.
        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        SCENARIO_CHK_001_unique_marker
        </scenario>
        </requirement>
        <requirement id="REQ-002">
        ### REQ-002: Second
        BODY_REQ_002_unique_marker.
        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-002">
        SCENARIO_CHK_002_unique_marker
        </scenario>
        </requirement>
        <requirement id="REQ-003">
        ### REQ-003: Third
        BODY_REQ_003_unique_marker.
        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

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
fn prompt_does_not_inline_spec_md_body_for_any_requirement() -> TestResult {
    // SPEC-0023 REQ-006: the rendered implementer prompt no longer
    // inlines the SPEC.md body (neither sliced nor full). The retired
    // SPEC-0019/SPEC-0020 slicer test ensured covered REQ bodies were
    // emitted; after T-006 the rendered prompt names the SPEC.md
    // repo-relative path instead, and the agent reads the file via the
    // host's Read primitive on demand.
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks = tasks_md_with(
        "SPEC-0099",
        "- [ ] **T-001**: only req2\n  - Covers: REQ-002\n  - Suggested files: `a.rs`\n",
    );
    write_spec(
        &ws.root,
        "0099-slice",
        &marker_three_req_spec_md("SPEC-0099"),
        "",
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001")?;

    assert!(
        out.contains(".speccy/specs/0099-slice/SPEC.md"),
        "rendered prompt must name the SPEC.md repo-relative path: {out}",
    );

    // No SPEC body bytes (covered or uncovered) must appear in the
    // rendered prompt after SPEC-0023 REQ-006.
    for marker in [
        "BODY_REQ_001_unique_marker.",
        "BODY_REQ_002_unique_marker.",
        "BODY_REQ_003_unique_marker.",
        "SCENARIO_CHK_001_unique_marker",
        "SCENARIO_CHK_002_unique_marker",
        "SCENARIO_CHK_003_unique_marker",
        "<requirement id=\"REQ-001\">",
        "<requirement id=\"REQ-002\">",
        "<requirement id=\"REQ-003\">",
        "<scenario id=\"CHK-001\">",
        "<scenario id=\"CHK-002\">",
        "<scenario id=\"CHK-003\">",
    ] {
        assert!(
            !out.contains(marker),
            "SPEC.md body marker `{marker}` must not be inlined into the rendered prompt: {out}",
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0020 T-006: end-to-end single-pass substitution regression — bytes
// inside a sliced scenario body that happen to contain handlebars-style
// placeholders are emitted as literals and NOT re-substituted. Pins the
// SPEC-0019 T-006 single-pass invariant at the render boundary, not just
// the render-helper unit test.
//
// SPEC-0023 REQ-005 retired the `{{agents}}` placeholder; the test still
// uses it in the scenario body to document the spirit of the invariant —
// a placeholder string that *would have been* substituted at the top
// level must survive verbatim inside scenario-body bytes. `{{task_id}}`
// stays the load-bearing live-substitution assertion.
// ---------------------------------------------------------------------------

#[test]
fn prompt_single_pass_substitution_invariant_at_top_level_after_spec_body_retirement() -> TestResult
{
    // SPEC-0023 REQ-006 retired SPEC.md inlining, so the original
    // SPEC-0019/SPEC-0020 single-pass invariant (scenario body bytes
    // survive verbatim because the slicer emits them, and the renderer
    // is single-pass so a literal `{{task_id}}` inside that scenario is
    // not re-substituted) no longer has a SPEC body to pin. The
    // single-pass invariant itself is still load-bearing — confirm at
    // the top level that `{{task_id}}` substitutes once and only once,
    // and that the literal `{{agents}}` placeholder bytes inserted
    // through a stable substituted variable do not get rescanned.
    let ws = Workspace::new()?;
    let spec_md = indoc::indoc! {r#"
        ---
        id: SPEC-0099
        slug: x
        title: Single-pass fixture
        status: in-progress
        created: 2026-05-15
        ---

        # SPEC-0099

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
        ### REQ-001: Documented placeholders survive verbatim
        Body.
        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        Single-pass invariant.
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
    // Sanity: `{{task_id}}` did get substituted at the top level
    // (e.g. in the rendered header).
    assert!(
        out.contains("`T-001`"),
        "task_id placeholder substitution must still work at the top level:\n{out}",
    );
    // SPEC-0023 REQ-006: the SPEC.md body is no longer inlined, so the
    // scenario body bytes the old test pinned are gone. The retired
    // placeholders themselves must not leak into the rendered prompt.
    for retired in ["{{agents}}", "{{spec_md}}", "{{tasks_md}}", "{{mission}}"] {
        assert!(
            !out.contains(retired),
            "retired placeholder `{retired}` must not appear in rendered prompt: {out}",
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0023 REQ-006: the implementer prompt no longer inlines SPEC.md (the
// slicer-fallback path is gone). The retired SPEC-0020 test pinned the
// raw-SPEC.md fallback when the SpecDoc parse failed. After T-006 the
// rendered prompt names the SPEC.md repo-relative path; the agent reads
// the file via the host's Read primitive on demand.
// ---------------------------------------------------------------------------

#[test]
fn prompt_does_not_inline_spec_md_when_parse_fails() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
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
    // SPEC-0023 REQ-006: SPEC.md body must not be inlined. The retired
    // raw-fallback bytes must not appear. The path is named instead so
    // the agent can read it itself.
    assert!(
        out.contains(".speccy/specs/0099-fallback/SPEC.md"),
        "rendered prompt must name the SPEC.md repo-relative path: {out}",
    );
    assert!(
        !out.contains("FALLBACK_REQ_001_unique_marker"),
        "raw SPEC.md body bytes must not be inlined into the rendered prompt: {out}",
    );
    Ok(())
}
