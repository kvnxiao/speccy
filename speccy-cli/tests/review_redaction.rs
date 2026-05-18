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

//! SPEC-0029 T-004 integration tests for the `speccy review`
//! `{{task_entry}}` redaction switch.
//!
//! Asserts CHK-003 (redaction shape, uniformity across personas, no-op
//! when no `<implementer-note>` exists, no placeholder marker) and the
//! CHK-004 cross-checks (`speccy implement` is NOT redacted).

mod common;

use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use indoc::indoc;
use speccy_cli::implement::ImplementArgs;
use speccy_cli::review::ReviewArgs;
use speccy_core::personas::ALL as PERSONAS_ALL;

// -- Helpers ----------------------------------------------------------------

fn write_agents(ws: &Workspace, body: &str) -> TestResult {
    fs_err::write(ws.root.join("AGENTS.md").as_std_path(), body)?;
    Ok(())
}

fn tasks_md_frontmatter(spec_id: &str, body: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n{body}",
    )
}

/// TASKS.md body containing one `<task>` carrying every body-item kind:
/// free prose, `<task-scenarios>`, two `<implementer-note>` elements
/// (initial + retry-session), two `<review>` elements (blocking + pass),
/// one `<retry>`, and a `Suggested files:` planner bullet.
fn tasks_md_full_mixed_body(spec_id: &str) -> String {
    let body = indoc! {r#"
        <tasks spec="__SPEC_ID__">

        <task id="T-001" state="in-review" covers="REQ-001">
        ## T-001: implement signup

        Free prose ahead of any element block.

        <task-scenarios>
        Given a signup form, when submitted with a valid email,
        then a user record is created.
        </task-scenarios>

        <implementer-note session="session-2026-05-18-spec0001-t001-initial">
        - Completed: shipped the signup handler and a happy-path test.
        - Undone: (none)
        - Commands run: cargo test --workspace
        - Exit codes: 0
        - Discovered issues: (none)
        - Procedural compliance: (none)
        </implementer-note>

        <review persona="business" verdict="blocking">
        Scope drifted into password reset which was a non-goal.
        </review>

        <retry>
        Narrow the change to the signup happy path; defer password reset to a
        separate SPEC.
        </retry>

        <implementer-note session="session-2026-05-18-spec0001-t001-retry">
        - Completed: scope narrowed; password reset reverted.
        - Undone: (none)
        - Commands run: cargo test --workspace
        - Exit codes: 0
        - Discovered issues: (none)
        - Procedural compliance: (none)
        </implementer-note>

        <review persona="business" verdict="pass">
        Scope restored; matches the SPEC intent.
        </review>

        - Suggested files: `src/auth.rs`, `tests/signup.rs`
        </task>

        </tasks>
    "#};
    tasks_md_frontmatter(spec_id, &body.replace("__SPEC_ID__", spec_id))
}

/// TASKS.md body containing one `<task>` with no `<implementer-note>`
/// element at all — the no-op-redaction case.
fn tasks_md_no_implementer_note(spec_id: &str) -> String {
    let body = indoc! {r#"
        <tasks spec="__SPEC_ID__">

        <task id="T-001" state="in-review" covers="REQ-001">
        ## T-001: plain task body

        Free prose with no implementer notes yet.

        <task-scenarios>
        Given the parser, when there is nothing to redact, then the
        helper is a no-op.
        </task-scenarios>

        - Suggested files: `src/parser.rs`
        </task>

        </tasks>
    "#};
    tasks_md_frontmatter(spec_id, &body.replace("__SPEC_ID__", spec_id))
}

fn capture_review_stdout(ws: &Workspace, task_ref: &str, persona: &str) -> TestResult<String> {
    let mut buf: Vec<u8> = Vec::new();
    speccy_cli::review::run(
        &ReviewArgs {
            task_ref: task_ref.to_owned(),
            persona: persona.to_owned(),
        },
        &ws.root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf)?)
}

fn capture_implement_stdout(ws: &Workspace, task_ref: &str) -> TestResult<String> {
    let mut buf: Vec<u8> = Vec::new();
    speccy_cli::implement::run(
        &ImplementArgs {
            task_ref: task_ref.to_owned(),
        },
        &ws.root,
        &mut buf,
    )?;
    Ok(String::from_utf8(buf)?)
}

/// Slice the substituted `{{task_entry}}` value out of a rendered
/// prompt — the substring from the rendered `<task id="..." state=`
/// open tag through the matching `</task>` close tag (inclusive).
///
/// Anchored on `state=` to skip the descriptive `<task
/// id="{{task_id}}">...</task>` literal that the prompt prose mentions (which
/// has no state attribute); cannot key off the `## Task entry (verbatim from
/// TASKS.md)` heading because the task body itself frequently contains `##
/// T-NNN:` headings that would terminate a naive section slice early.
fn task_entry_section(rendered: &str) -> &str {
    // Find the actual task subtree open tag, distinguished from the
    // descriptive literal `<task id="T-001">...</task>` in prompt prose
    // by the required `state="..."` attribute on rendered TASKS.md tasks.
    let mut search_from = 0usize;
    let open_tag_start = loop {
        let suffix = rendered
            .get(search_from..)
            .expect("search_from must be a valid byte offset into rendered");
        let rel = suffix
            .find("<task id=")
            .expect("rendered prompt must contain a `<task id=` open tag with state attr");
        let abs = search_from + rel;
        // Peek at the rest of the line for a `state="..."` attribute. The
        // descriptive prose literal closes with `>` immediately; rendered
        // tasks have `state="..."` before `>`.
        let tail = rendered
            .get(abs..)
            .expect("abs must be a valid byte offset into rendered");
        let line_end = tail.find('>').map_or(rendered.len(), |i| abs + i);
        let head = rendered
            .get(abs..line_end)
            .expect("abs..line_end must be a valid range");
        if head.contains("state=") {
            break abs;
        }
        search_from = line_end;
    };
    let close = "</task>";
    let from_open = rendered
        .get(open_tag_start..)
        .expect("open_tag_start must be a valid byte offset");
    let end = from_open
        .find(close)
        .map(|i| open_tag_start + i + close.len())
        .expect("rendered prompt must contain a matching `</task>` close tag");
    rendered
        .get(open_tag_start..end)
        .expect("open_tag_start..end must be a valid range")
}

fn seed_full_mixed_workspace(ws: &Workspace) -> TestResult {
    write_agents(ws, "# Agents conventions go here\n")?;
    write_spec(
        &ws.root,
        "0001-signup",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_full_mixed_body("SPEC-0001")),
    )?;
    Ok(())
}

fn seed_no_note_workspace(ws: &Workspace) -> TestResult {
    write_agents(ws, "# Agents\n")?;
    write_spec(
        &ws.root,
        "0001-parser",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md_no_implementer_note("SPEC-0001")),
    )?;
    Ok(())
}

// -- CHK-003 ----------------------------------------------------------------

#[test]
fn review_prompt_redacts_every_implementer_note_body_and_preserves_other_items() -> TestResult {
    let ws = Workspace::new()?;
    seed_full_mixed_workspace(&ws)?;

    let out = capture_review_stdout(&ws, "SPEC-0001/T-001", "business")?;
    let entry = task_entry_section(&out);

    // Other body items pass through verbatim.
    assert!(
        entry.contains("Free prose ahead of any element block."),
        "free prose must pass through verbatim in `## Task entry`: {entry}",
    );
    assert!(
        entry.contains("Given a signup form, when submitted with a valid email"),
        "<task-scenarios> body must pass through verbatim: {entry}",
    );
    assert!(
        entry.contains("Scope drifted into password reset which was a non-goal."),
        "blocking <review> body must pass through verbatim: {entry}",
    );
    assert!(
        entry.contains("Scope restored; matches the SPEC intent."),
        "passing <review> body must pass through verbatim: {entry}",
    );
    assert!(
        entry.contains("persona=\"business\" verdict=\"blocking\""),
        "blocking <review> attributes must be preserved: {entry}",
    );
    assert!(
        entry.contains("persona=\"business\" verdict=\"pass\""),
        "passing <review> attributes must be preserved: {entry}",
    );
    assert!(
        entry.contains("Narrow the change to the signup happy path"),
        "<retry> body must pass through verbatim: {entry}",
    );
    assert!(
        entry.contains("- Suggested files: `src/auth.rs`, `tests/signup.rs`"),
        "`Suggested files:` markdown bullet must pass through verbatim: {entry}",
    );

    // Zero bytes of either <implementer-note> body — neither s1 nor s1-retry.
    assert!(
        !entry.contains("shipped the signup handler"),
        "initial <implementer-note> body must be fully redacted: {entry}",
    );
    assert!(
        !entry.contains("scope narrowed; password reset reverted"),
        "retry-session <implementer-note> body must be fully redacted: {entry}",
    );
    assert!(
        !entry.contains("<implementer-note"),
        "<implementer-note> open tag must not appear in redacted entry: {entry}",
    );
    assert!(
        !entry.contains("</implementer-note>"),
        "<implementer-note> close tag must not appear in redacted entry: {entry}",
    );

    // None of the six implementer-note sub-bullet labels survive.
    for forbidden in [
        "Commands run:",
        "Exit codes:",
        "Discovered issues:",
        "Procedural compliance:",
        "Undone:",
        "Completed:",
    ] {
        assert!(
            !entry.contains(forbidden),
            "redacted `## Task entry` must not contain `{forbidden}`: {entry}",
        );
    }

    Ok(())
}

#[test]
fn review_prompt_has_no_placeholder_marker_indicating_redaction() -> TestResult {
    let ws = Workspace::new()?;
    seed_full_mixed_workspace(&ws)?;

    let out = capture_review_stdout(&ws, "SPEC-0001/T-001", "business")?;
    let entry = task_entry_section(&out);

    // DEC-002: silent redaction. No placeholder prose.
    for marker in [
        "redacted",
        "withheld",
        "notes omitted",
        "implementer notes hidden",
        "notes hidden",
    ] {
        assert!(
            !entry.to_ascii_lowercase().contains(marker),
            "redacted entry must contain no placeholder marker `{marker}`: {entry}",
        );
    }

    Ok(())
}

#[test]
fn review_prompt_task_entry_is_byte_identical_across_all_six_personas() -> TestResult {
    let ws = Workspace::new()?;
    seed_full_mixed_workspace(&ws)?;

    let mut entries: Vec<(String, String)> = Vec::with_capacity(PERSONAS_ALL.len());
    for persona in PERSONAS_ALL {
        let out = capture_review_stdout(&ws, "SPEC-0001/T-001", persona)?;
        entries.push(((*persona).to_owned(), task_entry_section(&out).to_owned()));
    }

    let (first_name, first_entry) = entries
        .first()
        .expect("PERSONAS_ALL must be non-empty")
        .clone();
    for (name, entry) in entries.iter().skip(1) {
        assert_eq!(
            entry, &first_entry,
            "`## Task entry` must be byte-identical across personas; `{name}` differs from `{first_name}`",
        );
    }

    Ok(())
}

#[test]
fn review_prompt_with_no_implementer_note_renders_unchanged_task_entry() -> TestResult {
    let ws = Workspace::new()?;
    seed_no_note_workspace(&ws)?;

    let out = capture_review_stdout(&ws, "SPEC-0001/T-001", "business")?;
    let entry = task_entry_section(&out);

    // No-op redaction: everything in the source <task> survives.
    assert!(
        entry.contains("Free prose with no implementer notes yet."),
        "no-op redaction must preserve free prose: {entry}",
    );
    assert!(
        entry.contains("Given the parser, when there is nothing to redact"),
        "no-op redaction must preserve <task-scenarios>: {entry}",
    );
    assert!(
        entry.contains("- Suggested files: `src/parser.rs`"),
        "no-op redaction must preserve `Suggested files:`: {entry}",
    );
    Ok(())
}

// -- CHK-004 (cross-check that implement is NOT redacted) -------------------

#[test]
fn implement_prompt_is_not_redacted_and_carries_implementer_note_bodies() -> TestResult {
    let ws = Workspace::new()?;
    seed_full_mixed_workspace(&ws)?;

    let out = capture_implement_stdout(&ws, "SPEC-0001/T-001")?;
    let entry = task_entry_section(&out);

    assert!(
        entry.contains("<implementer-note"),
        "implement prompt's `## Task entry` must carry `<implementer-note` (REQ-004): {entry}",
    );
    assert!(
        entry.contains("shipped the signup handler"),
        "implement prompt must carry the initial note body verbatim: {entry}",
    );
    assert!(
        entry.contains("scope narrowed; password reset reverted"),
        "implement prompt must carry the retry-session note body verbatim: {entry}",
    );
    Ok(())
}
