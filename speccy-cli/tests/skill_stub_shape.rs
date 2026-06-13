#![expect(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Stub-shape invariants.
//!
//! Asserts that for `phase` in {`decompose`, `work`, `ship`}:
//! (i)  each rendered SKILL.md body contains the literal substring
//!      `/agent speccy-<phase>` with the matching phase name and a
//!      reference to the matching agent file path;
//! (ii) the rendered SKILL.md bodies for the stub-delegate phases
//!      (`decompose`, `ship`) do not contain `## Steps` or
//!      `## When to use`; the two `speccy-work` SKILL.md bodies are
//!      recipe-shape and carry both headings.

use camino::Utf8PathBuf;
use regex::Regex;

fn workspace_root() -> Utf8PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let manifest = Utf8PathBuf::from(manifest_dir);
    manifest
        .parent()
        .expect("speccy-core has a parent")
        .to_path_buf()
}

const PINNED_PHASES: &[&str] = &["decompose", "work", "ship"];
// `work` migrated from stub-delegate
// to pure-include shape, so its SKILL.md body now carries the full
// `## When to use` and `## Steps` sections of a recipe skill. The
// stub-only invariants (no `## Steps`, no `## When to use`) no
// longer apply to `work`; the agent-file-pointer and `/agent`
// invocation references still appear in the pure-include body, so
// those assertions still apply uniformly.
const STUB_ONLY_PHASES: &[&str] = &["decompose", "ship"];

/// Test-only failure path. Scoped so the `clippy::panic` expectation
/// is in one place rather than spread across every assertion.
#[expect(
    clippy::panic,
    reason = "test-only fixture lookup; failure is a developer-facing assertion"
)]
fn fail(msg: &str) -> ! {
    panic!("{msg}");
}

/// Read a rendered SKILL.md for a given phase from the Claude Code
/// in-tree host pack (`.claude/skills/speccy-<phase>/SKILL.md`).
fn read_claude_skill(root: &Utf8PathBuf, phase: &str) -> String {
    let path = root.join(format!(".claude/skills/speccy-{phase}/SKILL.md"));
    fs_err::read_to_string(&path).unwrap_or_else(|err| {
        fail(&format!(
            "Claude Code SKILL.md `{path}` must be readable: {err}"
        ))
    })
}

/// Read a rendered SKILL.md for a given phase from the Codex in-tree
/// host pack (`.agents/skills/speccy-<phase>/SKILL.md`).
fn read_codex_skill(root: &Utf8PathBuf, phase: &str) -> String {
    let path = root.join(format!(".agents/skills/speccy-{phase}/SKILL.md"));
    fs_err::read_to_string(&path)
        .unwrap_or_else(|err| fail(&format!("Codex SKILL.md `{path}` must be readable: {err}")))
}

/// (i) Each stub SKILL.md body contains `/agent speccy-<phase>` and
///      a reference to the matching agent file path.
#[test]
fn stub_skill_names_agent_invocation_and_file_claude_code() {
    let root = workspace_root();
    for phase in PINNED_PHASES {
        let skill_body = read_claude_skill(&root, phase);
        let label = format!(".claude/skills/speccy-{phase}/SKILL.md");
        assert!(
            skill_body.contains(&format!("/agent speccy-{phase}")),
            "`{label}` must contain the literal `/agent speccy-{phase}` (ii)",
        );
        let agent_file_ref = format!(".claude/agents/speccy-{phase}.md");
        assert!(
            skill_body.contains(&agent_file_ref),
            "`{label}` must reference the matching agent file path `{agent_file_ref}` (ii)",
        );
    }
}

#[test]
fn stub_skill_names_agent_invocation_and_file_codex() {
    let root = workspace_root();
    for phase in PINNED_PHASES {
        let skill_body = read_codex_skill(&root, phase);
        let label = format!(".agents/skills/speccy-{phase}/SKILL.md");
        assert!(
            skill_body.contains(&format!("/agent speccy-{phase}")),
            "`{label}` must contain the literal `/agent speccy-{phase}` (ii)",
        );
        let agent_file_ref = format!(".codex/agents/speccy-{phase}.toml");
        assert!(
            skill_body.contains(&agent_file_ref),
            "`{label}` must reference the matching agent file path `{agent_file_ref}` (ii)",
        );
    }
}

/// (ii) Each stub SKILL.md body does NOT contain `## Steps` or
///      `## When to use`.
#[test]
fn stub_skill_has_no_steps_or_when_to_use_claude_code() {
    let root = workspace_root();
    for phase in STUB_ONLY_PHASES {
        let skill_body = read_claude_skill(&root, phase);
        let label = format!(".claude/skills/speccy-{phase}/SKILL.md");
        assert!(
            !skill_body.contains("## Steps"),
            "`{label}` must NOT contain `## Steps` — stubs are thin pointers, not full procedures (iii)",
        );
        assert!(
            !skill_body.contains("## When to use"),
            "`{label}` must NOT contain `## When to use` — stubs are thin pointers, not full procedures (iii)",
        );
    }
}

#[test]
fn stub_skill_has_no_steps_or_when_to_use_codex() {
    let root = workspace_root();
    for phase in STUB_ONLY_PHASES {
        let skill_body = read_codex_skill(&root, phase);
        let label = format!(".agents/skills/speccy-{phase}/SKILL.md");
        assert!(
            !skill_body.contains("## Steps"),
            "`{label}` must NOT contain `## Steps` — stubs are thin pointers, not full procedures (iii)",
        );
        assert!(
            !skill_body.contains("## When to use"),
            "`{label}` must NOT contain `## When to use` — stubs are thin pointers, not full procedures (iii)",
        );
    }
}

/// The `speccy-init` SKILL.md files keep their full procedural body
/// (the stub-shape transformation does not apply to init since it
/// has no subagent file to defer to).
#[test]
fn init_skill_stays_full_body_claude_code() {
    let root = workspace_root();
    let path = root.join(".claude/skills/speccy-init/SKILL.md");
    let body = fs_err::read_to_string(&path)
        .expect(".claude/skills/speccy-init/SKILL.md must be readable");
    assert!(
        body.contains("## Steps"),
        ".claude/skills/speccy-init/SKILL.md must carry the full procedural body (## Steps) since init has no subagent to defer to",
    );
    assert!(
        body.contains("## When to use"),
        ".claude/skills/speccy-init/SKILL.md must carry the full procedural body (## When to use) since init has no subagent to defer to",
    );
}

#[test]
fn init_skill_stays_full_body_codex() {
    let root = workspace_root();
    let path = root.join(".agents/skills/speccy-init/SKILL.md");
    let body = fs_err::read_to_string(&path)
        .expect(".agents/skills/speccy-init/SKILL.md must be readable");
    assert!(
        body.contains("## Steps"),
        ".agents/skills/speccy-init/SKILL.md must carry the full procedural body (## Steps) since init has no subagent to defer to",
    );
    assert!(
        body.contains("## When to use"),
        ".agents/skills/speccy-init/SKILL.md must carry the full procedural body (## When to use) since init has no subagent to defer to",
    );
}

/// The three remaining agent templates reference `modules/phases/` not
/// `modules/skills/` in their `{% include %}` directives.
#[test]
fn agent_templates_use_modules_phases_path() {
    let root = workspace_root();
    for phase in PINNED_PHASES {
        let tmpl_path = root.join(format!(
            "resources/agents/.claude/agents/speccy-{phase}.md.tmpl"
        ));
        let contents = fs_err::read_to_string(&tmpl_path).unwrap_or_else(|err| {
            fail(&format!(
                "agent template `{tmpl_path}` must be readable: {err}"
            ))
        });
        let expected = format!("{{% include \"modules/phases/speccy-{phase}.md\" %}}");
        assert!(
            contents.contains(&expected),
            "agent template `{tmpl_path}` must contain `{expected}` (post-rename path)",
        );
        assert!(
            !contents.contains("modules/skills/speccy-"),
            "agent template `{tmpl_path}` must NOT contain `modules/skills/speccy-` — path renamed to `modules/phases/`",
        );
    }
}

/// Agent description prose must not contain stale `context: fork`
/// wording or model/effort tier references in the `description:` field.
///
/// The task spec restricts these to the `description:` YAML field
/// value only — not the entire frontmatter (which legitimately contains
/// `effort: medium`). We extract the description value by finding the
/// `description:` line in the file.
#[test]
fn agent_description_prose_is_clean() {
    // Model/effort tier words must not appear in the description field
    // as standalone words. Anchor on word boundaries so ordinary prose
    // containing them as fragments (`workflow`, `allow`, `highlight`)
    // does not trip the gate.
    let banned = Regex::new(r"(?i)\b(sonnet|opus|haiku|xhigh|medium|high|low|max)\b")
        .expect("hardcoded model/effort tier regex is valid");
    let root = workspace_root();
    for phase in PINNED_PHASES {
        let path = root.join(format!(".claude/agents/speccy-{phase}.md"));
        let contents = fs_err::read_to_string(&path).unwrap_or_else(|err| {
            fail(&format!(
                "Claude Code agent file `{path}` must be readable: {err}"
            ))
        });
        // Extract the raw `description:` line value from frontmatter.
        // The description is on one line (enforced by the frontmatter
        // shape test in skill_packs.rs).
        let description_line = contents
            .lines()
            .find(|l| l.starts_with("description:"))
            .unwrap_or_else(|| {
                fail(&format!(
                    "`.claude/agents/speccy-{phase}.md` must have a `description:` frontmatter field"
                ))
            });
        // Strip the `description:` prefix to get the raw value.
        let description_value = description_line
            .strip_prefix("description:")
            .unwrap_or("")
            .trim();

        assert!(
            !description_value.contains("context: fork"),
            "`.claude/agents/speccy-{phase}.md` description value must not contain `context: fork` (dropped in third Changelog row)",
        );
        if let Some(found) = banned.find(description_value) {
            fail(&format!(
                "`.claude/agents/speccy-{phase}.md` description value must not contain the \
                 model/effort tier word `{}` (description-prose invariant)",
                found.as_str(),
            ));
        }
    }
}
