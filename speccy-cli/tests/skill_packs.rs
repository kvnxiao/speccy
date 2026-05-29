#![expect(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Skill-pack content tests.
//!
//! - [`persona_files_present`], [`persona_names_match_registry`],
//!   [`persona_content_shape`]: reviewer persona files match the registry and
//!   carry the expected frontmatter / body shape.
//! - [`claude_code_recipes`], [`codex_recipes`], [`recipe_content_shape`]:
//!   shipped recipes exist for both hosts and follow the recipe schema.
//! - [`implementer_persona_friction_reference`],
//!   [`agents_md_friction_paragraph`]: the implementer persona and AGENTS.md
//!   both reference the friction-to-skill-update convention.
//! - [`bundle_layout_has_skill_md_per_host`],
//!   [`shipped_skill_md_frontmatter_shape`],
//!   [`shipped_descriptions_natural_language_triggers`]: per-host wrapper
//!   templates exist with valid frontmatter and natural-language descriptions.

use serde::Deserialize;
use speccy_cli::embedded::RESOURCES;
use speccy_cli::host::HostChoice;
use speccy_cli::render::render_host_pack;
use speccy_core::personas;
use std::path::Path;

// --------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------

/// Read a host SKILL.md wrapper template body from the workspace
/// filesystem at `resources/agents/<install_root>/skills/<verb>/SKILL.md.tmpl`.
/// The wrapper templates carry the `name` / `description` frontmatter
/// the shipped SKILL.md files render to.
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
            panic_with_test_message(&format!("rendered host pack must contain `{needle}`"))
        });
    file.contents.as_str()
}

/// Read a persona body out of the embedded `RESOURCES` bundle by
/// leaf file name (e.g. `"reviewer-security.md"`). Persona bodies are
/// shipped at `resources/modules/personas/<file>` and are reachable
/// through `speccy_cli::embedded::RESOURCES`.
fn read_persona(name: &str) -> &'static str {
    let path = format!("modules/personas/{name}");
    let entry = RESOURCES.get_file(&path).unwrap_or_else(|| {
        panic_with_test_message(&format!("RESOURCES bundle should contain `{path}`"))
    });
    entry
        .contents_utf8()
        .expect("RESOURCES bundle entries should be valid UTF-8")
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
    "reviewer-business.md",
    "reviewer-tests.md",
    "reviewer-security.md",
    "reviewer-style.md",
    "reviewer-architecture.md",
    "reviewer-docs.md",
];

// After SPEC-0023 REQ-001 / REQ-002, `speccy-work` and `speccy-review`
// are single-task primitives — one invocation, one task, exit — and
// no longer declare loop exit criteria. `speccy-amend` is the only
// remaining loop recipe.
const LOOP_RECIPES: &[&str] = &["speccy-amend/SKILL.md"];

const SKILL_NAMES: &[&str] = &[
    "speccy-init",
    "speccy-plan",
    "speccy-decompose",
    "speccy-work",
    "speccy-review",
    "speccy-ship",
    "speccy-amend",
    "speccy-brainstorm",
    "speccy-orchestrate",
    "speccy-vet",
];

/// Per-host install root for the SKILL.md wrappers under
/// `resources/agents/<root>/skills/<verb>/SKILL.md.tmpl`. Mirrors the
/// install destination established by SPEC-0015 (Claude Code →
/// `.claude/`, Codex → `.agents/`); `.codex/` is the subagent root and
/// has no skills bundle.
const HOST_SKILL_ROOTS: &[(&str, &str)] = &[("claude-code", ".claude"), ("codex", ".agents")];

/// The three pinned phase-worker skill verbs whose SKILL.md.tmpl bodies
/// became thin stubs (T-009 / REQ-010). These stubs do not contain a
/// single `{% include %}` directive, do not contain `## When to use`,
/// and do not carry a full `speccy …` command in a code fence — they
/// are pointer-only bodies. The fourth phase (`speccy-init`) keeps its
/// full body sourced from `modules/phases/speccy-init.md` (T-009 scope
/// explicitly excludes it from the stub transformation).
// SPEC-0049 / REQ-003 / DEC-001: `speccy-work` migrated from
// stub-delegate to pure-include shape, so it is no longer a "stub"
// wrapper; only `speccy-decompose` and `speccy-ship` remain in
// stub-delegate form.
const PINNED_STUB_PHASES: &[&str] = &["speccy-decompose", "speccy-ship"];

// The current seven-verb CLI surface. This list is used as a substring
// matcher inside SKILL.md code fences to determine whether a rendered
// skill carries a speccy command.
const SPECCY_COMMANDS: &[&str] = &[
    "speccy init",
    "speccy status",
    "speccy next",
    "speccy check",
    "speccy verify",
    "speccy lock",
    "speccy vacancy",
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
        let path = format!("modules/personas/{file_name}");
        assert!(
            RESOURCES.get_file(&path).is_some(),
            "personas::ALL contains `{persona}` but `{path}` is missing from the embedded RESOURCES bundle",
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
    // The per-host rendered output is the shipped recipe surface. We
    // render once per host and check the rendered SKILL.md body
    // against the content-shape invariants.
    //
    // Exception: the three pinned phase-worker skills (`speccy-decompose`,
    // `speccy-work`, `speccy-ship`) have thin stub bodies. Stubs are
    // pointer-only: they name the matching agent file and
    // `/agent speccy-<phase>` invocation and explicitly do NOT carry
    // `## When to use`, `## Steps`, or a full speccy command in a code
    // fence. The full content-shape checks are skipped for these three
    // verbs.
    for (host, install_root) in [
        (HostChoice::ClaudeCode, ".claude"),
        (HostChoice::Codex, ".agents"),
    ] {
        let rendered = render_host_pack(host).unwrap_or_else(|err| {
            panic_with_test_message(&format!("render_host_pack({host:?}) should succeed: {err}"))
        });
        for verb in SKILL_NAMES {
            let body = find_rendered_skill(&rendered, install_root, verb);

            // Skip full content-shape checks for T-009 stub skills.
            if PINNED_STUB_PHASES.contains(verb) {
                // Stubs must be non-empty and must name the `/agent`
                // invocation pointer — the only content-shape guarantee
                // that applies to them.
                assert!(
                    !body.trim().is_empty(),
                    "stub recipe `{install_root}/skills/{verb}/SKILL.md` must be non-empty",
                );
                assert!(
                    body.contains(&format!("/agent {verb}")),
                    "stub recipe `{install_root}/skills/{verb}/SKILL.md` must contain `/agent {verb}` (T-009 REQ-010)",
                );
                continue;
            }

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

// --------------------------------------------------------------------
// SKILL.md frontmatter shape (name matches dir, description is a
// single line).
// --------------------------------------------------------------------

#[test]
fn shipped_skill_md_frontmatter_shape() {
    // SKILL.md frontmatter lives in the per-host wrapper templates at
    // `resources/agents/<install_root>/skills/<verb>/SKILL.md.tmpl`.
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
    // Codex hard-rejects descriptions over 1024 Unicode chars at skill load
    // (codex-rs/core-skills/src/loader.rs::MAX_DESCRIPTION_LEN; see
    // openai/codex#13941). This is the binding constraint; Claude Code's
    // documented 1536-char cap is softer (truncation). SPEC-0026 DEC-001.
    const MAX_DESCRIPTION_CHARS: usize = 1024;
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

/// Workspace root, derived from `CARGO_MANIFEST_DIR` (the `speccy-cli`
/// crate dir) by walking one level up.
fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        std::path::Path::to_path_buf,
    )
}

#[test]
fn resources_modules_personas_is_non_empty() {
    let root = workspace_root();
    let personas_dir = root.join("resources").join("modules").join("personas");
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
        .expect("resources/modules/personas/ must exist");
    assert!(
        persona_count >= 1,
        "resources/modules/personas/ must contain at least one .md file; got {persona_count}",
    );
}

// --------------------------------------------------------------------
// Divergence-block guard and rendered-output shape for `speccy-review`.
//
// Step 4 of the speccy-review module body lives on a
// `{% if host == "claude-code" %}` / `{% else %}` / `{% endif %}`
// triple so the rendered `/speccy-review` skill picks the host-native
// subagent primitive (Claude Code's `Task` tool with `subagent_type`;
// Codex's native sub-agent-spawn primitive against each registered
// `reviewer-<persona>` sub-agent).
// --------------------------------------------------------------------

/// Embedded copy of the `speccy-review` module body, used by the
/// T-011 source-shape guard below. Kept as a single `include_str!`
/// constant rather than a lookup table because only the one verb
/// is checked.
/// Source of truth for the four-persona fan-out is now the shared
/// partial included by both `speccy-review.md` and
/// `speccy-orchestrate.md`'s review dispatch. The host-divergence
/// block lives there too.
const SPECCY_REVIEW_FANOUT_PARTIAL: &str =
    include_str!("../../resources/modules/skills/partials/review-fanout.md");

/// Default reviewer fan-out used by both `/speccy-review` rendered
/// branches: the five personas Speccy invokes per task. Other shipped
/// reviewers (`architecture`, `docs`) are explicit-only.
const DEFAULT_REVIEWER_PERSONAS: &[&str] =
    &["business", "tests", "security", "style", "correctness"];

#[test]
fn speccy_review_fanout_partial_has_host_divergence_block() {
    // Source-shape guard: the shared review fan-out partial must
    // carry the canonical
    // `{% if host == "claude-code" %}` / `{% else %}` / `{% endif %}`
    // triple so the renderer (and any future contributor reading the
    // source) sees the same syntax. Both `speccy-review.md` and the
    // `speccy-orchestrate` review dispatch include this partial.
    let body = SPECCY_REVIEW_FANOUT_PARTIAL;
    assert!(
        body.contains("{% if host == \"claude-code\" %}"),
        "`partials/review-fanout.md` must contain a `{{% if host == \"claude-code\" %}}` block",
    );
    assert!(
        body.contains("{% else %}"),
        "`partials/review-fanout.md` must contain an `{{% else %}}` branch",
    );
    assert!(
        body.contains("{% endif %}"),
        "`partials/review-fanout.md` must close the divergence block with `{{% endif %}}`",
    );
}

#[test]
fn speccy_review_skill_prefers_native_subagents() {
    // Render once per host, then assert step 4 picks the host-native
    // subagent primitive and that both rendered outputs carry the
    // explicit `speccy review ... --persona X` fallback.

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
    // The `prose-spawn` wording must not leak into either render.
    // Pin it at the rendered-output layer so a future edit that
    // reintroduces the phrase fails this test first.
    assert!(
        !claude_body.to_lowercase().contains("prose-spawn"),
        "rendered Claude Code `speccy-review` SKILL.md must not contain `prose-spawn` wording; got:\n{claude_body}",
    );

    let codex =
        render_host_pack(HostChoice::Codex).expect("render_host_pack(codex) should succeed");
    let codex_body = find_rendered_skill(&codex, ".agents", "speccy-review");

    // Codex branch: step 4 must not mention `subagent_type:` (a
    // Claude-Code-specific key), must reference each default reviewer
    // subagent by name, and must invoke Codex's native sub-agent-spawn
    // primitive.
    assert!(
        !codex_body.contains("subagent_type:"),
        "rendered Codex `speccy-review` SKILL.md must not contain `subagent_type:` (Claude-Code-only key); got:\n{codex_body}",
    );
    assert!(
        !codex_body.to_lowercase().contains("prose-spawn"),
        "rendered Codex `speccy-review` SKILL.md must not contain `prose-spawn` wording; got:\n{codex_body}",
    );
    assert!(
        codex_body.contains("Codex's native sub-agent-spawn primitive"),
        "rendered Codex `speccy-review` SKILL.md must invoke `Codex's native sub-agent-spawn primitive` in step 4; got:\n{codex_body}",
    );
    for persona in DEFAULT_REVIEWER_PERSONAS {
        let needle = format!("`reviewer-{persona}`");
        assert!(
            codex_body.contains(&needle),
            "rendered Codex `speccy-review` SKILL.md must name persona `{persona}` as `{needle}`; got:\n{codex_body}",
        );
    }

    // Both rendered outputs must carry a spawn-prompt that references
    // the task selector (`SPEC-NNNN/T-NNN`) so the sub-agent knows
    // which task to review. The spawn prompt asks the sub-agent to
    // review the task directly without invoking a CLI command.
    for (label, body) in [
        (
            "claude-code .claude/skills/speccy-review/SKILL.md",
            claude_body,
        ),
        ("codex .agents/skills/speccy-review/SKILL.md", codex_body),
    ] {
        assert!(
            body.contains("SPEC-NNNN/T-NNN"),
            "rendered `{label}` must contain the `SPEC-NNNN/T-NNN` task selector \
             placeholder in the spawn prompt; got:\n{body}",
        );
        assert!(
            body.contains("<review persona="),
            "rendered `{label}` must reference the `<review persona=` element in \
             the spawn prompt so subagents know the expected output format; got:\n{body}",
        );
    }
}

// --------------------------------------------------------------------
// Claude Code SKILL.md wrappers under
// `resources/agents/.claude/skills/speccy-<verb>/SKILL.md.tmpl`.
//
// These wrappers are thin: a YAML frontmatter block (`name`,
// `description`) followed by exactly one
// `{% raw %}{% include "modules/skills/speccy-<verb>.md" %}{% endraw %}`
// directive.
// --------------------------------------------------------------------

/// Directory under the workspace root that holds the Claude Code
/// SKILL.md wrappers. Resolved via `CARGO_MANIFEST_DIR` so the test
/// is hermetic.
fn claude_skills_dir() -> std::path::PathBuf {
    workspace_root()
        .join("resources")
        .join("agents")
        .join(".claude")
        .join("skills")
}

#[test]
fn claude_code_skill_wrappers_match_skill_names() {
    let dir = claude_skills_dir();
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
        "the Claude Code SKILL.md.tmpl wrappers must match SKILL_NAMES exactly, one per shipped verb",
    );
}

/// Claude Code wrapper templates parse against `RecipeFrontmatter`
/// (the shared `name` / `description` serde-saphyr target) and embed
/// the matching module body via `{% include %}`.
#[test]
fn claude_code_wrapper_shape_and_body() {
    let dir = claude_skills_dir();
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

        // The three pinned phase-worker skills (`speccy-decompose`,
        // `speccy-work`, `speccy-ship`) have thin stub bodies instead
        // of a single `{% include %}` directive. `speccy-init` keeps
        // its full body but includes from `modules/phases/` rather
        // than `modules/skills/`.
        // All other skills follow the single-include shape.
        if PINNED_STUB_PHASES.contains(verb) {
            // Stub body: must reference `/agent speccy-<verb>` and the
            // matching agent file path. Must NOT be a single include
            // directive.
            assert!(
                !body.trim().starts_with("{%"),
                "stub wrapper `{}` body must not start with a `{{%` include directive (T-009 REQ-010); got: {:?}",
                path.display(),
                body.trim(),
            );
            assert!(
                body.contains(&format!("/agent {verb}")),
                "stub wrapper `{}` body must contain `/agent {verb}` (T-009 REQ-010)",
                path.display(),
            );
        } else if *verb == "speccy-init" {
            // init keeps full body but include path moved to modules/phases/.
            let expected_body = format!("{{% include \"modules/phases/{verb}.md\" %}}");
            assert_eq!(
                body.trim(),
                expected_body,
                "wrapper `{}` body (post-frontmatter, trimmed) must be the single `{{% include %}}` directive pointing at `modules/phases/` (T-009 path rename)",
                path.display(),
            );
        } else {
            let expected_body = format!("{{% include \"modules/skills/{verb}.md\" %}}");
            assert_eq!(
                body.trim(),
                expected_body,
                "wrapper `{}` body (post-frontmatter, trimmed) must be exactly the single `{{% include %}}` directive for the module body",
                path.display(),
            );
        }
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
fn t006_codex_skill_wrappers_match_skill_names() {
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
        "the Codex SKILL.md.tmpl wrappers must match SKILL_NAMES exactly, one per shipped verb",
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

        // The three pinned phase-worker skills (`speccy-decompose`,
        // `speccy-work`, `speccy-ship`) have thin stub bodies instead
        // of a single `{% include %}` directive. `speccy-init` keeps
        // its full body but includes from `modules/phases/` rather
        // than `modules/skills/`.
        // SPEC-0039 (REQ-003 / DEC-001 mechanism B): the Codex
        // `speccy-orchestrate` wrapper carries the host-neutral body
        // include plus a Codex-only permission-grant module include.
        // All other skills follow the single-include shape.
        if PINNED_STUB_PHASES.contains(verb) {
            // Stub body: must reference `/agent speccy-<verb>` and the
            // matching agent file path. Must NOT be a single include
            // directive.
            assert!(
                !body.trim().starts_with("{%"),
                "stub wrapper `{}` body must not start with a `{{%` include directive (T-009 REQ-010); got: {:?}",
                path.display(),
                body.trim(),
            );
            assert!(
                body.contains(&format!("/agent {verb}")),
                "stub wrapper `{}` body must contain `/agent {verb}` (T-009 REQ-010)",
                path.display(),
            );
        } else if *verb == "speccy-init" {
            // init keeps full body but include path moved to modules/phases/.
            let expected_body = format!("{{% include \"modules/phases/{verb}.md\" %}}");
            assert_eq!(
                body.trim(),
                expected_body,
                "wrapper `{}` body (post-frontmatter, trimmed) must be the single `{{% include %}}` directive pointing at `modules/phases/` (T-009 path rename)",
                path.display(),
            );
        } else if *verb == "speccy-orchestrate" {
            // SPEC-0039 REQ-003: the Codex orchestrate wrapper includes
            // the host-neutral body AND the Codex-only permission-grant
            // module via DEC-001 mechanism B (additive selective-include).
            assert!(
                body.contains("{% include \"modules/skills/speccy-orchestrate.md\" %}"),
                "wrapper `{}` body must include the host-neutral orchestrate body",
                path.display(),
            );
            assert!(
                body.contains("{% include \"modules/skills/speccy-orchestrate-codex-grant.md\" %}"),
                "wrapper `{}` body must include the Codex permission-grant module (SPEC-0039 REQ-003)",
                path.display(),
            );
        } else {
            let expected_body = format!("{{% include \"modules/skills/{verb}.md\" %}}");
            assert_eq!(
                body.trim(),
                expected_body,
                "wrapper `{}` body (post-frontmatter, trimmed) must be exactly the single `{{% include %}}` directive for the module body",
                path.display(),
            );
        }
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

/// Seven reviewer-persona names shipped by `speccy-core::personas::ALL`.
/// Duplicated locally as a `const &[&str]` so the T-009 tests stay
/// hermetic w.r.t. `personas::ALL`'s declared order.
const REVIEWER_PERSONAS: &[&str] = &[
    "business",
    "tests",
    "security",
    "style",
    "correctness",
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
fn t009_claude_code_reviewer_wrappers_exactly_seven() {
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
        "exactly seven Claude Code reviewer wrappers must exist, one per shipped reviewer persona",
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
    // SPEC-0032 T-001 added phase-worker subagent files at
    // `.claude/agents/speccy-<phase>.md` alongside the existing
    // reviewer subagent files. Filter on the `reviewer-` prefix so
    // the reviewer-shape assertions below stay scoped to reviewers.
    let agent_files: Vec<&speccy_cli::render::RenderedFile> = rendered
        .iter()
        .filter(|f| f.rel_path.as_str().starts_with(".claude/agents/reviewer-"))
        .collect();
    assert_eq!(
        agent_files.len(),
        7,
        "claude-code host pack should render seven reviewer subagent files under .claude/agents/reviewer-*.md; got {}",
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
// SPEC-0053 T-001 (CHK-001, CHK-002): the `reviewer-correctness`
// persona renders for both hosts with all `{% include %}` directives
// expanded, and the rendered body names the four deferral targets as
// out-of-scope and carries the literal confidence threshold `80`.
// --------------------------------------------------------------------

/// Returns the rendered `reviewer-correctness` body for the given host,
/// asserting the file exists.
fn rendered_correctness_body(host: HostChoice, dir: &str, suffix: &str) -> String {
    let rendered = render_host_pack(host).expect("render_host_pack should succeed");
    let rel = format!("{dir}/agents/reviewer-correctness.{suffix}");
    rendered
        .iter()
        .find(|f| f.rel_path.as_str() == rel)
        .unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "rendered host pack must include `{rel}` after T-001"
            ))
        })
        .contents
        .clone()
}

#[test]
fn reviewer_correctness_renders_with_includes_expanded_both_hosts() {
    // CHK-001: both hosts render the persona with every `{% ... %}`
    // include directive expanded and no `<...>` placeholder left.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_correctness_body(host, dir, suffix);
        assert!(
            !body.contains("{%"),
            "rendered `{dir}/agents/reviewer-correctness.{suffix}` must have all `{{% ... %}}` includes expanded; got:\n{body}",
        );
        // The persona body pulls in the shared review-contract snippets
        // via `{% include %}`; their expanded text must be present.
        assert!(
            body.contains("Your final message to the orchestrator"),
            "rendered reviewer-correctness ({dir}) must contain the expanded verdict-return contract; got:\n{body}",
        );
    }
}

#[test]
fn reviewer_correctness_body_names_deferrals_and_threshold() {
    // CHK-002: the rendered body names all four deferral targets as
    // out-of-scope and carries the literal confidence threshold `80`,
    // gating a silent drop of the scope/filter on a future edit.
    let body = rendered_correctness_body(HostChoice::ClaudeCode, ".claude", "md");
    // Scope the deferral-target assertion to the "Out of scope — defer"
    // section. `security`/`style`/`business` also appear in the Focus
    // section, so a body-wide `contains` would let three of the four
    // targets pass even if the deferral section were deleted; only the
    // section slice makes all four load-bearing.
    let mut in_defer = false;
    let mut defer_section = String::new();
    for line in body.lines() {
        if line.starts_with("## Out of scope") {
            in_defer = true;
            continue;
        }
        if in_defer && line.starts_with("## ") {
            break;
        }
        if in_defer {
            defer_section.push_str(line);
            defer_section.push('\n');
        }
    }
    assert!(
        !defer_section.is_empty(),
        "reviewer-correctness must have a non-empty out-of-scope deferral section; got:\n{body}",
    );
    for target in ["security", "style", "business", "tests"] {
        assert!(
            defer_section.contains(target),
            "reviewer-correctness out-of-scope section must name deferral target `{target}`; got:\n{defer_section}",
        );
    }
    assert!(
        body.contains("80"),
        "rendered reviewer-correctness must state the confidence->=80 reporting threshold; got:\n{body}",
    );
}

// --------------------------------------------------------------------
// SPEC-0053 T-002 (CHK-003): the `plan-explorer` persona renders for
// both hosts with all `{% include %}` directives expanded, and the
// rendered body carries the advisory, non-verdict contract — it must
// not contain the `<review` verdict-contract marker.
// --------------------------------------------------------------------

/// Returns the rendered `plan-explorer` body for the given host,
/// asserting the file exists.
fn rendered_plan_explorer_body(host: HostChoice, dir: &str, suffix: &str) -> String {
    let rendered = render_host_pack(host).expect("render_host_pack should succeed");
    let rel = format!("{dir}/agents/plan-explorer.{suffix}");
    rendered
        .iter()
        .find(|f| f.rel_path.as_str() == rel)
        .unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "rendered host pack must include `{rel}` after T-002"
            ))
        })
        .contents
        .clone()
}

#[test]
fn plan_explorer_renders_with_includes_expanded_both_hosts() {
    // CHK-003: both hosts render the persona with every `{% ... %}`
    // include directive expanded.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_plan_explorer_body(host, dir, suffix);
        assert!(
            !body.contains("{%"),
            "rendered `{dir}/agents/plan-explorer.{suffix}` must have all `{{% ... %}}` includes expanded; got:\n{body}",
        );
    }
}

#[test]
fn plan_explorer_body_has_no_review_verdict_marker_both_hosts() {
    // CHK-003: plan-explorer is advisory, not a reviewer. Its rendered
    // body must not carry the `<review` verdict-contract marker — that
    // would mean a verdict-contract snippet leaked in, contradicting
    // the report-only contract. The check is host-independent: the
    // body is identical across wrappers, but assert on both so a
    // wrapper that accidentally inlines a verdict snippet is caught.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_plan_explorer_body(host, dir, suffix);
        assert!(
            !body.contains("<review"),
            "rendered `{dir}/agents/plan-explorer.{suffix}` must not contain the `<review` verdict-contract marker (advisory, non-verdict contract); got:\n{body}",
        );
    }
}

// --------------------------------------------------------------------
// SPEC-0053 T-003 (CHK-004): the `plan-architect` persona renders for
// both hosts with all `{% include %}` directives expanded, carries the
// advisory non-verdict contract (no `<review` marker), and specifies
// that build-sequence items are agent-sized.
// --------------------------------------------------------------------

/// Returns the rendered `plan-architect` body for the given host,
/// asserting the file exists.
fn rendered_plan_architect_body(host: HostChoice, dir: &str, suffix: &str) -> String {
    let rendered = render_host_pack(host).expect("render_host_pack should succeed");
    let rel = format!("{dir}/agents/plan-architect.{suffix}");
    rendered
        .iter()
        .find(|f| f.rel_path.as_str() == rel)
        .unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "rendered host pack must include `{rel}` after T-003"
            ))
        })
        .contents
        .clone()
}

#[test]
fn plan_architect_renders_with_includes_expanded_both_hosts() {
    // CHK-004: both hosts render the persona with every `{% ... %}`
    // include directive expanded.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_plan_architect_body(host, dir, suffix);
        assert!(
            !body.contains("{%"),
            "rendered `{dir}/agents/plan-architect.{suffix}` must have all `{{% ... %}}` includes expanded; got:\n{body}",
        );
    }
}

#[test]
fn plan_architect_body_has_no_review_verdict_marker_both_hosts() {
    // CHK-004: plan-architect is advisory, not a reviewer. Its rendered
    // body must not carry the `<review` verdict-contract marker — that
    // would mean a verdict-contract snippet leaked in, contradicting the
    // blueprint-only contract.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_plan_architect_body(host, dir, suffix);
        assert!(
            !body.contains("<review"),
            "rendered `{dir}/agents/plan-architect.{suffix}` must not contain the `<review` verdict-contract marker (advisory, non-verdict contract); got:\n{body}",
        );
    }
}

#[test]
fn plan_architect_body_specifies_agent_sized_build_sequence_both_hosts() {
    // CHK-004 / REQ-003 <done-when>: the body must specify that the
    // build-sequence checklist items are agent-sized (one item ≈ one
    // task), which is what makes the checklist directly consumable as
    // candidate tasks.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_plan_architect_body(host, dir, suffix);
        // Assert on language that lives ONLY in the included persona
        // body's build-sequence section, never in the wrapper
        // frontmatter `description`. The description paraphrases the
        // contract ("a build-sequence checklist whose items are
        // agent-sized (one item ≈ one Speccy task)"), so a loose
        // substring like "agent-sized" / "build-sequence" against the
        // full rendered file would pass even if the body said nothing.
        // The dedicated section heading and the explicit "one item is a
        // plausible single Speccy task" sizing sentence appear only in
        // the body, so this fails RED when the body language is removed.
        assert!(
            body.contains("## Build sequence — an agent-sized ordered checklist"),
            "rendered `{dir}/agents/plan-architect.{suffix}` must carry the dedicated agent-sized build-sequence section heading from the persona body; got:\n{body}",
        );
        assert!(
            body.contains("one item is a plausible single Speccy task"),
            "rendered `{dir}/agents/plan-architect.{suffix}` body must specify that each build-sequence item is agent-sized (one plausible single Speccy task); got:\n{body}",
        );
    }
}

// --------------------------------------------------------------------
// SPEC-0053 T-005 (CHK-008 / CHK-009): the plan-time subagents are
// wired into their host skills. `speccy-brainstorm` and `speccy-plan`
// invoke `plan-explorer` and route its report into SPEC.md sections
// (never a new artifact file); `speccy-decompose` invokes
// `plan-architect`, names the build-sequence checklist as candidate
// tasks, and promotes decisions into `### Decisions`.
// --------------------------------------------------------------------

/// Returns the rendered `speccy-decompose` body. The decompose recipe
/// is a pinned phase-worker stub, so its full body renders into the
/// agent wrapper at `<install_root>/agents/speccy-decompose.md`, not
/// the SKILL.md stub.
fn rendered_decompose_body(host: HostChoice, install_root: &str, suffix: &str) -> String {
    let rendered = render_host_pack(host).expect("render_host_pack should succeed");
    let rel = format!("{install_root}/agents/speccy-decompose.{suffix}");
    rendered
        .iter()
        .find(|f| f.rel_path.as_str() == rel)
        .unwrap_or_else(|| {
            panic_with_test_message(&format!("rendered host pack must include `{rel}`"))
        })
        .contents
        .clone()
}

#[test]
fn brainstorm_and_plan_skills_invoke_plan_explorer_without_new_artifact() {
    // CHK-008: both `speccy-brainstorm` and `speccy-plan` reference
    // invoking the `plan-explorer` subagent, and neither directs the
    // explorer's report into a new `*.md` artifact file — its only
    // durable home is the existing SPEC.md routing targets.
    //
    // Render both hosts: the skill bodies are host-neutral but the
    // wrappers differ, so a per-host render guards against a wiring
    // edit that lands in only one pack.
    for host in [HostChoice::ClaudeCode, HostChoice::Codex] {
        let rendered = render_host_pack(host).expect("render_host_pack should succeed");
        let install_root = match host {
            HostChoice::ClaudeCode => ".claude",
            HostChoice::Codex => ".agents",
        };
        for verb in ["speccy-brainstorm", "speccy-plan"] {
            let body = find_rendered_skill(&rendered, install_root, verb);
            assert!(
                body.contains("plan-explorer"),
                "rendered `{install_root}/skills/{verb}/SKILL.md` must reference invoking the `plan-explorer` subagent; got:\n{body}",
            );
            // The routing prose must state the explorer report is
            // ephemeral and not persisted to a new artifact file. Assert
            // on the distinctive contiguous phrase that occurs ONLY in
            // the no-artifact routing clause of each wiring block — the
            // `new `*.md` artifact file` qualifier. A broad
            // `contains("artifact")` check passes on unrelated
            // pre-existing prose (brainstorm's `four artifacts`, plan's
            // `## Open Questions` line), so it would stay GREEN even if
            // the no-artifact clause were deleted while the
            // `plan-explorer` invocation stayed. Scoping to this phrase
            // ties the assertion to the routing sentence: deleting the
            // clause flips it RED.
            assert!(
                body.contains("new `*.md` artifact file"),
                "rendered `{install_root}/skills/{verb}/SKILL.md` must state the explorer report is not persisted to a new `*.md` artifact file (the no-artifact routing clause); got:\n{body}",
            );
        }
    }
}

#[test]
fn decompose_skill_invokes_plan_architect_with_candidate_tasks_and_decisions() {
    // CHK-009: the `speccy-decompose` body references invoking
    // `plan-architect`, names the build-sequence checklist as the
    // CANDIDATE task list, and references promoting decisions into
    // `### Decisions`.
    for (host, install_root, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_decompose_body(host, install_root, suffix);
        assert!(
            body.contains("plan-architect"),
            "rendered `{install_root}/agents/speccy-decompose.{suffix}` must reference invoking the `plan-architect` subagent; got:\n{body}",
        );
        assert!(
            body.contains("candidate"),
            "rendered `{install_root}/agents/speccy-decompose.{suffix}` must name the build-sequence checklist as the candidate task list; got:\n{body}",
        );
        assert!(
            body.contains("build-sequence"),
            "rendered `{install_root}/agents/speccy-decompose.{suffix}` must reference the build-sequence checklist from plan-architect; got:\n{body}",
        );
        assert!(
            body.contains("### Decisions"),
            "rendered `{install_root}/agents/speccy-decompose.{suffix}` must direct promoting decisions into `### Decisions`; got:\n{body}",
        );
    }
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
fn t010_codex_reviewer_wrappers_exactly_seven() {
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
        "exactly seven Codex reviewer wrappers must exist, one per shipped reviewer persona",
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
    //
    // SPEC-0032 T-004 added phase-worker subagent files at
    // `.codex/agents/speccy-<phase>.toml` alongside the existing
    // reviewer subagent files. Filter on the `reviewer-` prefix so
    // the reviewer-shape assertions below stay scoped to reviewers.
    let rendered =
        render_host_pack(HostChoice::Codex).expect("render_host_pack(codex) should succeed");
    let agent_files: Vec<&speccy_cli::render::RenderedFile> = rendered
        .iter()
        .filter(|f| f.rel_path.as_str().starts_with(".codex/agents/reviewer-"))
        .collect();
    assert_eq!(
        agent_files.len(),
        7,
        "codex host pack should render seven reviewer subagent files under .codex/agents/reviewer-*.toml; got {}",
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
    let entries = fs_err::read_dir(&dir).expect("resources/modules/personas/ must exist");
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

// --------------------------------------------------------------------
// SPEC-0025 REQ-002 content-shape: the speccy-brainstorm skill body
// must teach the Socratic flow with a hard gate, naming the four
// artifacts, the "2-3" soft-guidance count, the four destination
// sections, "one question at a time" discipline, and both terminal
// actions (`speccy-plan` for new specs, `speccy-amend` for
// amendments). Without these grep-style assertions, a mutation that
// strips the hard gate, drops a destination, or collapses the
// amendment branch leaves every other test green — the canonical
// "test passes even when the behaviour is gone" failure mode that
// SPEC-0025 T-001's tests reviewer flagged.
// --------------------------------------------------------------------

fn read_brainstorm_module_body() -> String {
    let path = workspace_root()
        .join("resources")
        .join("modules")
        .join("skills")
        .join("speccy-brainstorm.md");
    fs_err::read_to_string(&path).unwrap_or_else(|err| {
        panic_with_test_message(&format!(
            "`resources/modules/skills/speccy-brainstorm.md` must be readable: {err}"
        ))
    })
}

#[test]
fn brainstorm_module_body_names_four_artifacts() {
    let body = read_brainstorm_module_body();
    for label in [
        "Restated ask",
        "alternative framings",
        "Silent assumptions",
        "Open questions",
    ] {
        assert!(
            body.contains(label),
            "speccy-brainstorm.md must name the artifact label `{label}` (REQ-002 done-when item 1)",
        );
    }
    assert!(
        body.contains("- [ ]"),
        "speccy-brainstorm.md must instruct the agent to use the `- [ ]` checkbox format \
         for open questions so the output is copy-pasteable into SPEC.md (REQ-002 done-when item 1d)",
    );
}

#[test]
fn brainstorm_module_body_names_two_to_three_soft_guidance() {
    let body = read_brainstorm_module_body();
    assert!(
        body.contains("2-3"),
        "speccy-brainstorm.md must name `2-3` as the suggested alternative-framings count (REQ-002 done-when item 2)",
    );
    let lower = body.to_lowercase();
    assert!(
        lower.contains("soft guidance"),
        "speccy-brainstorm.md must explicitly mark `2-3` as soft guidance scaled to complexity (REQ-002 done-when item 2)",
    );
}

#[test]
fn brainstorm_module_body_teaches_one_question_at_a_time() {
    let body = read_brainstorm_module_body();
    let lower = body.to_lowercase();
    assert!(
        lower.contains("one question at a time"),
        "speccy-brainstorm.md must teach `one question at a time` as an explicit interaction discipline (REQ-002 done-when item 5; CHK-002)",
    );
}

#[test]
fn brainstorm_module_body_carries_prose_hard_gate() {
    let body = read_brainstorm_module_body();
    let lower = body.to_lowercase();
    assert!(
        lower.contains("hard gate"),
        "speccy-brainstorm.md must carry an explicit prose `hard gate` instruction (REQ-002 done-when item 3)",
    );
    let strong_marker = body.contains("Do NOT") || body.contains("do NOT") || body.contains("STOP");
    assert!(
        strong_marker,
        "speccy-brainstorm.md must use strong gate language (`Do NOT` / `STOP`) so an attentive agent honors it (REQ-002 done-when item 3)",
    );
    assert!(
        body.contains("{{ cmd_prefix }}speccy-plan"),
        "speccy-brainstorm.md hard-gate prose must name `{{{{ cmd_prefix }}}}speccy-plan` as the gated action (REQ-002 behavior; CHK-002)",
    );
    let machine_marker = body.contains("<HARD-GATE>") || body.contains("<hard-gate>");
    assert!(
        !machine_marker,
        "speccy-brainstorm.md must NOT introduce a machine sentinel for the gate (DEC-003 / REQ-002 done-when item 3)",
    );
}

#[test]
fn brainstorm_module_body_names_four_routing_destinations() {
    let body = read_brainstorm_module_body();
    for destination in [
        "## Summary",
        "## Assumptions",
        "## Open Questions",
        "## Notes",
    ] {
        assert!(
            body.contains(destination),
            "speccy-brainstorm.md must name `{destination}` as a routing destination (REQ-002 done-when item 4; CHK-002)",
        );
    }
    assert!(
        body.contains("### Decisions") && body.contains("<decision>"),
        "speccy-brainstorm.md must reference `### Decisions` / `<decision>` for load-bearing trade-offs (REQ-002 done-when item 4)",
    );
}

#[test]
fn brainstorm_module_body_names_both_terminal_actions() {
    let body = read_brainstorm_module_body();
    assert!(
        body.contains("{{ cmd_prefix }}speccy-plan"),
        "speccy-brainstorm.md must point at `{{{{ cmd_prefix }}}}speccy-plan` as the terminal action for the new-spec path (REQ-002 done-when item 6)",
    );
    assert!(
        body.contains("{{ cmd_prefix }}speccy-amend"),
        "speccy-brainstorm.md must point at `{{{{ cmd_prefix }}}}speccy-amend` as the terminal action for the amendment path (REQ-002 done-when item 6)",
    );
}

#[test]
fn brainstorm_module_body_uses_cmd_prefix_consistently() {
    // SPEC-0025 T-001 retry: the source module body must use
    // `{{ cmd_prefix }}speccy-plan` everywhere — bare `/speccy-plan`
    // bleeds through to the Codex mirror as a literal slash under a
    // no-prefix host. Allow `/speccy-plan` only inside the example
    // quoting the user's verbatim instruction ("skip the brainstorm,
    // just write the SPEC"), which has no slash, so the assertion is
    // unconditional: there must be zero literal `/speccy-plan`
    // occurrences in the resource module body.
    let body = read_brainstorm_module_body();
    assert!(
        !body.contains("/speccy-plan"),
        "speccy-brainstorm.md must use `{{{{ cmd_prefix }}}}speccy-plan` rather than a hard-coded `/speccy-plan`; \
         the literal slash bleeds into the Codex mirror (a no-prefix host)",
    );
    assert!(
        !body.contains("/speccy-amend"),
        "speccy-brainstorm.md must use `{{{{ cmd_prefix }}}}speccy-amend` rather than a hard-coded `/speccy-amend`; \
         same prefix-leak risk as the `speccy-plan` reference",
    );
}

#[test]
fn brainstorm_rendered_outputs_use_host_specific_prefix() {
    // The renderer resolves `{{ cmd_prefix }}` to `/` on Claude Code
    // (skill invocations are slash-commands) and to empty string on
    // Codex (skill invocations are bare). REQ-002 CHK-002 names both
    // host-specific forms; this test exercises the rendering rather
    // than just the module body.
    for (host, install_root, expected_plan, expected_amend, unexpected) in [
        (
            HostChoice::ClaudeCode,
            ".claude",
            "/speccy-plan",
            "/speccy-amend",
            // Bare forms must not appear on Claude Code where the
            // slash prefix is mandatory.
            None,
        ),
        (
            HostChoice::Codex,
            ".agents",
            "speccy-plan",
            "speccy-amend",
            // Slashed forms must not appear on Codex where the prefix
            // is empty.
            Some(("/speccy-plan", "/speccy-amend")),
        ),
    ] {
        let rendered = render_host_pack(host).unwrap_or_else(|err| {
            panic_with_test_message(&format!("render_host_pack({host:?}) should succeed: {err}"))
        });
        let body = find_rendered_skill(&rendered, install_root, "speccy-brainstorm");
        assert!(
            body.contains(expected_plan),
            "rendered `{install_root}/skills/speccy-brainstorm/SKILL.md` must contain `{expected_plan}` as the terminal new-spec action (REQ-002 CHK-002)",
        );
        assert!(
            body.contains(expected_amend),
            "rendered `{install_root}/skills/speccy-brainstorm/SKILL.md` must contain `{expected_amend}` as the terminal amendment action (REQ-002 done-when item 6)",
        );
        if let Some((slashed_plan, slashed_amend)) = unexpected {
            assert!(
                !body.contains(slashed_plan),
                "rendered Codex `{install_root}/skills/speccy-brainstorm/SKILL.md` must not contain `{slashed_plan}` — Codex skill invocations are bare (no leading slash)",
            );
            assert!(
                !body.contains(slashed_amend),
                "rendered Codex `{install_root}/skills/speccy-brainstorm/SKILL.md` must not contain `{slashed_amend}` — Codex skill invocations are bare (no leading slash)",
            );
        }
    }
}

// --------------------------------------------------------------------
// SPEC-0031 REQ-005 / CHK-005: reviewer-tests persona and prompt load
// the evidence file and stay framework-agnostic; the other six
// built-in reviewer personas carry no evidence-related instruction.
// --------------------------------------------------------------------

/// Reviewer personas other than `tests`. The SPEC-0031 REQ-005
/// asymmetry: only the `tests` persona / prompt names evidence
/// loading; the other six anchor on diff + SPEC + `<task-scenarios>`
/// alone.
const NON_TESTS_REVIEWER_PERSONAS: [&str; 6] = [
    "business",
    "security",
    "style",
    "correctness",
    "architecture",
    "docs",
];

/// Framework-specific anchor strings the reviewer-tests persona must
/// not name inside normative guidance. SPEC-0031 REQ-005's
/// framework-agnostic clause and CHK-005's literal grep.
const FRAMEWORK_ANCHORS: [&str; 9] = [
    "test result: FAILED",
    " \u{2717} ",
    "FAILED:",
    "error[E",
    "cargo test",
    "pnpm test",
    "pytest",
    "jest",
    "vitest",
];

/// SPEC-named fabrication patterns the reviewer-tests persona must
/// enumerate. Each marker is a distinctive substring that uniquely
/// identifies the corresponding bullet in the persona body. SPEC-0031
/// REQ-005 done-when item 2 names the five patterns.
const FABRICATION_PATTERN_MARKERS: [&str; 5] = [
    "structural artifacts",
    "test names",
    "identical",
    "suspiciously clean",
    "hygiene",
];

/// Slice the persona body into its normative portion: everything
/// before the `## Example` worked-example section, since the SPEC
/// carves out worked-example asides and `<!-- ... -->` annotations
/// from the framework-anchor check.
fn normative_persona_body(body: &str) -> &str {
    body.split("\n## Example").next().unwrap_or(body)
}

#[test]
fn reviewer_tests_persona_loads_evidence() {
    let body = read_persona("reviewer-tests.md");

    // Four-step evidence-loading sequence: the field is named and the
    // host Read primitive is invoked to load the referenced file.
    assert!(
        body.contains("Evidence:"),
        "`reviewer-tests.md` must name the `Evidence:` field so the reviewer knows what to extract from `<implementer-note>` bodies (SPEC-0031 REQ-005 done-when item 1)",
    );
    assert!(
        body.contains("Read primitive"),
        "`reviewer-tests.md` must instruct the reviewer to read the evidence file via the host Read primitive (SPEC-0031 REQ-005 done-when item 1)",
    );

    // Blocking-verdict guidance: evidence absence and fabrication both
    // map to `verdict=\"blocking\"`. SPEC-0031 REQ-005 done-when item 1.
    let normative = normative_persona_body(body);
    let lower = normative.to_lowercase();
    assert!(
        lower.contains("blocking"),
        "`reviewer-tests.md` normative guidance must name `blocking` as the verdict for evidence absence or fabrication (SPEC-0031 REQ-005 done-when item 1)",
    );

    // Fabrication-pattern enumeration: at least the five SPEC-named
    // patterns must appear in the persona body.
    for marker in FABRICATION_PATTERN_MARKERS {
        assert!(
            lower.contains(&marker.to_lowercase()),
            "`reviewer-tests.md` must enumerate the fabrication pattern marked by `{marker}` (SPEC-0031 REQ-005 done-when item 2)",
        );
    }

    // Framework-agnostic clause: no per-framework anchor strings
    // inside normative guidance. Worked-example asides under
    // `## Example` are out of scope.
    for anchor in FRAMEWORK_ANCHORS {
        assert!(
            !normative.contains(anchor),
            "`reviewer-tests.md` normative guidance must not name the framework-specific anchor `{anchor}` (SPEC-0031 REQ-005 done-when item 3); move it under `## Example` or rephrase framework-agnostically",
        );
    }
}

#[test]
fn non_tests_reviewer_files_carry_no_evidence_instruction() {
    // The asymmetry is the design: only the `tests` persona names
    // evidence loading. The other six anchor on diff + SPEC +
    // `<task-scenarios>` alone.
    for persona in NON_TESTS_REVIEWER_PERSONAS {
        let file = format!("reviewer-{persona}.md");
        let body = read_persona(&file);
        assert!(
            !body.contains("Evidence:"),
            "`personas/{file}` must not mention `Evidence:` — the SPEC-0031 REQ-005 asymmetry reserves evidence-loading instruction for the `tests` persona",
        );
        assert!(
            !body.contains("evidence file"),
            "`personas/{file}` must not mention `evidence file` — the SPEC-0031 REQ-005 asymmetry reserves evidence-loading instruction for the `tests` persona",
        );
    }
}

// --------------------------------------------------------------------
// SPEC-0053 CHK-006: packaging conventions for the three feature-dev
// ports (reviewer-correctness, plan-explorer, plan-architect). Each
// Claude wrapper declares `model: opus[1m]`, each Codex wrapper
// declares `model = "gpt-5.5"`, none declares `sonnet`, and each
// persona body carries a `feature-dev` attribution line.
// --------------------------------------------------------------------

/// The three personas ported from `feature-dev` in SPEC-0053
/// T-001 / T-002 / T-003. Their packaging invariants are asserted as
/// an aggregate here rather than per-authoring-task.
const FEATURE_DEV_PERSONAS: &[&str] = &["reviewer-correctness", "plan-explorer", "plan-architect"];

/// Look up a rendered agent wrapper body by its `rel_path` in a
/// `render_host_pack` output vector. `rel_path` already has the
/// `agents/` prefix and `.tmpl` suffix stripped, so the needle is the
/// install-root-relative destination (e.g.
/// `.claude/agents/plan-explorer.md`).
fn find_rendered_agent<'a>(
    rendered: &'a [speccy_cli::render::RenderedFile],
    rel_path: &str,
) -> &'a str {
    let file = rendered
        .iter()
        .find(|f| f.rel_path.as_str() == rel_path)
        .unwrap_or_else(|| {
            panic_with_test_message(&format!("rendered host pack must contain `{rel_path}`"))
        });
    file.contents.as_str()
}

#[derive(Debug, Deserialize)]
struct AgentModelFrontmatter {
    model: String,
}

#[test]
fn feature_dev_personas_declare_speccy_model_conventions_and_attribution() {
    let claude = render_host_pack(HostChoice::ClaudeCode)
        .unwrap_or_else(|err| panic_with_test_message(&format!("render claude pack: {err}")));
    let codex = render_host_pack(HostChoice::Codex)
        .unwrap_or_else(|err| panic_with_test_message(&format!("render codex pack: {err}")));

    for persona in FEATURE_DEV_PERSONAS {
        // Claude wrapper: YAML frontmatter, `model: opus[1m]`.
        let claude_path = format!(".claude/agents/{persona}.md");
        let claude_body = find_rendered_agent(&claude, &claude_path);
        let (claude_yaml, _rest) = split_frontmatter(claude_body).unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "Claude wrapper `{claude_path}` must have a `---` frontmatter fence"
            ))
        });
        let claude_fm: AgentModelFrontmatter =
            serde_saphyr::from_str(claude_yaml).unwrap_or_else(|err| {
                panic_with_test_message(&format!(
                    "Claude wrapper `{claude_path}` frontmatter must be valid YAML: {err}"
                ))
            });
        assert_eq!(
            claude_fm.model, "opus[1m]",
            "Claude wrapper `{claude_path}` must declare `model: opus[1m]`, got `{}`",
            claude_fm.model,
        );

        // Codex wrapper: the whole `.toml` file is TOML; `model = "gpt-5.5"`.
        let codex_path = format!(".codex/agents/{persona}.toml");
        let codex_body = find_rendered_agent(&codex, &codex_path);
        let codex_fm: AgentModelFrontmatter = toml::from_str(codex_body).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "Codex wrapper `{codex_path}` must be valid TOML: {err}"
            ))
        });
        assert_eq!(
            codex_fm.model, "gpt-5.5",
            "Codex wrapper `{codex_path}` must declare `model = \"gpt-5.5\"`, got `{}`",
            codex_fm.model,
        );

        // Neither wrapper may declare `sonnet` anywhere.
        assert!(
            !claude_body.contains("sonnet"),
            "Claude wrapper `{claude_path}` must not declare `sonnet`",
        );
        assert!(
            !codex_body.contains("sonnet"),
            "Codex wrapper `{codex_path}` must not declare `sonnet`",
        );

        // The persona body carries a `feature-dev` attribution line.
        let persona_body = read_persona(&format!("{persona}.md"));
        assert!(
            persona_body.contains("feature-dev"),
            "persona body `personas/{persona}.md` must carry a `feature-dev` attribution line",
        );
    }
}
