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
//! End-to-end tests for `speccy review`.
//!
//! Covers SPEC-0009:
//! - CHK-004 (`persona_arg_validation`): `--persona` required, validated
//!   against the six-entry registry, case-sensitive.
//! - CHK-006 (`prompt_renders`): template loaded, every placeholder
//!   substituted, budget trimming applied, output to stdout.
//! - CHK-007 (`shared_task_lookup_and_integration`): reuses
//!   `task_lookup::find`; ambiguity stderr suggests the `speccy review
//!   SPEC-NNNN/T-NNN --persona <name>` form.
//!
//! SPEC-0023 REQ-003 retired the inlined-diff path; the rendered prompt
//! no longer contains a `{{diff}}` placeholder, and the reviewer agent
//! fetches the diff via `git diff` itself.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use predicates::str::contains;
use speccy_cli::review::ReviewArgs;
use speccy_cli::review::run;

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

fn capture_stdout(
    ws: &Workspace,
    task_ref: &str,
    persona: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    run(
        &ReviewArgs {
            task_ref: task_ref.to_owned(),
            persona: persona.to_owned(),
        },
        &ws.root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf)?)
}

fn seed_one_task(ws: &Workspace) -> TestResult {
    write_agents(ws, "# Agents conventions go here\n")?;
    let tasks = tasks_md_with(
        "SPEC-0001",
        "- [?] **T-001**: implement signup\n  - Covers: REQ-001\n  - Implementer note: done.\n",
    );
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks),
    )?;
    Ok(())
}

// -- CHK-006: prompt_renders ------------------------------------------------

#[test]
fn prompt_renders_substitutes_every_placeholder() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;

    let out = capture_stdout(&ws, "T-001", "security")?;
    // {{task_id}}, {{spec_id}}, {{persona}}
    assert!(
        out.contains("`T-001`"),
        "task_id placeholder missing: {out}"
    );
    assert!(out.contains("`SPEC-0001`"), "spec_id placeholder missing");
    assert!(
        out.contains("(security)"),
        "persona placeholder missing: {out}",
    );
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
        out.contains("Implementer note: done."),
        "task subtree bullets missing"
    );
    // SPEC-0023 REQ-005: `{{agents}}` is retired. The AGENTS.md body
    // must not appear in the rendered prompt; the host auto-loads it.
    assert!(
        !out.contains("Agents conventions go here"),
        "AGENTS.md body must not be inlined into the rendered prompt: {out}",
    );
    // SPEC-0023 REQ-003: `{{diff}}` is gone — the rendered prompt
    // instructs the reviewer agent to run `git diff` itself.
    assert!(
        !out.contains("{{diff}}"),
        "diff placeholder must not appear in rendered prompt: {out}",
    );
    assert!(
        out.contains("git diff"),
        "rendered prompt must instruct the agent to run `git diff`: {out}",
    );
    // {{persona_content}} — embedded fallback stub content lands here.
    assert!(
        !out.contains("{{persona_content}}"),
        "persona_content placeholder not substituted",
    );
    // No raw placeholders left unsubstituted (including the retired
    // `{{agents}}` and `{{spec_md}}`).
    for raw in [
        "{{task_id}}",
        "{{spec_id}}",
        "{{spec_md}}",
        "{{spec_md_path}}",
        "{{task_entry}}",
        "{{agents}}",
        "{{persona}}",
    ] {
        assert!(!out.contains(raw), "placeholder `{raw}` not substituted");
    }
    Ok(())
}

#[test]
fn prompt_renders_picks_up_project_local_persona_override() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;
    // Write a project-local persona override.
    let dir = ws.root.join(".speccy").join("skills").join("personas");
    fs_err::create_dir_all(dir.as_std_path())?;
    let body = "# Custom security persona\n\nFlag bcrypt cost < 12.\n";
    fs_err::write(dir.join("reviewer-security.md").as_std_path(), body)?;

    let out = capture_stdout(&ws, "T-001", "security")?;
    assert!(
        out.contains("Custom security persona"),
        "project-local persona override should be inlined: {out}",
    );
    Ok(())
}

#[test]
fn prompt_renders_each_default_fan_out_persona() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;

    for persona in ["business", "tests", "security", "style"] {
        let out = capture_stdout(&ws, "T-001", persona)?;
        assert!(
            out.contains(&format!("({persona})")),
            "persona `{persona}` did not render its name: {out}",
        );
    }
    Ok(())
}

// -- CHK-004: persona_arg_validation ----------------------------------------

#[test]
fn persona_arg_validation_missing_persona_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("review")
        .arg("T-001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("missing required --persona"))
        .stderr(contains("business"))
        .stderr(contains("tests"))
        .stderr(contains("security"))
        .stderr(contains("style"))
        .stderr(contains("architecture"))
        .stderr(contains("docs"));
    Ok(())
}

#[test]
fn persona_arg_validation_unknown_persona_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("review")
        .arg("T-001")
        .arg("--persona")
        .arg("unknown")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("unknown persona `unknown`"))
        .stderr(contains(
            "business, tests, security, style, architecture, docs",
        ));
    Ok(())
}

#[test]
fn persona_arg_validation_is_case_sensitive() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("review")
        .arg("T-001")
        .arg("--persona")
        .arg("Security")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("unknown persona `Security`"));
    Ok(())
}

#[test]
fn persona_arg_validation_valid_name_succeeds() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("review")
        .arg("T-001")
        .arg("--persona")
        .arg("security")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();
    Ok(())
}

// -- CHK-007: shared_task_lookup_and_integration ---------------------------

#[test]
fn shared_task_lookup_invalid_format_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("review")
        .arg("FOO")
        .arg("--persona")
        .arg("security")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("invalid task reference `FOO`"));
    Ok(())
}

#[test]
fn shared_task_lookup_not_found_exits_one() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("review")
        .arg("T-999")
        .arg("--persona")
        .arg("security")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("not found"))
        .stderr(contains("speccy status"));
    Ok(())
}

#[test]
fn shared_task_lookup_ambiguous_suggests_review_form() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    write_spec(
        &ws.root,
        "0001-a",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_with(
            "SPEC-0001",
            "- [?] **T-001**: in spec-1\n  - Covers: REQ-001\n",
        )),
    )?;
    write_spec(
        &ws.root,
        "0002-b",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_with(
            "SPEC-0002",
            "- [?] **T-001**: in spec-2\n  - Covers: REQ-001\n",
        )),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("review")
        .arg("T-001")
        .arg("--persona")
        .arg("security")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("T-001 is ambiguous"))
        .stderr(contains("speccy review SPEC-0001/T-001 --persona <name>"))
        .stderr(contains("speccy review SPEC-0002/T-001 --persona <name>"));
    Ok(())
}

#[test]
fn shared_task_lookup_qualified_form_resolves() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let tasks_a = tasks_md_with(
        "SPEC-0001",
        "- [?] **T-001**: in spec-1\n  - Covers: REQ-001\n",
    );
    let tasks_b = tasks_md_with(
        "SPEC-0002",
        "- [?] **T-001**: in spec-2\n  - Covers: REQ-001\n",
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

    let out = capture_stdout(&ws, "SPEC-0002/T-001", "security")?;
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

#[test]
fn shared_task_lookup_outside_workspace_exits_one() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("review")
        .arg("T-001")
        .arg("--persona")
        .arg("security")
        .current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn shared_task_lookup_missing_task_id_exits_two() -> TestResult {
    let ws = Workspace::new()?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("review")
        .arg("--persona")
        .arg("security")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("missing required TASK-ID"));
    Ok(())
}

#[test]
fn help_succeeds() -> TestResult {
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("review").arg("--help");
    cmd.assert()
        .success()
        .stdout(contains("Usage: speccy"))
        .stdout(contains("review"))
        .stdout(contains("--persona"))
        .stdout(contains(
            "business, tests, security, style, architecture, docs",
        ));
    Ok(())
}

// -- SPEC-0023 REQ-003: rendered prompt no longer inlines the diff ---------

#[test]
fn rendered_prompt_omits_inline_diff_and_instructs_git_fetch() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;
    let out = capture_stdout(&ws, "T-001", "security")?;
    // SPEC-0023 REQ-003: no literal placeholder, no fallback note, and
    // no `diff --git` line — the prompt now tells the reviewer agent
    // to run `git diff` itself.
    assert!(
        !out.contains("{{diff}}"),
        "diff placeholder must not appear in rendered prompt: {out}",
    );
    assert!(
        !out.contains("no diff available"),
        "rendered prompt must not contain the retired fallback note: {out}",
    );
    assert!(
        !out.lines().any(|line| line.starts_with("diff --git")),
        "rendered prompt must not contain an inlined `diff --git` line: {out}",
    );
    assert!(
        out.contains("git diff"),
        "rendered prompt must instruct the agent to run `git diff`: {out}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0023 REQ-006: the reviewer prompt no longer inlines the SPEC.md body
// — neither sliced nor verbatim. The retired SPEC-0019/SPEC-0020 tests pinned
// the slicer contract (`reviewer_tests_scenario_text_equals_marker_body_bytes`,
// `reviewer_tests_multi_paragraph_scenario_body_renders_verbatim`,
// `reviewer_prompt_falls_back_to_raw_spec_md_when_parse_fails`). After
// T-006 the rendered prompt names the SPEC.md repo-relative path; the
// agent reads the file via the host's Read primitive on demand.
// ---------------------------------------------------------------------------

#[test]
fn reviewer_prompt_does_not_inline_spec_body_for_grouped_spec() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let spec_md = indoc::indoc! {r#"
        ---
        id: SPEC-0099
        slug: x
        title: Example SPEC-0099
        status: in-progress
        created: 2026-05-11
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

        <requirement id="REQ-002">
        ### REQ-002: Second
        REVIEWER_NO_INLINE_REQ_002_unique_marker
        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-002">
        Given REQ-002,
        when the reviewer reads the prompt,
        then the scenario body bytes are not inlined.
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    let tasks = tasks_md_with(
        "SPEC-0099",
        "- [?] **T-001**: only req2\n  - Covers: REQ-002\n  - Implementer note: done.\n",
    );
    write_spec(&ws.root, "0099-review-slice", spec_md, "", Some(&tasks))?;

    let out = capture_stdout(&ws, "T-001", "tests")?;
    // SPEC-0023 REQ-006: the SPEC.md body must not appear in the
    // rendered prompt — neither the requirement body nor the scenario
    // body. The path is named instead.
    assert!(
        out.contains(".speccy/specs/0099-review-slice/SPEC.md"),
        "rendered prompt must name the SPEC.md repo-relative path: {out}",
    );
    assert!(
        !out.contains("REVIEWER_NO_INLINE_REQ_002_unique_marker"),
        "SPEC.md body must not be inlined into the rendered prompt: {out}",
    );
    assert!(
        !out.contains("then the scenario body bytes are not inlined"),
        "scenario body must not be inlined into the rendered prompt: {out}",
    );
    Ok(())
}
