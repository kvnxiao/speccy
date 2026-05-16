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
//! - CHK-005 (`diff_fallback_chain`): exercised separately in `git_diff.rs` to
//!   keep this binary fast; this file verifies only that the literal `{{diff}}`
//!   placeholder is substituted.
//! - CHK-006 (`prompt_renders`): template loaded, every placeholder
//!   substituted, budget trimming applied, output to stdout.
//! - CHK-007 (`shared_task_lookup_and_integration`): reuses
//!   `task_lookup::find`; ambiguity stderr suggests the `speccy review
//!   SPEC-NNNN/T-NNN --persona <name>` form.

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
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n{body}",
    )
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
    // {{spec_md}}
    assert!(out.contains("Example SPEC-0001"), "spec_md missing");
    assert!(out.contains("### REQ-001: First"), "REQ heading missing");
    // {{task_entry}}
    assert!(
        out.contains("**T-001**: implement signup"),
        "task_entry missing"
    );
    assert!(
        out.contains("Implementer note: done."),
        "task subtree bullets missing"
    );
    // {{agents}}
    assert!(
        out.contains("Agents conventions go here"),
        "agents missing: {out}",
    );
    // {{diff}} — outside a git repo this is the documented fallback
    // string; the test ensures the placeholder was substituted (not
    // left literal).
    assert!(
        !out.contains("{{diff}}"),
        "diff placeholder not substituted"
    );
    // {{persona_content}} — embedded fallback stub content lands here.
    assert!(
        !out.contains("{{persona_content}}"),
        "persona_content placeholder not substituted",
    );
    // No raw placeholders left unsubstituted.
    for raw in [
        "{{task_id}}",
        "{{spec_id}}",
        "{{spec_md}}",
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

// -- CHK-005: diff_fallback_chain placeholder substitution -----------------

#[test]
fn diff_placeholder_is_substituted_with_fallback_outside_repo() -> TestResult {
    let ws = Workspace::new()?;
    seed_one_task(&ws)?;
    let out = capture_stdout(&ws, "T-001", "security")?;
    // Outside a git repo, the documented fallback is inlined as the
    // diff content; the placeholder must not be left literal.
    assert!(
        !out.contains("{{diff}}"),
        "diff placeholder must be substituted"
    );
    assert!(
        out.contains("no diff available"),
        "outside-repo diff must use the fallback note: {out}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0019 T-006: reviewer prompt is task-scoped — the scenario text the
// reviewer sees equals the marker body bytes from SPEC.md.
// ---------------------------------------------------------------------------

#[test]
fn reviewer_tests_scenario_text_equals_marker_body_bytes() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    let containing_spec = "SPEC-0099";
    let spec_md = indoc::indoc! {r#"
        ---
        id: SPEC-0099
        slug: x
        title: Example SPEC-0099
        status: in-progress
        created: 2026-05-11
        ---

        # SPEC-0099

        <requirement id="REQ-001">
        ### REQ-001: First
        unrelated body
        <scenario id="CHK-001">
        unrelated scenario
        </scenario>
        </requirement>
        <requirement id="REQ-002">
        ### REQ-002: Second
        body for REQ-002
        <scenario id="CHK-002">
        Given REQ-002,
        when the reviewer reads the prompt,
        then the scenario body bytes equal this text.
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
        containing_spec,
        "- [?] **T-001**: only req2\n  - Covers: REQ-002\n  - Implementer note: done.\n",
    );
    write_spec(&ws.root, "0099-review-slice", spec_md, "", Some(&tasks))?;

    let out = capture_stdout(&ws, "T-001", "tests")?;

    // Extract the CHK-002 scenario body bytes from the source.
    let start_tag = "<scenario id=\"CHK-002\">\n";
    let end_tag = "</scenario>";
    let after_start = spec_md
        .find(start_tag)
        .map(|i| i + start_tag.len())
        .expect("fixture must contain CHK-002 open tag");
    let tail = spec_md
        .get(after_start..)
        .expect("after_start must be a valid char boundary in fixture");
    let before_end = tail
        .find(end_tag)
        .map(|j| after_start + j)
        .expect("fixture must contain matching close tag");
    let body_bytes = spec_md
        .get(after_start..before_end)
        .expect("body slice must lie on valid char boundaries in fixture")
        .trim_end_matches('\n');

    // The full scenario body (multi-line) must appear in the rendered
    // prompt as a contiguous substring — that's how the reviewer sees
    // the validation scenario.
    assert!(
        out.contains(body_bytes),
        "reviewer prompt must include the CHK-002 scenario body bytes verbatim;\n\
         body_bytes={body_bytes:?}\n\
         out={out}",
    );

    // And the unrelated REQ-001 marker body must NOT appear (task-scoped
    // slicing excludes uncovered requirements).
    assert!(
        !out.contains("unrelated scenario"),
        "uncovered REQ-001's scenario body must be excluded from slice:\n{out}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0020 T-006: when the scenario body spans multiple Markdown
// paragraphs and includes a fenced code block plus literal `<` / `>`
// characters, the reviewer prompt must surface the full body bytes
// verbatim and contiguous with the source.
// ---------------------------------------------------------------------------

#[test]
fn reviewer_tests_multi_paragraph_scenario_body_renders_verbatim() -> TestResult {
    let ws = Workspace::new()?;
    write_agents(&ws, "# Agents\n")?;
    // Multi-paragraph scenario body. Contains:
    // - A fenced code block (with literal `<thinking>` inside, which the line-aware
    //   scanner must NOT promote to structure).
    // - Literal `<` / `>` Markdown characters in prose.
    // - Multiple paragraphs separated by blank lines.
    // These are exactly the inputs SPEC-0020 DEC-003 calls out as
    // requiring byte-verbatim preservation.
    let spec_md = indoc::indoc! {r#"
        ---
        id: SPEC-0099
        slug: x
        title: Reviewer multi-paragraph fixture
        status: in-progress
        created: 2026-05-15
        ---

        # SPEC-0099

        <requirement id="REQ-002">
        ### REQ-002: Reviewer surfaces multi-paragraph bodies verbatim
        body for REQ-002.
        <scenario id="CHK-002">
        Given a scenario whose body contains literal `<thinking>` and
        comparison expressions like `a < b > c`,

        when the reviewer reads the rendered prompt,

        then the body bytes must round-trip verbatim:

        ```rust
        fn assert_lt<T: Ord>(a: T, b: T) {
            assert!(a < b, "<expected> a < b");
        }
        ```

        And the trailing prose paragraph too.
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
        "- [?] **T-001**: only req2\n  - Covers: REQ-002\n  - Implementer note: done.\n",
    );
    write_spec(&ws.root, "0099-reviewer-multi", spec_md, "", Some(&tasks))?;

    let out = capture_stdout(&ws, "T-001", "tests")?;

    // Extract the CHK-002 scenario body bytes from the source verbatim.
    let start_tag = "<scenario id=\"CHK-002\">\n";
    let end_tag = "</scenario>";
    let after_start = spec_md
        .find(start_tag)
        .map(|i| i + start_tag.len())
        .expect("fixture must contain CHK-002 open tag");
    let tail = spec_md
        .get(after_start..)
        .expect("after_start must be a valid char boundary in fixture");
    let before_end = tail
        .find(end_tag)
        .map(|j| after_start + j)
        .expect("fixture must contain matching close tag");
    let body_bytes = spec_md
        .get(after_start..before_end)
        .expect("body slice must lie on valid char boundaries in fixture")
        .trim_end_matches('\n');

    // Sanity: the fixture really does contain the load-bearing pieces.
    assert!(body_bytes.contains("`<thinking>`"));
    assert!(body_bytes.contains("```rust"));
    assert!(body_bytes.contains("a < b > c"));

    // The full multi-paragraph scenario body must appear in the rendered
    // prompt as a contiguous substring: byte-for-byte preservation
    // across the fenced code block, the literal `<`/`>` characters, and
    // the paragraph breaks.
    assert!(
        out.contains(body_bytes),
        "reviewer prompt must include the CHK-002 multi-paragraph body bytes verbatim;\n\
         body_bytes={body_bytes:?}\n\
         out={out}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// SPEC-0020 T-006: when the SpecDoc parse fails (legacy comment marker
// outside any fenced code block), the reviewer slicer must fall back to
// the raw SPEC.md bytes so the reviewer prompt is never silently empty.
// ---------------------------------------------------------------------------

#[test]
fn reviewer_prompt_falls_back_to_raw_spec_md_when_parse_fails() -> TestResult {
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
        REVIEWER_FALLBACK_REQ_001_unique_marker
        <!-- /speccy:requirement -->
    "#};
    let tasks = tasks_md_with(
        "SPEC-0099",
        "- [?] **T-001**: covers req1\n  - Covers: REQ-001\n  - Implementer note: done.\n",
    );
    write_spec(
        &ws.root,
        "0099-rev-fallback",
        legacy_spec_md,
        "",
        Some(&tasks),
    )?;

    let out = capture_stdout(&ws, "T-001", "tests")?;
    assert!(
        out.contains("REVIEWER_FALLBACK_REQ_001_unique_marker"),
        "reviewer fallback path must inline the raw SPEC.md body when parse fails:\n{out}",
    );
    Ok(())
}
