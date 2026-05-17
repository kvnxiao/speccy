#![expect(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Skill-pack content tests for SPEC-0013, SPEC-0014, and SPEC-0015.
//!
//! SPEC-0013 checks (`.speccy/specs/0013-skill-packs/spec.toml`):
//!
//! - CHK-001: [`persona_files_present`]
//! - CHK-002: [`persona_names_match_registry`]
//! - CHK-003: [`prompt_templates_present`]
//! - CHK-004: [`prompt_placeholders_match_commands`]
//! - CHK-005: [`claude_code_recipes`]
//! - CHK-006: [`codex_recipes`]
//! - CHK-007: [`persona_content_shape`]
//! - CHK-008: [`recipe_content_shape`]
//!
//! SPEC-0014 checks
//! (`.speccy/specs/0014-handoff-and-friction-conventions/spec.toml`):
//!
//! - CHK-001: [`implementer_prompt_handoff_template`]
//! - CHK-002: [`implementer_prompt_handoff_referenced_in_task_steps`]
//! - CHK-003: [`implementer_prompt_friction_section`]
//! - CHK-004: [`implementer_persona_friction_reference`]
//! - CHK-005: [`agents_md_friction_paragraph`]
//! - CHK-006: [`report_prompt_skill_updates_section`]
//!
//! SPEC-0015 checks
//! (`.speccy/specs/0015-host-skill-layout/spec.toml`):
//!
//! - CHK-001: [`bundle_layout_has_skill_md_per_host`]
//! - CHK-002: [`bundle_legacy_flat_layout_absent`]
//! - CHK-005: [`shipped_skill_md_frontmatter_shape`]
//! - CHK-006: [`shipped_descriptions_natural_language_triggers`]
//! - (CHK-003, CHK-004 live in `tests/init.rs`:
//!   `copy_claude_code_pack_skill_md`, `copy_codex_pack_skill_md`.)

use serde::Deserialize;
use speccy_cli::host::HostChoice;
use speccy_cli::render::render_host_pack;
use speccy_core::personas;
use speccy_core::personas::PERSONAS;
use speccy_core::prompt::PROMPTS;
use std::path::Path;

// --------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------

/// Read a host SKILL.md wrapper template body from the workspace
/// filesystem at `resources/agents/<install_root>/skills/<verb>/SKILL.md.tmpl`.
/// The wrapper templates carry the same `name` / `description`
/// frontmatter that the legacy per-host `skills/<host>/<verb>/SKILL.md`
/// files used to expose; SPEC-0016 T-008 retargeted the SPEC-0013 /
/// SPEC-0015 frontmatter-shape checks to read here directly.
fn read_wrapper_template(install_root: &str, verb: &str) -> String {
    let path = workspace_root()
        .join("resources")
        .join("agents")
        .join(install_root)
        .join("skills")
        .join(verb)
        .join("SKILL.md.tmpl");
    fs_err::read_to_string(&path).unwrap_or_else(|err| {
        panic_with_test_message(&format!(
            "wrapper template `resources/agents/{install_root}/skills/{verb}/SKILL.md.tmpl` should be readable: {err}"
        ))
    })
}

/// Look up a rendered SKILL.md body by verb in a `render_host_pack`
/// output vector. Helper centralises the path-prefix matching so the
/// SPEC-0013 content-shape tests read as the assertion only.
fn find_rendered_skill<'a>(
    rendered: &'a [speccy_cli::render::RenderedFile],
    install_root: &str,
    verb: &str,
) -> &'a str {
    let needle = format!("{install_root}/skills/{verb}/SKILL.md");
    let file = rendered
        .iter()
        .find(|f| f.rel_path.as_str() == needle)
        .unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "rendered host pack must contain `{needle}` (T-008 SPEC-0013 retarget)"
            ))
        });
    file.contents.as_str()
}

/// Read a persona body out of the `speccy-core` `PERSONAS` bundle by
/// leaf file name (e.g. `"reviewer-security.md"`). SPEC-0016 T-002
/// moved persona bodies from `skills/shared/personas/` (inside the
/// per-host SKILLS bundle) to `resources/modules/personas/` (inside
/// the host-neutral PERSONAS bundle exposed by `speccy-core`).
fn read_persona(name: &str) -> &'static str {
    let entry = PERSONAS.get_file(name).unwrap_or_else(|| {
        panic_with_test_message(&format!(
            "PERSONAS bundle should contain `{name}` (post-T-002 layout)"
        ))
    });
    entry
        .contents_utf8()
        .expect("PERSONAS bundle entries should be valid UTF-8")
}

/// Read a prompt template body out of the `speccy-core` `PROMPTS`
/// bundle by leaf file name (e.g. `"plan-greenfield.md"`). Same
/// rationale as [`read_persona`].
fn read_prompt(name: &str) -> &'static str {
    let entry = PROMPTS.get_file(name).unwrap_or_else(|| {
        panic_with_test_message(&format!(
            "PROMPTS bundle should contain `{name}` (post-T-002 layout)"
        ))
    });
    entry
        .contents_utf8()
        .expect("PROMPTS bundle entries should be valid UTF-8")
}

/// Test-only failure path. Centralised so the `clippy::panic` expectation
/// is scoped to one function instead of every helper.
#[expect(
    clippy::panic,
    reason = "test-only fixture lookup; failure is a developer-facing assertion"
)]
fn panic_with_test_message(msg: &str) -> ! {
    panic!("{msg}");
}

fn split_frontmatter(source: &str) -> Option<(&str, &str)> {
    let after_open = source
        .strip_prefix("---\n")
        .or_else(|| source.strip_prefix("---\r\n"))?;
    let close_idx = after_open.find("\n---")?;
    let yaml = after_open.get(..close_idx)?;
    let after_close = after_open.get(close_idx.saturating_add(4)..)?;
    let body = after_close.strip_prefix('\n').unwrap_or(after_close);
    Some((yaml, body))
}

#[derive(Debug, Deserialize)]
struct RecipeFrontmatter {
    description: String,
    #[serde(default)]
    name: Option<String>,
}

const PERSONA_FILES: &[&str] = &[
    "planner.md",
    "implementer.md",
    "reviewer-business.md",
    "reviewer-tests.md",
    "reviewer-security.md",
    "reviewer-style.md",
    "reviewer-architecture.md",
    "reviewer-docs.md",
];

const PROMPT_FILES: &[&str] = &[
    "plan-greenfield.md",
    "plan-amend.md",
    "tasks-generate.md",
    "tasks-amend.md",
    "implementer.md",
    "reviewer-business.md",
    "reviewer-tests.md",
    "reviewer-security.md",
    "reviewer-style.md",
    "reviewer-architecture.md",
    "reviewer-docs.md",
    "report.md",
];

// After SPEC-0023 REQ-001 / REQ-002, `speccy-work` and `speccy-review`
// are single-task primitives — one invocation, one task, exit — and
// no longer declare loop exit criteria. `speccy-amend` is the only
// remaining loop recipe.
const LOOP_RECIPES: &[&str] = &["speccy-amend/SKILL.md"];

const SKILL_NAMES: &[&str] = &[
    "speccy-init",
    "speccy-plan",
    "speccy-tasks",
    "speccy-work",
    "speccy-review",
    "speccy-ship",
    "speccy-amend",
];

/// Per-host install root for the SKILL.md wrappers under
/// `resources/agents/<root>/skills/<verb>/SKILL.md.tmpl`. Mirrors the
/// install destination established by SPEC-0015 (Claude Code →
/// `.claude/`, Codex → `.agents/`); `.codex/` is the subagent root and
/// has no skills bundle.
const HOST_SKILL_ROOTS: &[(&str, &str)] = &[("claude-code", ".claude"), ("codex", ".agents")];

const SPECCY_COMMANDS: &[&str] = &[
    "speccy init",
    "speccy plan",
    "speccy tasks",
    "speccy implement",
    "speccy review",
    "speccy report",
    "speccy status",
    "speccy next",
    "speccy check",
    "speccy verify",
];

// --------------------------------------------------------------------
// CHK-001
// --------------------------------------------------------------------

#[test]
fn persona_files_present() {
    for name in PERSONA_FILES {
        let body = read_persona(name);
        assert!(
            !body.trim().is_empty(),
            "persona `{name}` must be non-empty",
        );
    }
}

// --------------------------------------------------------------------
// CHK-002
// --------------------------------------------------------------------

#[test]
fn persona_names_match_registry() {
    for persona in personas::ALL {
        let file_name = format!("reviewer-{persona}.md");
        assert!(
            PERSONAS.get_file(&file_name).is_some(),
            "personas::ALL contains `{persona}` but `{file_name}` is missing from the PERSONAS bundle",
        );
    }
    for required in ["planner.md", "implementer.md"] {
        assert!(
            PERSONAS.get_file(required).is_some(),
            "`{required}` must ship alongside the reviewer personas",
        );
    }
}

// --------------------------------------------------------------------
// CHK-003
// --------------------------------------------------------------------

#[test]
fn prompt_templates_present() {
    for name in PROMPT_FILES {
        let body = read_prompt(name);
        assert!(
            !body.trim().is_empty(),
            "prompt template `{name}` must be non-empty",
        );
    }
}

// --------------------------------------------------------------------
// CHK-004
// --------------------------------------------------------------------

fn assert_placeholders(template: &str, expected: &[&str], file_name: &str) {
    for placeholder in expected {
        let needle = format!("{{{{{placeholder}}}}}");
        assert!(
            template.contains(&needle),
            "template `{file_name}` must contain placeholder `{needle}`",
        );
        for ch in placeholder.chars() {
            assert!(
                ch.is_ascii_alphanumeric() || ch == '_',
                "placeholder `{placeholder}` in `{file_name}` is not a valid identifier",
            );
        }
    }
}

#[test]
fn prompt_placeholders_match_commands() {
    let plan_greenfield = read_prompt("plan-greenfield.md");
    assert_placeholders(plan_greenfield, &["next_spec_id"], "plan-greenfield.md");
    // Negative: the retired `{{vision}}` placeholder must not appear
    // anywhere in plan-greenfield.md (Vision was swapped for Mission
    // and the product north star now lives in AGENTS.md).
    assert!(
        !plan_greenfield.contains("{{vision}}"),
        "plan-greenfield.md must not contain the retired `{{{{vision}}}}` placeholder",
    );

    let plan_amend = read_prompt("plan-amend.md");
    assert_placeholders(
        plan_amend,
        &["spec_id", "spec_md_path", "mission_section"],
        "plan-amend.md",
    );

    let tasks_generate = read_prompt("tasks-generate.md");
    assert_placeholders(
        tasks_generate,
        &["spec_id", "spec_md_path"],
        "tasks-generate.md",
    );

    let tasks_amend = read_prompt("tasks-amend.md");
    assert_placeholders(
        tasks_amend,
        &["spec_id", "spec_md_path", "tasks_md_path"],
        "tasks-amend.md",
    );

    let implementer = read_prompt("implementer.md");
    assert_placeholders(
        implementer,
        &[
            "spec_id",
            "spec_md_path",
            "task_id",
            "task_entry",
            "suggested_files",
        ],
        "implementer.md",
    );

    let reviewer_required = &[
        "spec_id",
        "spec_md_path",
        "task_id",
        "task_entry",
        "persona",
        "persona_content",
    ];
    for persona in personas::ALL {
        let file = format!("reviewer-{persona}.md");
        let body = read_prompt(&file);
        assert_placeholders(body, reviewer_required, &file);
        // SPEC-0023 REQ-003: the rendered prompt no longer inlines the
        // branch diff; the template instructs the reviewer agent to run
        // `git diff` itself instead.
        assert!(
            !body.contains("{{diff}}"),
            "reviewer template `{file}` must not contain the retired `{{{{diff}}}}` placeholder",
        );
        assert!(
            body.contains("git diff"),
            "reviewer template `{file}` must instruct the agent to run `git diff`",
        );
    }

    let report = read_prompt("report.md");
    assert_placeholders(
        report,
        &["spec_id", "spec_md_path", "tasks_md_path", "retry_summary"],
        "report.md",
    );

    // SPEC-0023 REQ-005: the `{{agents}}` placeholder is retired
    // workspace-wide. Modern AI coding harnesses auto-load `AGENTS.md`
    // themselves; the CLI no longer inlines it. Assert across every
    // prompt template.
    for name in PROMPT_FILES {
        let body = read_prompt(name);
        assert!(
            !body.contains("{{agents}}"),
            "template `{name}` must not contain the retired `{{{{agents}}}}` placeholder (SPEC-0023 REQ-005)",
        );
    }

    // SPEC-0023 REQ-006: the `{{spec_md}}`, `{{tasks_md}}`, and
    // `{{mission}}` interpolations are retired workspace-wide. Rendered
    // prompts now name the file's repo-relative path and the agent
    // reads it via the host's Read primitive on demand.
    for name in PROMPT_FILES {
        let body = read_prompt(name);
        for retired in ["{{spec_md}}", "{{tasks_md}}", "{{mission}}"] {
            assert!(
                !body.contains(retired),
                "template `{name}` must not contain the retired `{retired}` placeholder (SPEC-0023 REQ-006)",
            );
        }
    }

    // Negative: an obvious typo must not appear in any template.
    let typo = "{{spec_idd}}";
    for name in PROMPT_FILES {
        let body = read_prompt(name);
        assert!(
            !body.contains(typo),
            "template `{name}` must not contain placeholder typo `{typo}`",
        );
    }
}

// --------------------------------------------------------------------
// CHK-005 / CHK-006: recipe frontmatter
// --------------------------------------------------------------------

fn assert_wrapper_frontmatter(install_root: &str, verb: &str, require_name: bool) {
    let body = read_wrapper_template(install_root, verb);
    let (yaml, _rest) = split_frontmatter(&body).unwrap_or_else(|| {
        panic_with_test_message(&format!(
            "wrapper `resources/agents/{install_root}/skills/{verb}/SKILL.md.tmpl` must have a `---` frontmatter fence"
        ))
    });

    let fm: RecipeFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
        panic_with_test_message(&format!(
            "wrapper `resources/agents/{install_root}/skills/{verb}/SKILL.md.tmpl` frontmatter must be valid YAML: {err}"
        ))
    });

    assert!(
        !fm.description.trim().is_empty(),
        "wrapper `resources/agents/{install_root}/skills/{verb}/SKILL.md.tmpl` `description` field must be non-empty",
    );

    if require_name {
        let name = fm.name.as_deref().unwrap_or("");
        assert!(
            !name.trim().is_empty(),
            "wrapper `resources/agents/{install_root}/skills/{verb}/SKILL.md.tmpl` `name` field is required for Codex",
        );
    }
}

#[test]
fn claude_code_recipes() {
    for verb in SKILL_NAMES {
        assert_wrapper_frontmatter(".claude", verb, true);
    }
}

#[test]
fn codex_recipes() {
    for verb in SKILL_NAMES {
        assert_wrapper_frontmatter(".agents", verb, true);
    }
}

// --------------------------------------------------------------------
// CHK-007: persona content shape (reviewer personas only)
// --------------------------------------------------------------------

#[test]
fn persona_content_shape() {
    let required_headings: &[&str] = &[
        "## Role",
        "## Focus",
        "## What to look for that's easy to miss",
        "## Inline note format",
        "## Example",
    ];
    for persona in personas::ALL {
        let file = format!("reviewer-{persona}.md");
        let body = read_persona(&file);
        assert!(
            body.contains("# Reviewer Persona:"),
            "persona `{file}` must open with `# Reviewer Persona: ...`",
        );

        let mut cursor: usize = 0;
        for heading in required_headings {
            let body_slice = body.get(cursor..).unwrap_or_default();
            let offset = body_slice.find(heading).unwrap_or_else(|| {
                panic_with_test_message(&format!(
                    "persona `{file}` is missing heading `{heading}` (or it appears out of order)"
                ))
            });
            cursor = cursor.saturating_add(offset).saturating_add(heading.len());
        }
    }
}

// --------------------------------------------------------------------
// CHK-008: recipe content shape
// --------------------------------------------------------------------

fn first_non_frontmatter_paragraph(body: &str) -> Option<&str> {
    let after = split_frontmatter(body).map_or(body, |(_yaml, rest)| rest);
    after
        .lines()
        .skip_while(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
        .find(|line| !line.trim().is_empty())
}

fn contains_speccy_command_in_code_fence(body: &str) -> bool {
    let mut in_fence = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            continue;
        }
        let trimmed = line.trim_start();
        if SPECCY_COMMANDS.iter().any(|cmd| trimmed.starts_with(cmd)) {
            return true;
        }
    }
    false
}

#[test]
fn recipe_content_shape() {
    // SPEC-0016 T-008: the per-host rendered output is the new
    // "shipped recipe" surface. We render once per host through the
    // SPEC-0016 renderer and check the rendered SKILL.md body against
    // the same content-shape invariants the legacy per-host files used
    // to satisfy.
    for (host, install_root) in [
        (HostChoice::ClaudeCode, ".claude"),
        (HostChoice::Codex, ".agents"),
    ] {
        let rendered = render_host_pack(host).unwrap_or_else(|err| {
            panic_with_test_message(&format!("render_host_pack({host:?}) should succeed: {err}"))
        });
        for verb in SKILL_NAMES {
            let body = find_rendered_skill(&rendered, install_root, verb);

            assert!(
                first_non_frontmatter_paragraph(body).is_some(),
                "rendered recipe `{install_root}/skills/{verb}/SKILL.md` must include an intro paragraph after the title",
            );

            assert!(
                body.contains("## When to use"),
                "rendered recipe `{install_root}/skills/{verb}/SKILL.md` must contain a `## When to use` section",
            );

            assert!(
                contains_speccy_command_in_code_fence(body),
                "rendered recipe `{install_root}/skills/{verb}/SKILL.md` must contain a fenced code block with a v1 `speccy ...` command",
            );

            let recipe_name = format!("{verb}/SKILL.md");
            if LOOP_RECIPES.contains(&recipe_name.as_str()) {
                let lower = body.to_lowercase();
                assert!(
                    lower.contains("loop exit") || lower.contains("exit criteria"),
                    "loop recipe `{install_root}/skills/{verb}/SKILL.md` must declare explicit loop exit criteria",
                );
            }
        }
    }
}

// --------------------------------------------------------------------
// SPEC-0014 helpers and fixtures
// --------------------------------------------------------------------

/// Six handoff-note field labels from SPEC-0014 DEC-001. Order matters
/// for readability only; presence is what the check enforces.
const HANDOFF_LABELS: [&str; 6] = [
    "Completed",
    "Undone",
    "Commands run",
    "Exit codes",
    "Discovered issues",
    "Procedural compliance",
];

/// Stable phrase the friction-to-skill-update pattern reuses across
/// the implementer prompt, the implementer persona, and AGENTS.md.
/// Changing it is a coordinated edit across all three files.
const FRICTION_PHRASE: &str = "update the relevant skill file under `skills/`";

/// Pulls fenced code blocks out of a markdown body. When `lang_filter`
/// is `Some`, only blocks opened with that language tag are returned;
/// `None` returns every fenced block. Bodies are concatenated lines
/// (newline-terminated) so substring checks behave naturally.
fn fenced_blocks(body: &str, lang_filter: Option<&str>) -> Vec<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    let mut current_matches = false;
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("```") {
            if let Some(open) = current.take() {
                if current_matches {
                    blocks.push(open);
                }
                current_matches = false;
            } else {
                let lang = rest.trim();
                current_matches = lang_filter.is_none_or(|want| lang == want);
                current = Some(String::new());
            }
            continue;
        }
        if let Some(buf) = current.as_mut() {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    blocks
}

/// Returns the slice of `body` belonging to the H2 section opened by
/// `heading` (exclusive of the heading line itself), terminated by
/// the next `\n## ` boundary or end of file.
fn section_body<'a>(body: &'a str, heading: &str) -> Option<&'a str> {
    let start = body.find(heading)?;
    let after_heading = body.get(start.checked_add(heading.len())?..)?;
    let end = after_heading.find("\n## ").unwrap_or(after_heading.len());
    after_heading.get(..end)
}

// --------------------------------------------------------------------
// SPEC-0014 CHK-001
// --------------------------------------------------------------------

#[test]
fn implementer_prompt_handoff_template() {
    let body = read_prompt("implementer.md");
    let blocks = fenced_blocks(body, Some("markdown"));
    assert!(
        !blocks.is_empty(),
        "implementer prompt must contain at least one ```markdown fenced block",
    );

    let found = blocks
        .iter()
        .any(|b| HANDOFF_LABELS.iter().all(|label| b.contains(label)));
    assert!(
        found,
        "implementer prompt must contain a ```markdown fenced block with all six handoff labels verbatim: {HANDOFF_LABELS:?}",
    );
}

// --------------------------------------------------------------------
// SPEC-0014 CHK-002
// --------------------------------------------------------------------

#[test]
fn implementer_prompt_handoff_referenced_in_task_steps() {
    let body = read_prompt("implementer.md");
    let task_section = section_body(body, "## Your task")
        .expect("implementer prompt must contain a `## Your task` section");

    assert!(
        task_section.contains("handoff template"),
        "`## Your task` must reference the handoff template by name",
    );
    assert!(
        task_section.contains("(none)"),
        "`## Your task` must instruct writing `(none)` for empty fields",
    );

    // The old freeform sentence must be gone everywhere in the prompt.
    let old_phrase =
        "summarizing what you did, including any out-of-scope edits made for the test to compile";
    assert!(
        !body.contains(old_phrase),
        "the pre-edit freeform implementer-note instruction must be removed from the prompt",
    );
}

// --------------------------------------------------------------------
// SPEC-0014 CHK-003
// --------------------------------------------------------------------

#[test]
fn implementer_prompt_friction_section() {
    let body = read_prompt("implementer.md");
    let heading = "## When you hit friction";
    let section = section_body(body, heading).unwrap_or_else(|| {
        panic_with_test_message(&format!(
            "implementer prompt must contain the `{heading}` heading"
        ))
    });

    let blocks = fenced_blocks(section, None);
    assert!(
        blocks.iter().any(|b| b.contains("skills/")),
        "`{heading}` section must contain at least one fenced block referencing a `skills/` path",
    );

    // Ordering invariant: friction section sits between Suggested files
    // and Your task so the implementer reads it before producing work.
    let suggested = body
        .find("## Suggested files")
        .expect("implementer prompt must contain `## Suggested files`");
    let friction = body
        .find(heading)
        .expect("implementer prompt must contain the friction heading");
    let your_task = body
        .find("## Your task")
        .expect("implementer prompt must contain `## Your task`");
    assert!(
        suggested < friction && friction < your_task,
        "`{heading}` must sit between `## Suggested files` and `## Your task`",
    );
}

// --------------------------------------------------------------------
// SPEC-0014 CHK-004
// --------------------------------------------------------------------

#[test]
fn implementer_persona_friction_reference() {
    let body = read_persona("implementer.md");
    let section = section_body(body, "## What to consider")
        .expect("implementer persona must contain `## What to consider`");

    assert!(
        section.contains(FRICTION_PHRASE),
        "`## What to consider` must contain the stable friction phrase `{FRICTION_PHRASE}`",
    );
    assert!(
        section.contains("friction"),
        "`## What to consider` must mention friction explicitly",
    );
    assert!(
        section.contains("## When you hit friction"),
        "`## What to consider` must point back to the prompt's `## When you hit friction` section",
    );
}

// --------------------------------------------------------------------
// SPEC-0014 CHK-005
// --------------------------------------------------------------------

/// `AGENTS.md` is not part of the embedded skill bundle; it lives at
/// the workspace root. We pull it in at compile time via `include_str!`
/// relative to this test file so the test stays hermetic.
const AGENTS_MD: &str = include_str!("../../AGENTS.md");

#[test]
fn agents_md_friction_paragraph() {
    let section = section_body(AGENTS_MD, "## Conventions for AI agents specifically")
        .expect("AGENTS.md must contain `## Conventions for AI agents specifically`");

    assert!(
        section.contains(FRICTION_PHRASE),
        "AGENTS.md conventions section must contain the stable friction phrase `{FRICTION_PHRASE}`",
    );
    assert!(
        section.contains("Procedural compliance"),
        "AGENTS.md conventions section must reference the `Procedural compliance` handoff field",
    );
}

// --------------------------------------------------------------------
// SPEC-0014 CHK-006
// --------------------------------------------------------------------

#[test]
fn report_prompt_skill_updates_section() {
    let body = read_prompt("report.md");

    let out_of_scope = body
        .find("## Out-of-scope items absorbed")
        .expect("report prompt must contain `## Out-of-scope items absorbed`");
    let skill_updates = body
        .find("## Skill updates")
        .expect("report prompt must contain `## Skill updates`");
    let deferred = body
        .find("## Deferred / known limitations")
        .expect("report prompt must contain `## Deferred / known limitations`");

    assert!(
        out_of_scope < skill_updates,
        "`## Skill updates` must appear after `## Out-of-scope items absorbed`",
    );
    assert!(
        skill_updates < deferred,
        "`## Skill updates` must appear before `## Deferred / known limitations`",
    );

    assert!(
        body.contains("git diff --name-only -- skills/"),
        "report prompt must reference `git diff --name-only -- skills/` as the derivation path for the skill-updates list",
    );
}

// --------------------------------------------------------------------
// SPEC-0015 CHK-001: bundle layout (per-host SKILL.md directories)
// --------------------------------------------------------------------

#[test]
fn bundle_layout_has_skill_md_per_host() {
    // SPEC-0016 T-008: the per-host SKILL.md surface now lives as
    // wrapper templates under `resources/agents/<install_root>/skills/`.
    // Claude Code → `.claude/skills/`, Codex → `.agents/skills/`.
    let root = workspace_root();
    for (_host, install_root) in HOST_SKILL_ROOTS {
        for skill in SKILL_NAMES {
            let rel = format!("resources/agents/{install_root}/skills/{skill}/SKILL.md.tmpl");
            let path = root.join(&rel);
            let body = fs_err::read_to_string(&path).unwrap_or_else(|err| {
                panic_with_test_message(&format!(
                    "workspace must contain `{rel}` (SPEC-0015 REQ-001 + CHK-001, retargeted by SPEC-0016 T-008): {err}"
                ))
            });
            assert!(!body.trim().is_empty(), "wrapper `{rel}` must be non-empty");
        }
    }
}

// --------------------------------------------------------------------
// SPEC-0015 CHK-002: legacy `skills/` tree removed from the workspace
// (post-SPEC-0016 T-008 the entire legacy tree is gone, not just its
// flat sub-layout).
// --------------------------------------------------------------------

#[test]
fn bundle_legacy_flat_layout_absent() {
    let root = workspace_root();
    let path = root.join("skills");
    assert!(
        !path.exists(),
        "legacy `skills/` tree must be gone from the workspace (SPEC-0016 T-008); per-host wrappers now live under `resources/agents/<install_root>/skills/`",
    );
}

// --------------------------------------------------------------------
// SPEC-0015 CHK-005: SKILL.md frontmatter shape (name matches dir,
// description is a single line)
// --------------------------------------------------------------------

#[test]
fn shipped_skill_md_frontmatter_shape() {
    // SPEC-0016 T-008: the SKILL.md frontmatter now lives in the
    // per-host wrapper templates at
    // `resources/agents/<install_root>/skills/<verb>/SKILL.md.tmpl`.
    // Frontmatter content is byte-identical between the wrapper and
    // the (now-deleted) legacy per-host SKILL.md file, so the
    // SPEC-0015 REQ-003 shape assertions transfer over unchanged.
    for (_host, install_root) in HOST_SKILL_ROOTS {
        for skill in SKILL_NAMES {
            let body = read_wrapper_template(install_root, skill);
            let label = format!("resources/agents/{install_root}/skills/{skill}/SKILL.md.tmpl");
            let (yaml, _rest) = split_frontmatter(&body).unwrap_or_else(|| {
                panic_with_test_message(&format!("`{label}` must have a `---` frontmatter fence"))
            });

            let fm: RecipeFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
                panic_with_test_message(&format!("`{label}` frontmatter must be valid YAML: {err}"))
            });

            let name = fm.name.as_deref().unwrap_or("");
            assert!(
                !name.trim().is_empty(),
                "`{label}` `name` field is required (SPEC-0015 REQ-003)",
            );
            assert_eq!(
                name, *skill,
                "`{label}` `name` field must equal the parent directory `{skill}` (SPEC-0015 REQ-003)",
            );
            assert!(
                !fm.description.trim().is_empty(),
                "`{label}` `description` field must be non-empty",
            );
            assert!(
                !fm.description.contains('\n'),
                "`{label}` `description` must be a single line (no embedded newlines) so both hosts' YAML loaders agree on its shape",
            );
        }
    }
}

// --------------------------------------------------------------------
// SPEC-0015 CHK-006: descriptions tuned for natural-language activation
// --------------------------------------------------------------------

#[test]
fn shipped_descriptions_natural_language_triggers() {
    const MAX_DESCRIPTION_CHARS: usize = 500;
    for (_host, install_root) in HOST_SKILL_ROOTS {
        for skill in SKILL_NAMES {
            let body = read_wrapper_template(install_root, skill);
            let label = format!("resources/agents/{install_root}/skills/{skill}/SKILL.md.tmpl");
            let (yaml, _rest) = split_frontmatter(&body).unwrap_or_else(|| {
                panic_with_test_message(&format!("`{label}` must have a `---` frontmatter fence"))
            });
            let fm: RecipeFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
                panic_with_test_message(&format!("`{label}` frontmatter must be valid YAML: {err}"))
            });
            let desc = fm.description.trim();

            // Anti-pattern: "Phase N." internal jargon at the start.
            let phase_prefix = desc.starts_with("Phase ")
                && desc
                    .chars()
                    .nth("Phase ".len())
                    .is_some_and(|c| c.is_ascii_digit());
            assert!(
                !phase_prefix,
                "`{label}` description must not start with `Phase <digit>` jargon (SPEC-0015 REQ-004); got: {desc:?}",
            );

            // Required trigger marker for natural-language activation.
            assert!(
                desc.to_lowercase().contains("use when"),
                "`{label}` description must contain a `use when` trigger marker (case-insensitive); got: {desc:?}",
            );

            // Codex caps the skill list at ~2% of the context window, so
            // every description has to stay tight.
            let len = desc.chars().count();
            assert!(
                len <= MAX_DESCRIPTION_CHARS,
                "`{label}` description must be at most {MAX_DESCRIPTION_CHARS} characters; got {len}",
            );
        }
    }
}

// --------------------------------------------------------------------
// SPEC-0016 T-002: legacy `skills/shared/` is gone from the workspace.
// --------------------------------------------------------------------

/// Workspace root, derived from `CARGO_MANIFEST_DIR` (the `speccy-cli`
/// crate dir) by walking one level up.
fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        std::path::Path::to_path_buf,
    )
}

#[test]
fn t002_workspace_has_no_skills_shared_personas_or_prompts() {
    let root = workspace_root();
    let personas_dir = root.join("skills").join("shared").join("personas");
    let prompts_dir = root.join("skills").join("shared").join("prompts");
    assert!(
        !personas_dir.exists(),
        "after T-002, `skills/shared/personas/` must not exist on disk \
         (personas now live under `resources/modules/personas/`); \
         found at {}",
        personas_dir.display(),
    );
    assert!(
        !prompts_dir.exists(),
        "after T-002, `skills/shared/prompts/` must not exist on disk \
         (prompts now live under `resources/modules/prompts/`); found \
         at {}",
        prompts_dir.display(),
    );
}

#[test]
fn t002_resources_modules_personas_and_prompts_are_non_empty() {
    let root = workspace_root();
    let personas_dir = root.join("resources").join("modules").join("personas");
    let prompts_dir = root.join("resources").join("modules").join("prompts");
    let persona_count = fs_err::read_dir(&personas_dir)
        .map(|it| {
            it.filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                })
                .count()
        })
        .expect("resources/modules/personas/ must exist after T-002");
    let prompt_count = fs_err::read_dir(&prompts_dir)
        .map(|it| {
            it.filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                })
                .count()
        })
        .expect("resources/modules/prompts/ must exist after T-002");
    assert!(
        persona_count >= 1,
        "resources/modules/personas/ must contain at least one .md file; got {persona_count}",
    );
    assert!(
        prompt_count >= 1,
        "resources/modules/prompts/ must contain at least one .md file; got {prompt_count}",
    );
}

// --------------------------------------------------------------------
// SPEC-0016 T-011 / CHK-007: divergence-block guard and rendered-output
// shape for `speccy-review`.
//
// Step 4 of the speccy-review module body lives on a
// `{% if host == "claude-code" %}` / `{% else %}` / `{% endif %}`
// triple so the rendered `/speccy-review` skill picks the host-native
// subagent primitive (Claude Code's `Task` tool with `subagent_type`;
// Codex's prose-spawn by name). Both rendered outputs additionally
// carry an explicit `speccy review T-NNN --persona X` fallback for
// harnesses that don't recognise the subagent type.
// --------------------------------------------------------------------

/// Embedded copy of the `speccy-review` module body, used by the
/// T-011 source-shape guard below. Kept as a single `include_str!`
/// constant rather than a lookup table because only the one verb
/// is checked.
const SPECCY_REVIEW_MODULE_BODY: &str =
    include_str!("../../resources/modules/skills/speccy-review.md");

/// Default reviewer fan-out used by both `/speccy-review` rendered
/// branches: the four personas Speccy invokes per task. Other shipped
/// reviewers (`architecture`, `docs`) are explicit-only.
const DEFAULT_REVIEWER_PERSONAS: &[&str] = &["business", "tests", "security", "style"];

#[test]
fn t011_speccy_review_module_has_host_divergence_block() {
    // Source-shape guard: the module body must carry the canonical
    // `{% if host == "claude-code" %}` / `{% else %}` / `{% endif %}`
    // triple so the renderer (and any future contributor reading the
    // source) sees the same syntax.
    let body = SPECCY_REVIEW_MODULE_BODY;
    assert!(
        body.contains("{% if host == \"claude-code\" %}"),
        "`speccy-review.md` must contain a `{{% if host == \"claude-code\" %}}` block (T-011)",
    );
    assert!(
        body.contains("{% else %}"),
        "`speccy-review.md` must contain an `{{% else %}}` branch (T-011)",
    );
    assert!(
        body.contains("{% endif %}"),
        "`speccy-review.md` must close the divergence block with `{{% endif %}}` (T-011)",
    );
}

#[test]
fn speccy_review_skill_prefers_native_subagents() {
    // CHK-007: render once per host, then assert step 4 picks the
    // host-native subagent primitive and that both rendered outputs
    // carry the explicit `speccy review ... --persona X` fallback.

    let claude = render_host_pack(HostChoice::ClaudeCode)
        .expect("render_host_pack(claude-code) should succeed");
    let claude_body = find_rendered_skill(&claude, ".claude", "speccy-review");

    // Claude Code branch: step 4 must invoke the `Task` tool with the
    // host-native `subagent_type` for each default persona.
    assert!(
        claude_body.contains("subagent_type: \"reviewer-"),
        "rendered Claude Code `speccy-review` SKILL.md must reference `subagent_type: \"reviewer-` in step 4; got:\n{claude_body}",
    );
    for persona in DEFAULT_REVIEWER_PERSONAS {
        let needle = format!("subagent_type: \"reviewer-{persona}\"");
        assert!(
            claude_body.contains(&needle),
            "rendered Claude Code `speccy-review` SKILL.md must name persona `{persona}` as `{needle}`; got:\n{claude_body}",
        );
    }
    // Negative: the Codex prose-spawn wording must not leak into the
    // Claude Code render.
    assert!(
        !claude_body.contains("Prose-spawn the four reviewer subagents"),
        "rendered Claude Code `speccy-review` SKILL.md must not contain the Codex prose-spawn wording; got:\n{claude_body}",
    );

    let codex =
        render_host_pack(HostChoice::Codex).expect("render_host_pack(codex) should succeed");
    let codex_body = find_rendered_skill(&codex, ".agents", "speccy-review");

    // Codex branch: step 4 must not mention `subagent_type:` (a
    // Claude-Code-specific key), and must instead reference the four
    // default reviewer subagents by name in prose.
    assert!(
        !codex_body.contains("subagent_type:"),
        "rendered Codex `speccy-review` SKILL.md must not contain `subagent_type:` (Claude-Code-only key); got:\n{codex_body}",
    );
    for persona in DEFAULT_REVIEWER_PERSONAS {
        let needle = format!("`reviewer-{persona}`");
        assert!(
            codex_body.contains(&needle),
            "rendered Codex `speccy-review` SKILL.md must name persona `{persona}` in prose as `{needle}`; got:\n{codex_body}",
        );
    }

    // Both rendered outputs must carry the bash command form
    // `speccy review <SPEC-NNNN/T-NNN> --persona <persona>` as the
    // payload the spawned sub-agent runs. SPEC-0023 REQ-002 retired the
    // per-persona "explicit fallback example" requirement — the spawn
    // prompt now uses placeholders that the orchestrator fills in
    // when invoking the sub-agent — but `speccy review` and
    // `--persona` must still appear so the CLI path is one search
    // away. Persona-by-name presence is enforced above via
    // `subagent_type:` (Claude) and `reviewer-<persona>` prose
    // (Codex).
    for (label, body) in [
        (
            "claude-code .claude/skills/speccy-review/SKILL.md",
            claude_body,
        ),
        ("codex .agents/skills/speccy-review/SKILL.md", codex_body),
    ] {
        assert!(
            body.contains("speccy review"),
            "rendered `{label}` must contain the literal `speccy review` CLI command; got:\n{body}",
        );
        assert!(
            body.contains("--persona "),
            "rendered `{label}` must show a `--persona <persona>` example in the spawn prompt; got:\n{body}",
        );
    }
}

// --------------------------------------------------------------------
// SPEC-0016 T-008: the legacy `skills/` tree has been deleted; this
// guard makes its absence loud.
// --------------------------------------------------------------------

#[test]
fn t008_legacy_skills_tree_is_gone() {
    let root = workspace_root();
    let legacy = root.join("skills");
    assert!(
        !legacy.exists(),
        "after T-008, the legacy `skills/` tree must not exist on disk \
         (per-host wrappers now live under `resources/agents/<install_root>/skills/`); \
         found at {}",
        legacy.display(),
    );
}

// --------------------------------------------------------------------
// SPEC-0016 T-005: Claude Code SKILL.md wrappers under
// `resources/agents/.claude/skills/speccy-<verb>/SKILL.md.tmpl`.
//
// These wrappers are thin: a YAML frontmatter block (`name`,
// `description`) followed by exactly one
// `{% raw %}{% include "modules/skills/speccy-<verb>.md" %}{% endraw %}`
// directive. The renderer-wiring task (T-007) materialises them via
// MiniJinja; here we only validate the on-disk shape since the
// `RESOURCES`-backed embed doesn't exist yet.
// --------------------------------------------------------------------

/// Directory under the workspace root that holds the Claude Code
/// SKILL.md wrappers. Resolved via `CARGO_MANIFEST_DIR` so the test
/// is hermetic.
fn t005_claude_skills_dir() -> std::path::PathBuf {
    workspace_root()
        .join("resources")
        .join("agents")
        .join(".claude")
        .join("skills")
}

#[test]
fn t005_claude_code_skill_wrappers_exactly_seven() {
    let dir = t005_claude_skills_dir();
    let mut found: Vec<String> = Vec::new();
    let entries =
        fs_err::read_dir(&dir).expect("resources/agents/.claude/skills/ must exist after T-005");
    for entry in entries {
        let entry = entry.expect("read_dir entry should be readable");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let tmpl = path.join("SKILL.md.tmpl");
        if tmpl.is_file() {
            let stem = path
                .file_name()
                .and_then(|s| s.to_str())
                .expect("skill directory name must be valid UTF-8")
                .to_owned();
            found.push(stem);
        }
    }
    found.sort();
    let mut expected: Vec<String> = SKILL_NAMES.iter().map(|s| (*s).to_owned()).collect();
    expected.sort();
    assert_eq!(
        found, expected,
        "exactly seven Claude Code SKILL.md.tmpl wrappers must exist, one per shipped verb",
    );
}

/// `RecipeFrontmatter` is the existing serde-saphyr target for legacy
/// SKILL.md files. T-005 reuses it; the YAML shape (`name`,
/// `description`) is identical.
#[test]
fn t005_claude_code_wrapper_shape_and_body() {
    let dir = t005_claude_skills_dir();
    for verb in SKILL_NAMES {
        let path = dir.join(verb).join("SKILL.md.tmpl");
        let contents = fs_err::read_to_string(&path).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "Claude Code wrapper `{}` must exist and be readable: {err}",
                path.display()
            ))
        });
        let (yaml, body) = split_frontmatter(&contents).unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "wrapper `{}` must have a `---` frontmatter fence",
                path.display()
            ))
        });

        let fm: RecipeFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "wrapper `{}` frontmatter must parse as YAML: {err}",
                path.display()
            ))
        });
        let name = fm.name.as_deref().unwrap_or("");
        assert_eq!(
            name,
            *verb,
            "wrapper `{}` `name` field must equal `{verb}`",
            path.display(),
        );
        assert!(
            !fm.description.trim().is_empty(),
            "wrapper `{}` `description` must be non-empty",
            path.display(),
        );
        assert!(
            !fm.description.contains('\n'),
            "wrapper `{}` `description` must be a single line",
            path.display(),
        );

        let expected_body = format!("{{% include \"modules/skills/{verb}.md\" %}}");
        assert_eq!(
            body.trim(),
            expected_body,
            "wrapper `{}` body (post-frontmatter, trimmed) must be exactly the single `{{% include %}}` directive for the module body",
            path.display(),
        );
    }
}

// --------------------------------------------------------------------
// SPEC-0016 T-006: Codex SKILL.md wrappers under
// `resources/agents/.agents/skills/speccy-<verb>/SKILL.md.tmpl`.
//
// Structurally identical to the T-005 Claude Code wrappers: a YAML
// frontmatter block (`name`, `description`) followed by exactly one
// `{% raw %}{% include "modules/skills/speccy-<verb>.md" %}{% endraw %}`
// directive. The `.agents/skills/` path mirrors the Codex install
// destination established by SPEC-0015 (OpenAI's documented
// project-local scan path), not `.codex/skills/`.
// --------------------------------------------------------------------

/// Directory under the workspace root that holds the Codex SKILL.md
/// wrappers. Resolved via `CARGO_MANIFEST_DIR` so the test is
/// hermetic.
fn t006_codex_skills_dir() -> std::path::PathBuf {
    workspace_root()
        .join("resources")
        .join("agents")
        .join(".agents")
        .join("skills")
}

#[test]
fn t006_codex_skill_wrappers_exactly_seven() {
    let dir = t006_codex_skills_dir();
    let mut found: Vec<String> = Vec::new();
    let entries =
        fs_err::read_dir(&dir).expect("resources/agents/.agents/skills/ must exist after T-006");
    for entry in entries {
        let entry = entry.expect("read_dir entry should be readable");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let tmpl = path.join("SKILL.md.tmpl");
        if tmpl.is_file() {
            let stem = path
                .file_name()
                .and_then(|s| s.to_str())
                .expect("skill directory name must be valid UTF-8")
                .to_owned();
            found.push(stem);
        }
    }
    found.sort();
    let mut expected: Vec<String> = SKILL_NAMES.iter().map(|s| (*s).to_owned()).collect();
    expected.sort();
    assert_eq!(
        found, expected,
        "exactly seven Codex SKILL.md.tmpl wrappers must exist, one per shipped verb",
    );
}

#[test]
fn t006_codex_wrapper_shape_and_body() {
    let dir = t006_codex_skills_dir();
    for verb in SKILL_NAMES {
        let path = dir.join(verb).join("SKILL.md.tmpl");
        let contents = fs_err::read_to_string(&path).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "Codex wrapper `{}` must exist and be readable: {err}",
                path.display()
            ))
        });
        let (yaml, body) = split_frontmatter(&contents).unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "wrapper `{}` must have a `---` frontmatter fence",
                path.display()
            ))
        });

        let fm: RecipeFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "wrapper `{}` frontmatter must parse as YAML: {err}",
                path.display()
            ))
        });
        let name = fm.name.as_deref().unwrap_or("");
        assert_eq!(
            name,
            *verb,
            "wrapper `{}` `name` field must equal `{verb}`",
            path.display(),
        );
        assert!(
            !fm.description.trim().is_empty(),
            "wrapper `{}` `description` must be non-empty",
            path.display(),
        );
        assert!(
            !fm.description.contains('\n'),
            "wrapper `{}` `description` must be a single line",
            path.display(),
        );

        let expected_body = format!("{{% include \"modules/skills/{verb}.md\" %}}");
        assert_eq!(
            body.trim(),
            expected_body,
            "wrapper `{}` body (post-frontmatter, trimmed) must be exactly the single `{{% include %}}` directive for the module body",
            path.display(),
        );
    }
}

// --------------------------------------------------------------------
// SPEC-0016 T-009: Claude Code reviewer subagent wrappers under
// `resources/agents/.claude/agents/reviewer-<persona>.md.tmpl`.
//
// Six wrappers, one per shipped reviewer persona. Each wrapper is a
// YAML frontmatter block (`name: reviewer-<persona>`,
// `description: <one-line>`) followed by exactly one
// `{% include "modules/personas/reviewer-<persona>.md" %}` directive
// (no `{% raw %}` wrapping; persona bodies currently contain no
// `{{` / `{%` literals, and SPEC-0016 DEC-004's TOML-safety invariant
// test lands in T-010). The wrapper byte-shape mirrors the T-005
// SKILL.md wrappers: ends at `%}` with no trailing newline so the
// rendered output keeps the persona body's leading/trailing newlines
// as the only blank lines straddling the include site.
// --------------------------------------------------------------------

/// Six reviewer-persona names shipped by `speccy-core::personas::ALL`.
/// Duplicated locally as a `const &[&str]` so the T-009 tests stay
/// hermetic w.r.t. `personas::ALL`'s declared order.
const REVIEWER_PERSONAS: &[&str] = &[
    "business",
    "tests",
    "security",
    "style",
    "architecture",
    "docs",
];

fn t009_claude_agents_dir() -> std::path::PathBuf {
    workspace_root()
        .join("resources")
        .join("agents")
        .join(".claude")
        .join("agents")
}

#[test]
fn t009_claude_code_reviewer_wrappers_exactly_six() {
    let dir = t009_claude_agents_dir();
    let mut found: Vec<String> = Vec::new();
    let entries =
        fs_err::read_dir(&dir).expect("resources/agents/.claude/agents/ must exist after T-009");
    for entry in entries {
        let entry = entry.expect("read_dir entry should be readable");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .expect("wrapper file name must be valid UTF-8");
        let Some(stem) = name
            .strip_prefix("reviewer-")
            .and_then(|s| s.strip_suffix(".md.tmpl"))
        else {
            continue;
        };
        found.push(stem.to_owned());
    }
    found.sort();
    let mut expected: Vec<String> = REVIEWER_PERSONAS.iter().map(|s| (*s).to_owned()).collect();
    expected.sort();
    assert_eq!(
        found, expected,
        "exactly six Claude Code reviewer wrappers must exist, one per shipped reviewer persona",
    );
}

#[test]
fn t009_claude_code_reviewer_wrapper_shape_and_body() {
    let dir = t009_claude_agents_dir();
    for persona in REVIEWER_PERSONAS {
        let path = dir.join(format!("reviewer-{persona}.md.tmpl"));
        let contents = fs_err::read_to_string(&path).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "Claude Code reviewer wrapper `{}` must exist and be readable: {err}",
                path.display()
            ))
        });
        let (yaml, body) = split_frontmatter(&contents).unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "wrapper `{}` must have a `---` frontmatter fence",
                path.display()
            ))
        });

        let fm: RecipeFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "wrapper `{}` frontmatter must parse as YAML: {err}",
                path.display()
            ))
        });
        let name = fm.name.as_deref().unwrap_or("");
        let expected_name = format!("reviewer-{persona}");
        assert_eq!(
            name,
            expected_name,
            "wrapper `{}` `name` field must equal `{expected_name}`",
            path.display(),
        );
        assert!(
            !fm.description.trim().is_empty(),
            "wrapper `{}` `description` must be non-empty",
            path.display(),
        );
        assert!(
            !fm.description.contains('\n'),
            "wrapper `{}` `description` must be a single line",
            path.display(),
        );
        // SPEC-0015 description-length invariant: descriptions land in
        // Claude Code's subagent registry; the SKILL.md descriptions
        // already honour the ~500-char cap and the subagent descriptions
        // should too.
        let len = fm.description.chars().count();
        assert!(
            len <= 500,
            "wrapper `{}` description must be at most 500 characters; got {len}",
            path.display(),
        );

        let expected_body = format!("{{% include \"modules/personas/reviewer-{persona}.md\" %}}");
        assert_eq!(
            body.trim(),
            expected_body,
            "wrapper `{}` body (post-frontmatter, trimmed) must be exactly the single `{{% include %}}` directive for the persona body",
            path.display(),
        );
    }
}

#[test]
fn t009_claude_code_reviewer_wrappers_render_to_subagent_files() {
    // The renderer walks `agents/.claude/` recursively (see
    // `render::render_host_pack`), so adding wrappers under
    // `agents/.claude/agents/` produces additional `RenderedFile`
    // entries with `rel_path` rooted at `.claude/agents/`. This test
    // exercises the rendered-output shape: count, frontmatter name
    // equals filename stem, and (for the security persona) the body
    // carries the documented focus bullet drawn verbatim from the
    // persona module file.
    let rendered = render_host_pack(HostChoice::ClaudeCode)
        .expect("render_host_pack(claude-code) should succeed");
    let agent_files: Vec<&speccy_cli::render::RenderedFile> = rendered
        .iter()
        .filter(|f| f.rel_path.as_str().starts_with(".claude/agents/"))
        .collect();
    assert_eq!(
        agent_files.len(),
        6,
        "claude-code host pack should render six reviewer subagent files under .claude/agents/; got {}",
        agent_files.len(),
    );

    for file in &agent_files {
        let path = file.rel_path.as_str();
        let stem = path
            .strip_prefix(".claude/agents/")
            .and_then(|s| s.strip_suffix(".md"))
            .unwrap_or_else(|| {
                panic_with_test_message(&format!(
                    "rendered subagent file `{path}` must be of the form `.claude/agents/<stem>.md`"
                ))
            });
        assert!(
            file.contents.starts_with("---\n") || file.contents.starts_with("---\r\n"),
            "rendered subagent file `{path}` must open with a `---` YAML frontmatter fence",
        );
        let (yaml, body) = split_frontmatter(&file.contents).unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "rendered subagent file `{path}` must have a `---` frontmatter fence"
            ))
        });
        let fm: RecipeFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "rendered subagent file `{path}` frontmatter must parse as YAML: {err}"
            ))
        });
        let name = fm.name.as_deref().unwrap_or("");
        assert_eq!(
            name, stem,
            "rendered subagent `{path}` `name` field must equal its filename stem `{stem}`",
        );
        assert!(
            !fm.description.trim().is_empty(),
            "rendered subagent `{path}` `description` must be non-empty",
        );
        assert!(
            !body.trim().is_empty(),
            "rendered subagent `{path}` body (post-frontmatter) must be non-empty",
        );
    }

    // SPEC-0016 REQ-003 / TASKS.md T-009 obligation: the security
    // reviewer subagent's body must carry the focus bullet drawn
    // verbatim from `resources/modules/personas/reviewer-security.md`.
    let security = agent_files
        .iter()
        .find(|f| f.rel_path.as_str() == ".claude/agents/reviewer-security.md")
        .expect("rendered output must include the security reviewer subagent");
    assert!(
        security
            .contents
            .contains("Authentication and authorization boundaries"),
        "rendered .claude/agents/reviewer-security.md must contain the focus bullet drawn from the persona body; got:\n{}",
        security.contents,
    );
}

// --------------------------------------------------------------------
// SPEC-0016 T-010: Codex reviewer subagent wrappers under
// `resources/agents/.codex/agents/reviewer-<persona>.toml.tmpl`.
//
// Six wrappers, one per shipped reviewer persona. Each wrapper is a
// flat-TOML document with three top-level keys: `name` (string),
// `description` (string), and `developer_instructions` (string, TOML
// triple-quoted) wrapping a single
// `{% include "modules/personas/reviewer-<persona>.md" %}` directive
// (no `{% raw %}` wrapping, mirroring T-009; persona bodies currently
// contain no `{{` / `{%` literals). The wrapper byte-shape mirrors the
// T-005 / T-006 / T-009 wrappers: ends at `"""` (the close of the
// triple-quoted block) with no trailing newline. SPEC-0016 DEC-004
// invariant: persona bodies must not contain the literal substring
// `"""` because it would terminate the TOML triple-quoted string
// prematurely; the `t010_persona_bodies_have_no_toml_triple_quote`
// test enforces this going forward.
// --------------------------------------------------------------------

fn t010_codex_agents_dir() -> std::path::PathBuf {
    workspace_root()
        .join("resources")
        .join("agents")
        .join(".codex")
        .join("agents")
}

#[test]
fn t010_codex_reviewer_wrappers_exactly_six() {
    let dir = t010_codex_agents_dir();
    let mut found: Vec<String> = Vec::new();
    let entries =
        fs_err::read_dir(&dir).expect("resources/agents/.codex/agents/ must exist after T-010");
    for entry in entries {
        let entry = entry.expect("read_dir entry should be readable");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .expect("wrapper file name must be valid UTF-8");
        let Some(stem) = name
            .strip_prefix("reviewer-")
            .and_then(|s| s.strip_suffix(".toml.tmpl"))
        else {
            continue;
        };
        found.push(stem.to_owned());
    }
    found.sort();
    let mut expected: Vec<String> = REVIEWER_PERSONAS.iter().map(|s| (*s).to_owned()).collect();
    expected.sort();
    assert_eq!(
        found, expected,
        "exactly six Codex reviewer wrappers must exist, one per shipped reviewer persona",
    );
}

#[test]
fn t010_codex_reviewer_wrapper_shape_and_body() {
    let dir = t010_codex_agents_dir();
    for persona in REVIEWER_PERSONAS {
        let path = dir.join(format!("reviewer-{persona}.toml.tmpl"));
        let contents = fs_err::read_to_string(&path).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "Codex reviewer wrapper `{}` must exist and be readable: {err}",
                path.display()
            ))
        });

        // Top-level key presence. The wrapper isn't valid TOML yet
        // (the `{% include %}` directive is unexpanded), so we
        // string-search rather than parse here.
        let expected_name = format!("name = \"reviewer-{persona}\"");
        assert!(
            contents.contains(&expected_name),
            "wrapper `{}` must set `{expected_name}` at the top level",
            path.display(),
        );
        assert!(
            contents.contains("description = "),
            "wrapper `{}` must set a `description` key at the top level",
            path.display(),
        );
        assert!(
            contents.contains("developer_instructions = \"\"\""),
            "wrapper `{}` must set `developer_instructions` as a TOML triple-quoted string",
            path.display(),
        );

        // The triple-quoted body must wrap the include directive for
        // the matching persona, with the closing `\"\"\"` as the final
        // bytes of the file (no trailing newline) to mirror the
        // T-005/T-006/T-009 wrapper trailing-byte shape.
        let expected_include =
            format!("{{% include \"modules/personas/reviewer-{persona}.md\" %}}");
        assert!(
            contents.contains(&expected_include),
            "wrapper `{}` must contain the include directive `{expected_include}` inside `developer_instructions`",
            path.display(),
        );
        assert!(
            contents.ends_with("\"\"\""),
            "wrapper `{}` must end with the closing `\"\"\"` of `developer_instructions` (no trailing newline); last 16 bytes: {:?}",
            path.display(),
            &contents
                .get(contents.len().saturating_sub(16)..)
                .unwrap_or(""),
        );
    }
}

#[test]
fn t010_codex_reviewer_wrappers_render_to_subagent_files() {
    // The renderer walks `agents/.codex/` recursively (see
    // `render::render_host_pack`), so adding wrappers under
    // `agents/.codex/agents/` produces additional `RenderedFile`
    // entries with `rel_path` rooted at `.codex/agents/`. This test
    // exercises the rendered-output shape: count, parse-as-TOML, three
    // required top-level keys, name equals filename stem, and the
    // security reviewer carries the focus bullet drawn verbatim from
    // the persona module file.
    let rendered =
        render_host_pack(HostChoice::Codex).expect("render_host_pack(codex) should succeed");
    let agent_files: Vec<&speccy_cli::render::RenderedFile> = rendered
        .iter()
        .filter(|f| f.rel_path.as_str().starts_with(".codex/agents/"))
        .collect();
    assert_eq!(
        agent_files.len(),
        6,
        "codex host pack should render six reviewer subagent files under .codex/agents/; got {}",
        agent_files.len(),
    );

    for file in &agent_files {
        let path = file.rel_path.as_str();
        let stem = path
            .strip_prefix(".codex/agents/")
            .and_then(|s| s.strip_suffix(".toml"))
            .unwrap_or_else(|| {
                panic_with_test_message(&format!(
                    "rendered subagent file `{path}` must be of the form `.codex/agents/<stem>.toml`"
                ))
            });

        // Parse the rendered output as TOML. The wrapper is flat
        // (three top-level keys), so a `toml::Value` table is the
        // right shape.
        let parsed: toml::Value = toml::from_str(&file.contents).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "rendered subagent `{path}` must parse as TOML: {err}\ncontents:\n{}",
                file.contents
            ))
        });
        let table = parsed.as_table().unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "rendered subagent `{path}` must be a top-level TOML table"
            ))
        });

        let name = table
            .get("name")
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| {
                panic_with_test_message(&format!(
                    "rendered subagent `{path}` must have a string `name` key"
                ))
            });
        assert_eq!(
            name, stem,
            "rendered subagent `{path}` `name` field must equal its filename stem `{stem}`",
        );

        let description = table
            .get("description")
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| {
                panic_with_test_message(&format!(
                    "rendered subagent `{path}` must have a string `description` key"
                ))
            });
        assert!(
            !description.trim().is_empty(),
            "rendered subagent `{path}` `description` must be non-empty",
        );

        let dev_instructions = table
            .get("developer_instructions")
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| {
                panic_with_test_message(&format!(
                    "rendered subagent `{path}` must have a string `developer_instructions` key"
                ))
            });
        assert!(
            !dev_instructions.trim().is_empty(),
            "rendered subagent `{path}` `developer_instructions` must be non-empty",
        );
    }

    // SPEC-0016 REQ-003 / TASKS.md T-010 obligation: the security
    // reviewer subagent's `developer_instructions` body must carry the
    // focus bullet drawn verbatim from
    // `resources/modules/personas/reviewer-security.md`.
    let security = agent_files
        .iter()
        .find(|f| f.rel_path.as_str() == ".codex/agents/reviewer-security.toml")
        .expect("rendered output must include the security reviewer subagent");
    let security_parsed: toml::Value = toml::from_str(&security.contents)
        .expect("rendered reviewer-security.toml must parse as TOML");
    let security_table = security_parsed
        .as_table()
        .expect("rendered reviewer-security.toml must be a top-level table");
    let security_name = security_table
        .get("name")
        .and_then(toml::Value::as_str)
        .expect("rendered reviewer-security.toml must have a string `name` key");
    assert_eq!(
        security_name, "reviewer-security",
        "rendered reviewer-security.toml `name` must equal `reviewer-security`",
    );
    let security_dev = security_table
        .get("developer_instructions")
        .and_then(toml::Value::as_str)
        .expect("rendered reviewer-security.toml must have a string `developer_instructions` key");
    assert!(
        security_dev.contains("Authentication and authorization boundaries"),
        "rendered .codex/agents/reviewer-security.toml `developer_instructions` must contain the focus bullet drawn from the persona body; got:\n{security_dev}",
    );
}

/// SPEC-0016 DEC-004 invariant: persona body files must not contain
/// the literal substring `"""` because the Codex reviewer wrapper
/// embeds the persona body inside a TOML triple-quoted string
/// (`developer_instructions = """..."""`); a `"""` in the persona body
/// would terminate the string prematurely and break the rendered TOML.
/// This guard lives long-term (not transient like T-003's
/// byte-equivalence oracle): the invariant must hold for every future
/// persona edit.
#[test]
fn t010_persona_bodies_have_no_toml_triple_quote() {
    let dir = workspace_root()
        .join("resources")
        .join("modules")
        .join("personas");
    let entries = fs_err::read_dir(&dir)
        .expect("resources/modules/personas/ must exist (SPEC-0016 T-002 layout)");
    let mut checked = 0_u32;
    for entry in entries {
        let entry = entry.expect("read_dir entry should be readable");
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_md = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
        if !is_md {
            continue;
        }
        let body = fs_err::read_to_string(&path).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "persona body `{}` must be readable: {err}",
                path.display()
            ))
        });
        assert!(
            !body.contains("\"\"\""),
            "persona body `{}` contains the literal substring `\"\"\"`, which would terminate the Codex \
             reviewer wrapper's TOML triple-quoted `developer_instructions` block prematurely (SPEC-0016 DEC-004). \
             Remove or escape the substring before committing.",
            path.display(),
        );
        checked = checked.saturating_add(1);
    }
    assert!(
        checked >= 1,
        "expected at least one persona body to be checked under {}; found none",
        dir.display(),
    );
}
