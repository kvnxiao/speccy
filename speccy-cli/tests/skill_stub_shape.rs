#![expect(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! CHK-010: stub-shape invariants for SPEC-0032 T-009.
//!
//! Asserts that for `phase` in {`tasks`, `work`, `ship`}:
//! (i)  the rendered SKILL.md body byte-length at
//!      `.claude/skills/speccy-<phase>/SKILL.md` is strictly less than
//!      the rendered agent body byte-length at
//!      `.claude/agents/speccy-<phase>.md`, and the same relationship
//!      holds for the Codex side (`.agents/skills/speccy-<phase>/SKILL.md`
//!      vs `.codex/agents/speccy-<phase>.toml`'s `developer_instructions`
//!      value);
//! (ii) each of those six rendered SKILL.md bodies contains the literal
//!      substring `/agent speccy-<phase>` with the matching phase name
//!      and a reference to the matching agent file path;
//! (iii) each of those six rendered SKILL.md bodies does not contain
//!       `## Steps` or `## When to use`.

use camino::Utf8PathBuf;

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

/// Read the rendered Claude Code agent file for a given phase
/// (`.claude/agents/speccy-<phase>.md`) and return its body
/// byte-length (bytes below the frontmatter delimiter).
fn claude_agent_body_len(root: &Utf8PathBuf, phase: &str) -> usize {
    let path = root.join(format!(".claude/agents/speccy-{phase}.md"));
    let contents = fs_err::read_to_string(&path).unwrap_or_else(|err| {
        fail(&format!(
            "Claude Code agent file `{path}` must be readable: {err}"
        ))
    });
    body_after_frontmatter(&contents, path.as_str()).len()
}

/// Extract the `developer_instructions` value from a Codex TOML agent
/// file (`.codex/agents/speccy-<phase>.toml`) and return its byte
/// length. Returns `None` if the file does not exist yet (T-004 is a
/// prerequisite that ships the Codex phase-worker TOML files; this
/// test skips gracefully when T-004 has not run).
fn codex_agent_dev_instructions_len(root: &Utf8PathBuf, phase: &str) -> Option<usize> {
    let path = root.join(format!(".codex/agents/speccy-{phase}.toml"));
    let Ok(contents) = fs_err::read_to_string(&path) else {
        return None;
    };
    let parsed: toml::Value = toml::from_str(&contents).unwrap_or_else(|err| {
        fail(&format!(
            "Codex agent TOML `{path}` must parse as TOML: {err}"
        ))
    });
    Some(
        parsed
            .as_table()
            .and_then(|t| t.get("developer_instructions"))
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| {
                fail(&format!(
                    "Codex agent TOML `{path}` must have a string `developer_instructions` key"
                ))
            })
            .len(),
    )
}

/// Strip the YAML frontmatter from `contents` and return the body
/// slice. Panics with `label` in the message if the frontmatter is
/// absent or malformed.
fn body_after_frontmatter<'a>(contents: &'a str, label: &str) -> &'a str {
    let after_open = contents
        .strip_prefix("---\n")
        .or_else(|| contents.strip_prefix("---\r\n"))
        .unwrap_or_else(|| {
            fail(&format!(
                "file `{label}` must open with a `---` frontmatter fence"
            ))
        });
    let close_idx = after_open.find("\n---").unwrap_or_else(|| {
        fail(&format!(
            "file `{label}` frontmatter must have a closing `---` fence"
        ))
    });
    let after_close = after_open
        .get(close_idx.saturating_add(4)..)
        .unwrap_or_default();
    after_close.strip_prefix('\n').unwrap_or(after_close)
}

/// (i) The stub SKILL.md body is strictly smaller than the agent body.
#[test]
fn stub_skill_body_smaller_than_agent_body_claude_code() {
    let root = workspace_root();
    for phase in PINNED_PHASES {
        let skill_body = read_claude_skill(&root, phase);
        let skill_body_len = body_after_frontmatter(
            &skill_body,
            &format!(".claude/skills/speccy-{phase}/SKILL.md"),
        )
        .len();
        let agent_body_len = claude_agent_body_len(&root, phase);
        assert!(
            skill_body_len < agent_body_len,
            "Claude Code `.claude/skills/speccy-{phase}/SKILL.md` body ({skill_body_len} bytes) \
             must be strictly smaller than `.claude/agents/speccy-{phase}.md` body \
             ({agent_body_len} bytes) — the stub-shape invariant from CHK-010",
        );
    }
}

#[test]
fn stub_skill_body_smaller_than_agent_body_codex() {
    let root = workspace_root();
    for phase in PINNED_PHASES {
        let skill_body = read_codex_skill(&root, phase);
        let skill_body_len = body_after_frontmatter(
            &skill_body,
            &format!(".agents/skills/speccy-{phase}/SKILL.md"),
        )
        .len();
        // T-004 (a prerequisite to T-009) ships the Codex phase-worker
        // TOML files. Skip this half of the assertion when T-004 has
        // not yet run (i.e. the TOML files are absent from the working
        // tree).
        let Some(agent_dev_instructions_len) = codex_agent_dev_instructions_len(&root, phase)
        else {
            continue;
        };
        assert!(
            skill_body_len < agent_dev_instructions_len,
            "Codex `.agents/skills/speccy-{phase}/SKILL.md` body ({skill_body_len} bytes) \
             must be strictly smaller than `.codex/agents/speccy-{phase}.toml` \
             `developer_instructions` ({agent_dev_instructions_len} bytes) — the \
             stub-shape invariant from CHK-010",
        );
    }
}

/// (ii) Each stub SKILL.md body contains `/agent speccy-<phase>` and
///      a reference to the matching agent file path.
#[test]
fn stub_skill_names_agent_invocation_and_file_claude_code() {
    let root = workspace_root();
    for phase in PINNED_PHASES {
        let skill_body = read_claude_skill(&root, phase);
        let label = format!(".claude/skills/speccy-{phase}/SKILL.md");
        assert!(
            skill_body.contains(&format!("/agent speccy-{phase}")),
            "`{label}` must contain the literal `/agent speccy-{phase}` (CHK-010 ii)",
        );
        let agent_file_ref = format!(".claude/agents/speccy-{phase}.md");
        assert!(
            skill_body.contains(&agent_file_ref),
            "`{label}` must reference the matching agent file path `{agent_file_ref}` (CHK-010 ii)",
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
            "`{label}` must contain the literal `/agent speccy-{phase}` (CHK-010 ii)",
        );
        let agent_file_ref = format!(".codex/agents/speccy-{phase}.toml");
        assert!(
            skill_body.contains(&agent_file_ref),
            "`{label}` must reference the matching agent file path `{agent_file_ref}` (CHK-010 ii)",
        );
    }
}

/// (iii) Each stub SKILL.md body does NOT contain `## Steps` or
///       `## When to use`.
#[test]
fn stub_skill_has_no_steps_or_when_to_use_claude_code() {
    let root = workspace_root();
    for phase in PINNED_PHASES {
        let skill_body = read_claude_skill(&root, phase);
        let label = format!(".claude/skills/speccy-{phase}/SKILL.md");
        assert!(
            !skill_body.contains("## Steps"),
            "`{label}` must NOT contain `## Steps` — stubs are thin pointers, not full procedures (CHK-010 iii)",
        );
        assert!(
            !skill_body.contains("## When to use"),
            "`{label}` must NOT contain `## When to use` — stubs are thin pointers, not full procedures (CHK-010 iii)",
        );
    }
}

#[test]
fn stub_skill_has_no_steps_or_when_to_use_codex() {
    let root = workspace_root();
    for phase in PINNED_PHASES {
        let skill_body = read_codex_skill(&root, phase);
        let label = format!(".agents/skills/speccy-{phase}/SKILL.md");
        assert!(
            !skill_body.contains("## Steps"),
            "`{label}` must NOT contain `## Steps` — stubs are thin pointers, not full procedures (CHK-010 iii)",
        );
        assert!(
            !skill_body.contains("## When to use"),
            "`{label}` must NOT contain `## When to use` — stubs are thin pointers, not full procedures (CHK-010 iii)",
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

/// The phase body files exist at `resources/modules/phases/` and the
/// old paths at `resources/modules/skills/speccy-<phase>.md` are gone
/// (for the four phase names).
#[test]
fn phase_body_files_moved_to_modules_phases() {
    let root = workspace_root();
    for phase in ["decompose", "work", "ship", "init"] {
        let new_path = root.join(format!("resources/modules/phases/speccy-{phase}.md"));
        assert!(
            new_path.exists(),
            "`resources/modules/phases/speccy-{phase}.md` must exist after the rename (T-009 CHK-010)",
        );
        let old_path = root.join(format!("resources/modules/skills/speccy-{phase}.md"));
        assert!(
            !old_path.exists(),
            "`resources/modules/skills/speccy-{phase}.md` must NOT exist after the rename — moved to `modules/phases/` (T-009 CHK-010)",
        );
    }
}

/// The `speccy-init.md` agent file must not exist in `.claude/agents/`
/// after T-009 deletes it.
#[test]
fn speccy_init_agent_file_deleted() {
    let root = workspace_root();
    let claude_path = root.join(".claude/agents/speccy-init.md");
    assert!(
        !claude_path.exists(),
        "`.claude/agents/speccy-init.md` must NOT exist after T-009 — the init phase has no pinned subagent (DEC-009 / REQ-010)",
    );
    let tmpl_path = root.join("resources/agents/.claude/agents/speccy-init.md.tmpl");
    assert!(
        !tmpl_path.exists(),
        "`resources/agents/.claude/agents/speccy-init.md.tmpl` must NOT exist after T-009 — deleted along with the rendered file",
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
            "agent template `{tmpl_path}` must contain `{expected}` (post-rename path) (T-009 CHK-010)",
        );
        assert!(
            !contents.contains("modules/skills/speccy-"),
            "agent template `{tmpl_path}` must NOT contain `modules/skills/speccy-` — path renamed to `modules/phases/` (T-009 CHK-010)",
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
            "`.claude/agents/speccy-{phase}.md` description value must not contain `context: fork` (dropped in third Changelog row / DEC-001)",
        );
        // Model/effort tier words must not appear in the description field.
        for banned in [
            "Sonnet", "Opus", "Haiku", "xhigh", "medium", "high", "low", " max",
        ] {
            assert!(
                !description_value.contains(banned),
                "`.claude/agents/speccy-{phase}.md` description value must not contain `{banned}` (T-009 CHK-010 description-prose invariant)",
            );
        }
    }
}
