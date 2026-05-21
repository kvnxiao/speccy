#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Tests for REQ-008 (SPEC-0033 T-007): skill and phase body files must
//! discover speccy resources via CLI JSON envelopes only, not via
//! direct filesystem patterns.
//!
//! SPEC-0033 T-007 checks:
//!
//! - [`chk014_no_direct_speccy_resource_patterns_in_skills_or_phases`]: no raw
//!   `.speccy/specs/*` globs or bare `SPEC.md`/`TASKS.md`/`MISSION.md`/
//!   `REPORT.md` paths (not bound to a template placeholder) appear in any
//!   skill or phase body file.
//! - [`chk015_speccy_plan_uses_vacancy_not_status_for_new_spec_id`]:
//!   `speccy-plan.md` invokes `speccy vacancy --json` (not `speccy status
//!   --json`) to allocate a new SPEC ID.
//! - [`no_old_cli_verbs_in_skill_or_phase_bodies`]: deleted CLI verbs (`speccy
//!   plan`, `speccy tasks`, `speccy implement`, `speccy review`, `speccy
//!   report`) do not appear as commands in any skill or phase body file.
//! - [`no_kind_filter_flag_in_skill_or_phase_bodies`]: the removed `--kind`
//!   flag to `speccy next` does not appear in any skill or phase body file.

use speccy_cli::embedded::RESOURCES;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read a module file (skill or phase body) from the embedded RESOURCES
/// bundle by sub-path relative to `modules/` (e.g.
/// `"skills/speccy-plan.md"` or `"phases/speccy-work.md"`).
fn read_module(sub_path: &str) -> Option<&'static str> {
    let path = format!("modules/{sub_path}");
    RESOURCES.get_file(&path).and_then(|f| f.contents_utf8())
}

/// Read a module file, panicking with a clear message if absent.
fn require_module(sub_path: &str) -> &'static str {
    read_module(sub_path).unwrap_or_else(|| {
        panic_with_message(&format!(
            "RESOURCES bundle must contain `modules/{sub_path}`; \
             check that the file exists under `resources/modules/`",
        ))
    })
}

/// Test-only failure path. Centralised so the `clippy::panic` expectation
/// is scoped to one function.
#[expect(
    clippy::panic,
    reason = "test-only fixture lookup; failure is a developer-facing assertion"
)]
fn panic_with_message(msg: &str) -> ! {
    panic!("{msg}");
}

// ---------------------------------------------------------------------------
// Module lists
// ---------------------------------------------------------------------------

const SKILL_FILES: &[&str] = &[
    "skills/speccy-plan.md",
    "skills/speccy-amend.md",
    "skills/speccy-brainstorm.md",
    "skills/speccy-review.md",
];

const PHASE_FILES: &[&str] = &[
    "phases/speccy-tasks.md",
    "phases/speccy-work.md",
    "phases/speccy-ship.md",
    "phases/speccy-init.md",
];

/// Persona snippet files that are shared building blocks — they do NOT
/// individually contain discovery patterns, but the reviewer persona bodies
/// that include them are checked. The non-reviewer files (`implementer.md`,
/// `planner.md`) are interactive-skill prompts and are also checked.
const PERSONA_FILES: &[&str] = &[
    "personas/reviewer-architecture.md",
    "personas/reviewer-business.md",
    "personas/reviewer-docs.md",
    "personas/reviewer-security.md",
    "personas/reviewer-style.md",
    "personas/reviewer-tests.md",
    "personas/implementer.md",
    "personas/planner.md",
    "personas/diff_fetch_command.md",
    "personas/inline_note_format.md",
    "personas/no_tasks_md_writes.md",
    "personas/verdict_return_contract.md",
];

// ---------------------------------------------------------------------------
// CHK-014: no direct speccy-resource discovery patterns
// ---------------------------------------------------------------------------

/// CHK-014: no `.speccy/specs/*` glob expressions, no bare `SPEC.md` /
/// `TASKS.md` / `MISSION.md` / `REPORT.md` filesystem paths (not bound
/// to a `{{ ... }}` template placeholder), and no directory-enumeration
/// instructions targeting `.speccy/specs/` appear in any skill, phase,
/// or persona body file.
///
/// General-purpose Read/Glob/grep references for non-speccy project files
/// (AGENTS.md, Cargo.toml, source code) are NOT violations.
#[test]
fn chk014_no_direct_speccy_resource_patterns_in_skills_or_phases() {
    // Patterns that indicate direct filesystem discovery of speccy resources.
    // Each entry is a (pattern, description) pair.
    let forbidden_patterns: &[(&str, &str)] =
        &[(".speccy/specs/*", "glob discovery of .speccy/specs/")];

    // Check all skill, phase, and persona files.
    let all_files: Vec<&str> = SKILL_FILES
        .iter()
        .chain(PHASE_FILES.iter())
        .chain(PERSONA_FILES.iter())
        .copied()
        .collect();

    for sub_path in &all_files {
        let body = require_module(sub_path);
        for (pattern, desc) in forbidden_patterns {
            assert!(
                !body.contains(pattern),
                "skill/phase/persona file `resources/modules/{sub_path}` \
                 contains `{pattern}` which is a {desc}; \
                 use `speccy status --json`, `speccy next --json`, or \
                 `speccy vacancy --json` CLI envelopes instead \
                 (SPEC-0033 REQ-008 / CHK-014)",
            );
        }

        // Check for raw bare speccy-resource filename references used as
        // direct Read/Glob/filesystem targets (not prose mentions). The
        // violation pattern is a line in a code fence or an explicit tool
        // call that references the bare filename as a positional argument
        // or path without a leading speccy-output field binding it. We
        // look specifically for `Read SPEC.md`, `cat SPEC.md`, or similar
        // direct target invocations — NOT prose mentions like "edit SPEC.md".
        //
        // The patterns below match known direct-target forms. They
        // deliberately do NOT match prose lines that mention the filenames
        // as objects of discussion (those are valid and abundant in skill
        // bodies), nor do they match `spec_md_path`, `tasks_md_path` field
        // names, nor paths that already contain an NNNN-slug segment
        // (those come from CLI output and are already bound).
        let direct_read_patterns: &[(&str, &str)] = &[
            ("Read SPEC.md", "direct `Read SPEC.md` tool call"),
            ("Read TASKS.md", "direct `Read TASKS.md` tool call"),
            ("Read MISSION.md", "direct `Read MISSION.md` tool call"),
            ("Read REPORT.md", "direct `Read REPORT.md` tool call"),
        ];
        for (pattern, desc) in direct_read_patterns {
            assert!(
                !body.contains(pattern),
                "skill/phase/persona file `resources/modules/{sub_path}` \
                 contains `{pattern}` which is a {desc}; \
                 obtain the path from `speccy status --json` or \
                 `speccy next --json` path fields instead \
                 (SPEC-0033 REQ-008 / CHK-014)",
            );
        }

        // Separately check for `.speccy/specs/` directory enumeration
        // instructions (e.g. "Scan .speccy/specs/", "walk .speccy/specs/").
        // We only flag cases where `.speccy/specs/` appears as a scan target,
        // not in historical prose like "the workspace scanner reads
        // `.speccy/specs/`". The pattern `.speccy/specs/` preceded by verbs
        // that indicate active scanning is the violation.
        //
        // The brainstorm body had "Scan\n   `.speccy/specs/`" (newline-split),
        // so we normalise whitespace before checking.
        let normalised = body.split_whitespace().collect::<Vec<_>>().join(" ");
        let scan_instruction = normalised.contains("Scan `.speccy/specs/`")
            || normalised.contains("scan `.speccy/specs/`")
            || normalised.contains("walk `.speccy/specs/`")
            || normalised.contains("Walk `.speccy/specs/`");
        assert!(
            !scan_instruction,
            "skill/phase/persona file `resources/modules/{sub_path}` \
             contains a directory-enumeration instruction targeting \
             `.speccy/specs/`; use `speccy status --json` or \
             `speccy next --json` instead (SPEC-0033 REQ-008 / CHK-014)",
        );
    }
}

// ---------------------------------------------------------------------------
// CHK-015: speccy-plan uses vacancy not status for new SPEC ID
// ---------------------------------------------------------------------------

/// CHK-015: `resources/modules/skills/speccy-plan.md` invokes
/// `speccy vacancy --json` to allocate a new SPEC ID, not
/// `speccy status --json`.
#[test]
fn chk015_speccy_plan_uses_vacancy_not_status_for_new_spec_id() {
    let body = require_module("skills/speccy-plan.md");

    assert!(
        body.contains("speccy vacancy --json"),
        "`resources/modules/skills/speccy-plan.md` must invoke \
         `speccy vacancy --json` to learn the next SPEC ID \
         (SPEC-0033 REQ-008 / CHK-015)",
    );

    assert!(
        !body.contains("speccy status --json"),
        "`resources/modules/skills/speccy-plan.md` must NOT invoke \
         `speccy status --json` to allocate a new SPEC ID \
         — use `speccy vacancy --json` instead \
         (SPEC-0033 REQ-008 / CHK-015)",
    );
}

// ---------------------------------------------------------------------------
// No old (deleted) CLI verbs in skill or phase body files
// ---------------------------------------------------------------------------

/// Deleted CLI commands (`speccy plan`, `speccy tasks`, `speccy implement`,
/// `speccy review`, `speccy report`) must not appear as invokable commands
/// in any skill, phase, or persona body file.
///
/// The test checks for the command patterns as they would appear in
/// code-fenced blocks (the primary way commands are presented to agents).
/// Prose text that mentions a verb as a noun (e.g. "the plan phase") is
/// not a violation; the pattern `speccy plan` followed by a word boundary
/// or end-of-text in a code fence is.
#[test]
fn no_old_cli_verbs_in_skill_or_phase_bodies() {
    // These are verb-only patterns — each is a complete `speccy <verb>`
    // invocation that should no longer appear in skill or persona bodies.
    // We look for them in code fences where they would be executed.
    let deleted_verb_patterns: &[(&str, &str)] = &[
        ("speccy plan ", "`speccy plan` (deleted in SPEC-0033 T-001)"),
        (
            "speccy plan\n",
            "`speccy plan` (deleted in SPEC-0033 T-001)",
        ),
        (
            "speccy tasks ",
            "`speccy tasks` (deleted in SPEC-0033 T-001)",
        ),
        (
            "speccy tasks\n",
            "`speccy tasks` (deleted in SPEC-0033 T-001)",
        ),
        (
            "speccy implement ",
            "`speccy implement` (deleted in SPEC-0033 T-001)",
        ),
        (
            "speccy implement\n",
            "`speccy implement` (deleted in SPEC-0033 T-001)",
        ),
        (
            "speccy review ",
            "`speccy review` (deleted in SPEC-0033 T-001)",
        ),
        (
            "speccy review\n",
            "`speccy review` (deleted in SPEC-0033 T-001)",
        ),
        (
            "speccy report ",
            "`speccy report` (deleted in SPEC-0033 T-001)",
        ),
        (
            "speccy report\n",
            "`speccy report` (deleted in SPEC-0033 T-001)",
        ),
    ];

    let all_files: Vec<&str> = SKILL_FILES
        .iter()
        .chain(PHASE_FILES.iter())
        .chain(PERSONA_FILES.iter())
        .copied()
        .collect();

    for sub_path in &all_files {
        let body = require_module(sub_path);
        for (pattern, desc) in deleted_verb_patterns {
            assert!(
                !body.contains(pattern),
                "skill/phase/persona file `resources/modules/{sub_path}` \
                 contains `{pattern}` which references the deleted command \
                 {desc}; remove or replace with the equivalent current \
                 workflow (SPEC-0033 REQ-008)",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// CHK-019: speccy-tasks template documents TASKS.md output shape
// ---------------------------------------------------------------------------

/// CHK-019: `resources/modules/phases/speccy-tasks.md` Step 2 must contain
/// a concrete example fragment documenting the required TASKS.md output
/// shape, including the `# Tasks: SPEC-` level-1 heading and the
/// space-separated `covers="REQ-001 REQ-002"` multi-REQ form.
///
/// The two literal substrings CHK-019 asserts are the same substrings that
/// downstream agents must produce — asserting them here locks the template
/// wording to the parser's expectations (REQ-013 / SPEC-0034).
#[test]
fn chk019_speccy_tasks_template_documents_output_shape() {
    let body = require_module("phases/speccy-tasks.md");

    assert!(
        body.contains("# Tasks: SPEC-"),
        "`resources/modules/phases/speccy-tasks.md` Step 2 must contain a \
         concrete example fragment with the literal substring `# Tasks: SPEC-` \
         to document the required level-1 heading shape (REQ-013 / CHK-019)",
    );

    assert!(
        body.contains(r#"covers="REQ-001 REQ-002""#),
        "`resources/modules/phases/speccy-tasks.md` Step 2 must contain a \
         concrete example fragment with the literal substring \
         `covers=\"REQ-001 REQ-002\"` to demonstrate the space-separated \
         multi-REQ form (REQ-013 / CHK-019)",
    );
}

// ---------------------------------------------------------------------------
// No --kind filter flag in skill or phase bodies
// ---------------------------------------------------------------------------

/// The removed `--kind` flag to `speccy next` must not appear in any
/// skill, phase, or persona body file (replaced by derived action-kind
/// logic in T-004).
#[test]
fn no_kind_filter_flag_in_skill_or_phase_bodies() {
    let all_files: Vec<&str> = SKILL_FILES
        .iter()
        .chain(PHASE_FILES.iter())
        .chain(PERSONA_FILES.iter())
        .copied()
        .collect();

    for sub_path in &all_files {
        let body = require_module(sub_path);
        assert!(
            !body.contains("--kind"),
            "skill/phase/persona file `resources/modules/{sub_path}` \
             contains `--kind` which references the removed \
             `speccy next --kind` flag; replace with \
             `speccy next SPEC-NNNN --json` or `speccy next --json` \
             (SPEC-0033 REQ-008 / T-004)",
        );
    }
}
