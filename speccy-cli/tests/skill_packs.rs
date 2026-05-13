#![expect(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Skill-pack content tests for SPEC-0013.
//!
//! Each test maps to one CHK-NNN in
//! `.speccy/specs/0013-skill-packs/spec.toml`:
//!
//! - CHK-001: [`persona_files_present`]
//! - CHK-002: [`persona_names_match_registry`]
//! - CHK-003: [`prompt_templates_present`]
//! - CHK-004: [`prompt_placeholders_match_commands`]
//! - CHK-005: [`claude_code_recipes`]
//! - CHK-006: [`codex_recipes`]
//! - CHK-007: [`persona_content_shape`]
//! - CHK-008: [`recipe_content_shape`]

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
    "speccy-init.md",
    "speccy-plan.md",
    "speccy-tasks.md",
    "speccy-work.md",
    "speccy-review.md",
    "speccy-amend.md",
    "speccy-ship.md",
];

const LOOP_RECIPES: &[&str] = &["speccy-work.md", "speccy-review.md", "speccy-amend.md"];

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
        assert_recipe_frontmatter("claude-code", name, false);
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
