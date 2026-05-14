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

use include_dir::Dir;
use serde::Deserialize;
use speccy_cli::embedded::SKILLS;
use speccy_core::personas;

// --------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------

fn bundle_dir(sub: &str) -> &'static Dir<'static> {
    SKILLS
        .get_dir(sub)
        .expect("embedded skill bundle should contain the requested sub-path")
}

fn read_bundle_file(sub: &str, name: &str) -> &'static str {
    let dir = bundle_dir(sub);
    let path = format!("{sub}/{name}");
    let entry = dir.get_file(&path).unwrap_or_else(|| {
        panic_with_test_message(&format!("embedded bundle should contain `{path}`"))
    });
    entry
        .contents_utf8()
        .expect("embedded bundle entries should be valid UTF-8")
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

const RECIPE_FILES: &[&str] = &[
    "speccy-init/SKILL.md",
    "speccy-plan/SKILL.md",
    "speccy-tasks/SKILL.md",
    "speccy-work/SKILL.md",
    "speccy-review/SKILL.md",
    "speccy-amend/SKILL.md",
    "speccy-ship/SKILL.md",
];

const LOOP_RECIPES: &[&str] = &[
    "speccy-work/SKILL.md",
    "speccy-review/SKILL.md",
    "speccy-amend/SKILL.md",
];

const SKILL_NAMES: &[&str] = &[
    "speccy-init",
    "speccy-plan",
    "speccy-tasks",
    "speccy-work",
    "speccy-review",
    "speccy-ship",
    "speccy-amend",
];

const HOSTS: &[&str] = &["claude-code", "codex"];

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
        let body = read_bundle_file("shared/personas", name);
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
        let dir = bundle_dir("shared/personas");
        let path = format!("shared/personas/{file_name}");
        assert!(
            dir.get_file(&path).is_some(),
            "personas::ALL contains `{persona}` but `{file_name}` is missing from the bundle",
        );
    }
    for required in ["planner.md", "implementer.md"] {
        let path = format!("shared/personas/{required}");
        assert!(
            bundle_dir("shared/personas").get_file(&path).is_some(),
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
        let body = read_bundle_file("shared/prompts", name);
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
    let plan_greenfield = read_bundle_file("shared/prompts", "plan-greenfield.md");
    assert_placeholders(
        plan_greenfield,
        &["vision", "agents", "next_spec_id"],
        "plan-greenfield.md",
    );

    let plan_amend = read_bundle_file("shared/prompts", "plan-amend.md");
    assert_placeholders(
        plan_amend,
        &["spec_id", "spec_md", "agents"],
        "plan-amend.md",
    );

    let tasks_generate = read_bundle_file("shared/prompts", "tasks-generate.md");
    assert_placeholders(
        tasks_generate,
        &["spec_id", "spec_md", "agents"],
        "tasks-generate.md",
    );

    let tasks_amend = read_bundle_file("shared/prompts", "tasks-amend.md");
    assert_placeholders(
        tasks_amend,
        &["spec_id", "spec_md", "tasks_md", "agents"],
        "tasks-amend.md",
    );

    let implementer = read_bundle_file("shared/prompts", "implementer.md");
    assert_placeholders(
        implementer,
        &[
            "spec_id",
            "spec_md",
            "task_id",
            "task_entry",
            "suggested_files",
            "agents",
        ],
        "implementer.md",
    );

    let reviewer_required = &[
        "spec_id",
        "spec_md",
        "task_id",
        "task_entry",
        "diff",
        "persona",
        "persona_content",
        "agents",
    ];
    for persona in personas::ALL {
        let file = format!("reviewer-{persona}.md");
        let body = read_bundle_file("shared/prompts", &file);
        assert_placeholders(body, reviewer_required, &file);
    }

    let report = read_bundle_file("shared/prompts", "report.md");
    assert_placeholders(
        report,
        &["spec_id", "spec_md", "tasks_md", "retry_summary", "agents"],
        "report.md",
    );

    // Negative: an obvious typo must not appear in any template.
    let typo = "{{spec_idd}}";
    for name in PROMPT_FILES {
        let body = read_bundle_file("shared/prompts", name);
        assert!(
            !body.contains(typo),
            "template `{name}` must not contain placeholder typo `{typo}`",
        );
    }
}

// --------------------------------------------------------------------
// CHK-005 / CHK-006: recipe frontmatter
// --------------------------------------------------------------------

fn assert_recipe_frontmatter(sub: &str, file_name: &str, require_name: bool) {
    let body = read_bundle_file(sub, file_name);
    let (yaml, _rest) = split_frontmatter(body).unwrap_or_else(|| {
        panic_with_test_message(&format!(
            "recipe `{sub}/{file_name}` must have a `---` frontmatter fence"
        ))
    });

    let fm: RecipeFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
        panic_with_test_message(&format!(
            "recipe `{sub}/{file_name}` frontmatter must be valid YAML: {err}"
        ))
    });

    assert!(
        !fm.description.trim().is_empty(),
        "recipe `{sub}/{file_name}` `description` field must be non-empty",
    );

    if require_name {
        let name = fm.name.as_deref().unwrap_or("");
        assert!(
            !name.trim().is_empty(),
            "recipe `{sub}/{file_name}` `name` field is required for Codex",
        );
    }
}

#[test]
fn claude_code_recipes() {
    for name in RECIPE_FILES {
        assert_recipe_frontmatter("claude-code", name, true);
    }
}

#[test]
fn codex_recipes() {
    for name in RECIPE_FILES {
        assert_recipe_frontmatter("codex", name, true);
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
        let body = read_bundle_file("shared/personas", &file);
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
    for sub in ["claude-code", "codex"] {
        for name in RECIPE_FILES {
            let body = read_bundle_file(sub, name);

            assert!(
                first_non_frontmatter_paragraph(body).is_some(),
                "recipe `{sub}/{name}` must include an intro paragraph after the title",
            );

            assert!(
                body.contains("## When to use"),
                "recipe `{sub}/{name}` must contain a `## When to use` section",
            );

            assert!(
                contains_speccy_command_in_code_fence(body),
                "recipe `{sub}/{name}` must contain a fenced code block with a v1 `speccy ...` command",
            );

            if LOOP_RECIPES.contains(name) {
                let lower = body.to_lowercase();
                assert!(
                    lower.contains("loop exit") || lower.contains("exit criteria"),
                    "loop recipe `{sub}/{name}` must declare explicit loop exit criteria",
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
    let body = read_bundle_file("shared/prompts", "implementer.md");
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
    let body = read_bundle_file("shared/prompts", "implementer.md");
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
    let body = read_bundle_file("shared/prompts", "implementer.md");
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
    let body = read_bundle_file("shared/personas", "implementer.md");
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
    let body = read_bundle_file("shared/prompts", "report.md");

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
    for host in HOSTS {
        for skill in SKILL_NAMES {
            let path = format!("{host}/{skill}/SKILL.md");
            let entry = SKILLS.get_file(&path).unwrap_or_else(|| {
                panic_with_test_message(&format!(
                    "embedded bundle must contain `{path}` (SPEC-0015 REQ-001 + CHK-001)"
                ))
            });
            let body = entry
                .contents_utf8()
                .expect("SKILL.md entries must be valid UTF-8");
            assert!(!body.trim().is_empty(), "skill `{path}` must be non-empty");
        }
    }
}

// --------------------------------------------------------------------
// SPEC-0015 CHK-002: legacy flat layout removed from the bundle
// --------------------------------------------------------------------

#[test]
fn bundle_legacy_flat_layout_absent() {
    for host in HOSTS {
        let legacy_dir_path = format!("{host}/speccy");
        assert!(
            SKILLS.get_dir(&legacy_dir_path).is_none(),
            "legacy directory `{legacy_dir_path}` must be gone from the bundle (SPEC-0015 REQ-001 + CHK-002); flat .md files are not Codex-discoverable as skills",
        );
    }
}

// --------------------------------------------------------------------
// SPEC-0015 CHK-005: SKILL.md frontmatter shape (name matches dir,
// description is a single line)
// --------------------------------------------------------------------

#[test]
fn shipped_skill_md_frontmatter_shape() {
    for host in HOSTS {
        for skill in SKILL_NAMES {
            let sub = format!("{host}/{skill}");
            let body = read_bundle_file(&sub, "SKILL.md");
            let (yaml, _rest) = split_frontmatter(body).unwrap_or_else(|| {
                panic_with_test_message(&format!(
                    "`{sub}/SKILL.md` must have a `---` frontmatter fence"
                ))
            });

            let fm: RecipeFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
                panic_with_test_message(&format!(
                    "`{sub}/SKILL.md` frontmatter must be valid YAML: {err}"
                ))
            });

            let name = fm.name.as_deref().unwrap_or("");
            assert!(
                !name.trim().is_empty(),
                "`{sub}/SKILL.md` `name` field is required (SPEC-0015 REQ-003)",
            );
            assert_eq!(
                name, *skill,
                "`{sub}/SKILL.md` `name` field must equal the parent directory `{skill}` (SPEC-0015 REQ-003)",
            );
            assert!(
                !fm.description.trim().is_empty(),
                "`{sub}/SKILL.md` `description` field must be non-empty",
            );
            assert!(
                !fm.description.contains('\n'),
                "`{sub}/SKILL.md` `description` must be a single line (no embedded newlines) so both hosts' YAML loaders agree on its shape",
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
    for host in HOSTS {
        for skill in SKILL_NAMES {
            let sub = format!("{host}/{skill}");
            let body = read_bundle_file(&sub, "SKILL.md");
            let (yaml, _rest) = split_frontmatter(body).unwrap_or_else(|| {
                panic_with_test_message(&format!(
                    "`{sub}/SKILL.md` must have a `---` frontmatter fence"
                ))
            });
            let fm: RecipeFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
                panic_with_test_message(&format!(
                    "`{sub}/SKILL.md` frontmatter must be valid YAML: {err}"
                ))
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
                "`{sub}/SKILL.md` description must not start with `Phase <digit>` jargon (SPEC-0015 REQ-004); got: {desc:?}",
            );

            // Required trigger marker for natural-language activation.
            assert!(
                desc.to_lowercase().contains("use when"),
                "`{sub}/SKILL.md` description must contain a `use when` trigger marker (case-insensitive); got: {desc:?}",
            );

            // Codex caps the skill list at ~2% of the context window, so
            // every description has to stay tight.
            let len = desc.chars().count();
            assert!(
                len <= MAX_DESCRIPTION_CHARS,
                "`{sub}/SKILL.md` description must be at most {MAX_DESCRIPTION_CHARS} characters; got {len}",
            );
        }
    }
}
