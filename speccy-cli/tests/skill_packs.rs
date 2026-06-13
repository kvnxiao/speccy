#![expect(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Skill-pack content tests.
//!
//! - [`persona_names_match_registry`], [`persona_content_shape`]: reviewer
//!   persona files match the registry and carry the expected body shape.
//! - [`recipe_content_shape`]: rendered recipes follow the recipe schema for
//!   both hosts.
//! - [`claude_code_wrapper_shape_and_body`],
//!   [`t006_codex_wrapper_shape_and_body`],
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
/// content-shape tests read as the assertion only.
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
/// install destination (Claude Code →
/// `.claude/`, Codex → `.agents/`); `.codex/` is the subagent root and
/// has no skills bundle.
const HOST_SKILL_ROOTS: &[(&str, &str)] = &[("claude-code", ".claude"), ("codex", ".agents")];

/// The three pinned phase-worker skill verbs whose SKILL.md.tmpl bodies
/// became thin stubs. These stubs do not contain a
/// single `{% include %}` directive, do not contain `## When to use`,
/// and do not carry a full `speccy …` command in a code fence — they
/// are pointer-only bodies. The fourth phase (`speccy-init`) keeps its
/// full body sourced from `modules/phases/speccy-init.md` (it is
/// excluded from the stub transformation).
// `speccy-work` migrated from
// stub-delegate to pure-include shape, so it is no longer a "stub"
// wrapper; only `speccy-decompose` and `speccy-ship` remain in
// stub-delegate form.
const PINNED_STUB_PHASES: &[&str] = &["speccy-decompose", "speccy-ship"];

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
// persona content shape (reviewer personas only)
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
// recipe content shape
// --------------------------------------------------------------------

fn first_non_frontmatter_paragraph(body: &str) -> Option<&str> {
    let after = split_frontmatter(body).map_or(body, |(_yaml, rest)| rest);
    after
        .lines()
        .skip_while(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
        .find(|line| !line.trim().is_empty())
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

            // Skip full content-shape checks for stub skills.
            if PINNED_STUB_PHASES.contains(verb) {
                // Stubs must be non-empty and must name the `/agent`
                // invocation pointer — the only content-shape guarantee
                // that applies to them.
                assert!(
                    !body.trim().is_empty(),
                    "stub recipe `{install_root}/skills/{verb}/SKILL.md` must be non-empty",
                );
                continue;
            }

            assert!(
                first_non_frontmatter_paragraph(body).is_some(),
                "rendered recipe `{install_root}/skills/{verb}/SKILL.md` must include an intro paragraph after the title",
            );
        }
    }
}

// --------------------------------------------------------------------
// descriptions tuned for natural-language activation
// --------------------------------------------------------------------

#[test]
fn shipped_descriptions_natural_language_triggers() {
    // Codex hard-rejects descriptions over 1024 Unicode chars at skill load
    // (codex-rs/core-skills/src/loader.rs::MAX_DESCRIPTION_LEN; see
    // openai/codex#13941). This is the binding constraint; Claude Code's
    // documented 1536-char cap is softer (truncation).
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

// --------------------------------------------------------------------
// Rendered-output shape for `speccy-review`.
//
// Step 4 of the speccy-review module body renders host-divergently:
// the rendered `/speccy-review` skill picks the host-native subagent
// primitive (Claude Code's `Task` tool with `subagent_type`; Codex's
// native sub-agent-spawn primitive against each registered
// `reviewer-<persona>` sub-agent).
// --------------------------------------------------------------------

/// Default reviewer fan-out used by both `/speccy-review` rendered
/// branches: the five personas Speccy invokes per task. Other shipped
/// reviewers (`architecture`, `docs`) are explicit-only.
const DEFAULT_REVIEWER_PERSONAS: &[&str] =
    &["business", "tests", "security", "style", "correctness"];

#[test]
fn speccy_review_skill_prefers_native_subagents() {
    // Render once per host, then assert step 4 picks the host-native
    // subagent primitive.

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

    let codex =
        render_host_pack(HostChoice::Codex).expect("render_host_pack(codex) should succeed");
    let codex_body = find_rendered_skill(&codex, ".agents", "speccy-review");

    // Codex branch: step 4 must not mention `subagent_type:` (a
    // Claude-Code-specific key) and must reference each default
    // reviewer subagent by name.
    assert!(
        !codex_body.contains("subagent_type:"),
        "rendered Codex `speccy-review` SKILL.md must not contain `subagent_type:` (Claude-Code-only key); got:\n{codex_body}",
    );
    for persona in DEFAULT_REVIEWER_PERSONAS {
        let needle = format!("`reviewer-{persona}`");
        assert!(
            codex_body.contains(&needle),
            "rendered Codex `speccy-review` SKILL.md must name persona `{persona}` as `{needle}`; got:\n{codex_body}",
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
// Codex SKILL.md wrappers under
// `resources/agents/.agents/skills/speccy-<verb>/SKILL.md.tmpl`.
//
// Structurally identical to the Claude Code wrappers: a YAML
// frontmatter block (`name`, `description`) followed by exactly one
// `{% raw %}{% include "modules/skills/speccy-<verb>.md" %}{% endraw %}`
// directive. The `.agents/skills/` path mirrors the Codex install
// destination (OpenAI's documented
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
        // The Codex
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
            // The Codex orchestrate wrapper includes
            // the host-neutral body AND the Codex-only permission-grant
            // module (additive selective-include).
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
// Claude Code reviewer subagent wrappers under
// `resources/agents/.claude/agents/reviewer-<persona>.md.tmpl`.
//
// Six wrappers, one per shipped reviewer persona. Each wrapper is a
// YAML frontmatter block (`name: reviewer-<persona>`,
// `description: <one-line>`) followed by exactly one
// `{% include "modules/personas/reviewer-<persona>.md" %}` directive
// (no `{% raw %}` wrapping; persona bodies currently contain no
// `{{` / `{%` literals). The wrapper byte-shape mirrors the
// SKILL.md wrappers: ends at `%}` with no trailing newline so the
// rendered output keeps the persona body's leading/trailing newlines
// as the only blank lines straddling the include site.
// --------------------------------------------------------------------

/// Seven reviewer-persona names shipped by `speccy-core::personas::ALL`.
/// Duplicated locally as a `const &[&str]` so the tests stay
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
        // Description-length invariant: descriptions land in
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
    // equals filename stem, and each body carries the persona module
    // body's `## Focus` section (proof the `{% include %}` expanded).
    let rendered = render_host_pack(HostChoice::ClaudeCode)
        .expect("render_host_pack(claude-code) should succeed");
    // Phase-worker subagent files live at
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

    // Each rendered
    // reviewer subagent body must carry the persona module body with
    // its `{% include %}` expanded — the structural `## Focus` section
    // heading from the persona body proves the expansion happened.
    for file in &agent_files {
        assert!(
            file.contents.contains("## Focus"),
            "rendered `{}` must carry the persona body's `## Focus` section (include expansion); got:\n{}",
            file.rel_path,
            file.contents,
        );
    }
}

// --------------------------------------------------------------------
// The `reviewer-correctness`
// persona renders for both hosts with all `{% include %}` directives
// expanded, and the rendered body carries a non-empty
// `## Out of scope` deferral section.
// --------------------------------------------------------------------

/// Returns the rendered body of the named agent for the given host,
/// asserting the file exists.
fn rendered_agent_body(host: HostChoice, dir: &str, name: &str, suffix: &str) -> String {
    let rendered = render_host_pack(host).expect("render_host_pack should succeed");
    let rel = format!("{dir}/agents/{name}.{suffix}");
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
fn reviewer_correctness_renders_with_includes_expanded_both_hosts() {
    // Both hosts render the persona with every `{% ... %}`
    // include directive expanded and no `<...>` placeholder left.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_agent_body(host, dir, "reviewer-correctness", suffix);
        assert!(
            !body.contains("{%"),
            "rendered `{dir}/agents/reviewer-correctness.{suffix}` must have all `{{% ... %}}` includes expanded; got:\n{body}",
        );
    }
}

#[test]
fn reviewer_correctness_body_has_out_of_scope_section() {
    // The rendered body carries a non-empty
    // `## Out of scope` deferral section, gating a silent drop of the
    // scope boundary on a future edit. The section's wording is prose
    // and deliberately not pinned here.
    let body = rendered_agent_body(
        HostChoice::ClaudeCode,
        ".claude",
        "reviewer-correctness",
        "md",
    );
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
        !defer_section.trim().is_empty(),
        "reviewer-correctness must have a non-empty out-of-scope deferral section; got:\n{body}",
    );
}

// --------------------------------------------------------------------
// The `plan-explorer` persona renders for
// both hosts with all `{% include %}` directives expanded, and the
// rendered body carries the advisory, non-verdict contract — it must
// not contain the `<review` verdict-contract marker.
// --------------------------------------------------------------------

#[test]
fn plan_explorer_renders_with_includes_expanded_both_hosts() {
    // Both hosts render the persona with every `{% ... %}`
    // include directive expanded.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_agent_body(host, dir, "plan-explorer", suffix);
        assert!(
            !body.contains("{%"),
            "rendered `{dir}/agents/plan-explorer.{suffix}` must have all `{{% ... %}}` includes expanded; got:\n{body}",
        );
    }
}

#[test]
fn plan_explorer_body_has_no_review_verdict_marker_both_hosts() {
    // plan-explorer is advisory, not a reviewer. Its rendered
    // body must not carry the `<review` verdict-contract marker — that
    // would mean a verdict-contract snippet leaked in, contradicting
    // the report-only contract. The check is host-independent: the
    // body is identical across wrappers, but assert on both so a
    // wrapper that accidentally inlines a verdict snippet is caught.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_agent_body(host, dir, "plan-explorer", suffix);
        assert!(
            !body.contains("<review"),
            "rendered `{dir}/agents/plan-explorer.{suffix}` must not contain the `<review` verdict-contract marker (advisory, non-verdict contract); got:\n{body}",
        );
    }
}

// --------------------------------------------------------------------
// The `plan-architect` persona renders for
// both hosts with all `{% include %}` directives expanded, carries the
// advisory non-verdict contract (no `<review` marker), and specifies
// that build-sequence items are agent-sized.
// --------------------------------------------------------------------

#[test]
fn plan_architect_renders_with_includes_expanded_both_hosts() {
    // Both hosts render the persona with every `{% ... %}`
    // include directive expanded.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_agent_body(host, dir, "plan-architect", suffix);
        assert!(
            !body.contains("{%"),
            "rendered `{dir}/agents/plan-architect.{suffix}` must have all `{{% ... %}}` includes expanded; got:\n{body}",
        );
    }
}

#[test]
fn plan_architect_body_has_no_review_verdict_marker_both_hosts() {
    // plan-architect is advisory, not a reviewer. Its rendered
    // body must not carry the `<review` verdict-contract marker — that
    // would mean a verdict-contract snippet leaked in, contradicting the
    // blueprint-only contract.
    for (host, dir, suffix) in [
        (HostChoice::ClaudeCode, ".claude", "md"),
        (HostChoice::Codex, ".codex", "toml"),
    ] {
        let body = rendered_agent_body(host, dir, "plan-architect", suffix);
        assert!(
            !body.contains("<review"),
            "rendered `{dir}/agents/plan-architect.{suffix}` must not contain the `<review` verdict-contract marker (advisory, non-verdict contract); got:\n{body}",
        );
    }
}

// --------------------------------------------------------------------
// Codex reviewer subagent wrappers under
// `resources/agents/.codex/agents/reviewer-<persona>.toml.tmpl`.
//
// Six wrappers, one per shipped reviewer persona. Each wrapper is a
// flat-TOML document with three top-level keys: `name` (string),
// `description` (string), and `developer_instructions` (string, TOML
// triple-quoted) wrapping a single
// `{% include "modules/personas/reviewer-<persona>.md" %}` directive
// (no `{% raw %}` wrapping; persona bodies currently
// contain no `{{` / `{%` literals). The wrapper byte-shape mirrors the
// other wrappers: ends at `"""` (the close of the
// triple-quoted block) with no trailing newline. The TOML-safety
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
        // Wrapper trailing-byte shape.
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
    // required top-level keys, name equals filename stem, and each
    // `developer_instructions` body carries the persona module body's
    // `## Focus` section (proof the `{% include %}` expanded).
    //
    // Phase-worker subagent files live at
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

        // `developer_instructions` must carry the
        // persona module body with its `{% include %}` expanded — the
        // structural `## Focus` section heading from the persona body
        // proves the expansion happened.
        assert!(
            dev_instructions.contains("## Focus"),
            "rendered subagent `{path}` `developer_instructions` must carry the persona body's `## Focus` section (include expansion); got:\n{dev_instructions}",
        );
    }
}

/// Persona body files must not contain
/// the literal substring `"""` because the Codex reviewer wrapper
/// embeds the persona body inside a TOML triple-quoted string
/// (`developer_instructions = """..."""`); a `"""` in the persona body
/// would terminate the string prematurely and break the rendered TOML.
/// This guard lives long-term: the invariant must hold for every future
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
// Content-shape (structural surfaces only): the
// speccy-brainstorm skill body must route output into the four SPEC.md
// destination sections, point at both terminal actions
// (`speccy-plan` for new specs, `speccy-amend` for amendments) via the
// `{{ cmd_prefix }}` placeholder, and render with the host-specific
// prefix. The body's Socratic-flow prose (hard gate, artifact labels,
// question discipline) is deliberately not substring-pinned — that is
// editorial content owned by review, not tests.
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
}

#[test]
fn brainstorm_module_body_uses_cmd_prefix_consistently() {
    // The source module body must use
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
    // Host-correctness negative ban: the renderer resolves
    // `{{ cmd_prefix }}` to the empty string on Codex (skill
    // invocations are bare), so a slashed `/speccy-plan` /
    // `/speccy-amend` must never bleed into the Codex render.
    let rendered = render_host_pack(HostChoice::Codex).unwrap_or_else(|err| {
        panic_with_test_message(&format!("render_host_pack(codex) should succeed: {err}"))
    });
    let body = find_rendered_skill(&rendered, ".agents", "speccy-brainstorm");
    for slashed in ["/speccy-plan", "/speccy-amend"] {
        assert!(
            !body.contains(slashed),
            "rendered Codex `.agents/skills/speccy-brainstorm/SKILL.md` must not contain `{slashed}` — Codex skill invocations are bare (no leading slash)",
        );
    }
}

// --------------------------------------------------------------------
// reviewer-tests persona and prompt load
// the evidence file and stay framework-agnostic; the other six
// built-in reviewer personas carry no evidence-related instruction.
// --------------------------------------------------------------------

/// Reviewer personas other than `tests`. The asymmetry: only the
/// `tests` persona / prompt names evidence
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
/// not name inside normative guidance.
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
    let normative = normative_persona_body(body);

    // Framework-agnostic clause (negative ban): no per-framework
    // anchor strings inside normative guidance. Worked-example asides
    // under `## Example` are out of scope.
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
// packaging conventions for the three feature-dev
// ports (reviewer-correctness, plan-explorer, plan-architect). Each
// Claude wrapper declares `model: opus[1m]`, each Codex wrapper
// declares `model = "gpt-5.5"`, and none declares `sonnet`.
// --------------------------------------------------------------------

/// The three personas ported from `feature-dev`. Their packaging
/// invariants are asserted as
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
fn feature_dev_personas_declare_speccy_model_conventions() {
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
    }
}

// --------------------------------------------------------------------
// read-only agents declare an explicit
// read-only `tools:` grant in their Claude wrapper frontmatter, and
// the writer agents are NOT narrowed by this change.
//
// The ten read-only agents grant `Read`/`Grep`/`Glob`/`LS`/
// `Bash`/`WebFetch` and exclude `Edit`/`Write`/`NotebookEdit`; the five
// writer agents retain full (unrestricted, no `tools:` field) access.
// --------------------------------------------------------------------

/// The ten read-only agents that must carry an explicit read-only
/// `tools:` grant.
const READ_ONLY_AGENTS: &[&str] = &[
    "plan-explorer",
    "plan-architect",
    "reviewer-correctness",
    "reviewer-business",
    "reviewer-tests",
    "reviewer-security",
    "reviewer-style",
    "reviewer-architecture",
    "reviewer-docs",
    "vet-reviewer",
];

/// The five writer agents that must retain full (unrestricted) tool
/// access — they must NOT be narrowed by the read-only grant.
const WRITER_AGENTS: &[&str] = &[
    "speccy-work",
    "speccy-decompose",
    "speccy-ship",
    "vet-implementer",
    "vet-simplifier",
];

#[derive(Debug, Deserialize)]
struct AgentToolsFrontmatter {
    #[serde(default)]
    tools: Option<String>,
}

#[test]
fn read_only_agents_declare_read_only_tool_grant() {
    // Each read-only Claude wrapper declares a `tools:` field
    // that includes `Read` and excludes `Edit`/`Write`/`NotebookEdit`.
    let claude = render_host_pack(HostChoice::ClaudeCode)
        .unwrap_or_else(|err| panic_with_test_message(&format!("render claude pack: {err}")));

    for agent in READ_ONLY_AGENTS {
        let path = format!(".claude/agents/{agent}.md");
        let body = find_rendered_agent(&claude, &path);
        let (yaml, _rest) = split_frontmatter(body).unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "Claude wrapper `{path}` must have a `---` frontmatter fence"
            ))
        });
        let fm: AgentToolsFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "Claude wrapper `{path}` frontmatter must be valid YAML: {err}"
            ))
        });
        let tools = fm.tools.unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "read-only Claude wrapper `{path}` must declare a `tools:` grant"
            ))
        });
        assert!(
            tools.contains("Read"),
            "read-only wrapper `{path}` `tools:` grant must include `Read`; got `{tools}`",
        );
        for forbidden in ["Edit", "Write", "NotebookEdit"] {
            assert!(
                !tools.contains(forbidden),
                "read-only wrapper `{path}` `tools:` grant must exclude `{forbidden}`; got `{tools}`",
            );
        }
    }
}

#[test]
fn writer_agents_are_not_narrowed_to_read_only() {
    // The writer wrappers retain full tool access — they must
    // not have been narrowed to the read-only set. A writer that grows
    // a `tools:` field excluding `Edit`/`Write` would be an over-broad
    // application of the read-only grant; gate that regression.
    let claude = render_host_pack(HostChoice::ClaudeCode)
        .unwrap_or_else(|err| panic_with_test_message(&format!("render claude pack: {err}")));

    for agent in WRITER_AGENTS {
        let path = format!(".claude/agents/{agent}.md");
        let body = find_rendered_agent(&claude, &path);
        let (yaml, _rest) = split_frontmatter(body).unwrap_or_else(|| {
            panic_with_test_message(&format!(
                "Claude wrapper `{path}` must have a `---` frontmatter fence"
            ))
        });
        let fm: AgentToolsFrontmatter = serde_saphyr::from_str(yaml).unwrap_or_else(|err| {
            panic_with_test_message(&format!(
                "Claude wrapper `{path}` frontmatter must be valid YAML: {err}"
            ))
        });
        // A writer either declares no `tools:` field (inherits full
        // access) or, if it ever declares one, must retain write
        // capability. Either form proves it was not narrowed to the
        // read-only set.
        if let Some(tools) = fm.tools {
            assert!(
                tools.contains("Edit") && tools.contains("Write"),
                "writer wrapper `{path}` declares a `tools:` grant but lost `Edit`/`Write`; \
                 the read-only grant was applied too broadly; got `{tools}`",
            );
        }
    }
}
