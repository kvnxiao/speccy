#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![allow(
    clippy::panic_in_result_fn,
    reason = "assert!/assert_eq! inside TestResult-returning tests is idiomatic"
)]
//! End-to-end tests for `speccy init` through the binary entry point.

use assert_cmd::Command;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use predicates::str::contains;
use serde::Deserialize;
use speccy_cli::host::HostChoice;
use speccy_cli::render::render_host_pack;
use std::path::Path;
use tempfile::TempDir;

mod common;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Frontmatter shape shared by every rendered SKILL.md file. Mirrors
/// the `RecipeFrontmatter` type in `tests/skill_packs.rs`; the YAML
/// keys (`name`, `description`) are the same for both checks.
#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    description: String,
    #[serde(default)]
    name: Option<String>,
}

/// Split a SKILL.md body into `(yaml_frontmatter, post_fm_body)`,
/// returning `None` if the source does not open with `---\n` and
/// contain a matching close fence. Duplicated locally rather than
/// pulled from `tests/skill_packs.rs` because Cargo treats each
/// integration test as a separate crate.
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

/// Skill names shipped by both host packs, mirrored from
/// `skill_packs::SKILL_NAMES`. Each ships at
/// `<host>/<name>/SKILL.md` in the bundle and at
/// `<dest>/<name>/SKILL.md` in the user's project.
const SKILL_NAMES: [&str; 8] = [
    "speccy-init",
    "speccy-plan",
    "speccy-decompose",
    "speccy-work",
    "speccy-review",
    "speccy-ship",
    "speccy-amend",
    "speccy-brainstorm",
];

struct ProjectFixture {
    _dir: TempDir,
    root: Utf8PathBuf,
}

fn project_with_name(name: &str) -> TestResult<ProjectFixture> {
    let parent = tempfile::tempdir()?;
    let root_std = parent.path().join(name);
    fs_err::create_dir_all(&root_std)?;
    let root = Utf8PathBuf::from_path_buf(root_std)
        .map_err(|p| format!("project root must be UTF-8: {}", p.display()))?;
    Ok(ProjectFixture { _dir: parent, root })
}

fn mkdir(root: &Utf8Path, rel: &str) -> TestResult {
    fs_err::create_dir_all(root.join(rel).as_std_path())?;
    Ok(())
}

fn write_file(root: &Utf8Path, rel: &str, body: &str) -> TestResult {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent.as_std_path())?;
    }
    fs_err::write(path.as_std_path(), body)?;
    Ok(())
}

fn read_file(root: &Utf8Path, rel: &str) -> TestResult<String> {
    let path = root.join(rel);
    Ok(fs_err::read_to_string(path.as_std_path())?)
}

#[test]
fn scaffold_gitkeep() -> TestResult {
    // After a fresh `speccy init`, `.speccy/.gitkeep` exists and a
    // follow-up `speccy status` succeeds against the `.speccy/` marker
    // directory.
    let fx = project_with_name("acme")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init").current_dir(fx.root.as_std_path());
    cmd.assert().success();

    assert!(
        fx.root.join(".speccy/.gitkeep").exists(),
        "speccy init must scaffold `.speccy/.gitkeep`",
    );

    let mut status_cmd = Command::cargo_bin("speccy")?;
    status_cmd.arg("status").current_dir(fx.root.as_std_path());
    status_cmd.assert().success();
    Ok(())
}

#[test]
fn refuse_without_force() -> TestResult {
    // Any file that exists and differs from the planned content
    // triggers exit 1 with `--force` mentioned in stderr.
    let fx = project_with_name("refuse")?;
    mkdir(&fx.root, ".claude")?;
    write_file(
        &fx.root,
        ".claude/skills/speccy-init/SKILL.md",
        "OLD-SHIPPED-CONTENT",
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init").current_dir(fx.root.as_std_path());
    cmd.assert().failure().code(1).stderr(contains("--force"));
    Ok(())
}

#[test]
fn force_overwrites_shipped_files() -> TestResult {
    // `--force` overwrites a shipped file when the on-disk bytes
    // diverge from what `speccy init` would render.
    let fx = project_with_name("force-shipped")?;
    mkdir(&fx.root, ".claude")?;
    write_file(
        &fx.root,
        ".claude/skills/speccy-init/SKILL.md",
        "OLD-SHIPPED-CONTENT",
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--force")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let shipped = read_file(&fx.root, ".claude/skills/speccy-init/SKILL.md")?;
    assert_ne!(
        shipped, "OLD-SHIPPED-CONTENT",
        "--force should refresh .claude/skills/speccy-init/SKILL.md",
    );
    let (yaml, _body) = split_frontmatter(&shipped)
        .ok_or("refreshed SKILL.md must have a `---` frontmatter fence")?;
    let fm: SkillFrontmatter = serde_saphyr::from_str(yaml)
        .map_err(|err| format!("refreshed SKILL.md frontmatter must parse as YAML: {err}"))?;
    assert_eq!(
        fm.name.as_deref(),
        Some("speccy-init"),
        "refreshed SKILL.md `name` field must equal `speccy-init`",
    );
    assert!(
        !fm.description.trim().is_empty(),
        "refreshed SKILL.md `description` must be non-empty",
    );
    Ok(())
}

#[test]
fn force_preserves_user_files() -> TestResult {
    let fx = project_with_name("force-user")?;
    mkdir(&fx.root, ".speccy")?;
    mkdir(&fx.root, ".claude")?;
    write_file(
        &fx.root,
        ".claude/skills/my-personal-skill/SKILL.md",
        "USER-AUTHORED-DO-NOT-TOUCH",
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--force")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let preserved = read_file(&fx.root, ".claude/skills/my-personal-skill/SKILL.md")?;
    assert_eq!(
        preserved, "USER-AUTHORED-DO-NOT-TOUCH",
        "user-authored files in .claude/skills/ must survive --force",
    );
    Ok(())
}

// `.speccy/MEMORY.md` is a repo-owned ledger
// the eject pipeline never enumerates. Like `.speccy/BACKLOG.md`, it sits in
// the never-planned-against bucket: `speccy init` only scaffolds `.speccy/`
// and renders the host pack, so neither a first `init` nor `init --force`
// creates, edits, or deletes it.

#[test]
fn force_preserves_speccy_memory_ledger() -> TestResult {
    // Seed `.speccy/MEMORY.md` with arbitrary non-empty content, run
    // `init --force` for both shipped hosts in the same fixture, and assert
    // the file is byte-identical after each — proving the ledger sits outside
    // the set of files the eject pipeline enumerates.
    let fx = project_with_name("memory-preserve")?;
    let seed =
        "## Learned conventions\n\n- entry one\n- entry two\n\n<garbage> not a real grammar\n";
    write_file(&fx.root, ".speccy/MEMORY.md", seed)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--force")
        .arg("--host")
        .arg("claude-code")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let after_claude = read_file(&fx.root, ".speccy/MEMORY.md")?;
    assert_eq!(
        after_claude, seed,
        "SPEC-0064 REQ-001 / CHK-001: `init --force --host claude-code` must leave .speccy/MEMORY.md byte-identical",
    );

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--force")
        .arg("--host")
        .arg("codex")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let after_codex = read_file(&fx.root, ".speccy/MEMORY.md")?;
    assert_eq!(
        after_codex, seed,
        "SPEC-0064 REQ-001 / CHK-001: `init --force --host codex` must leave .speccy/MEMORY.md byte-identical",
    );
    Ok(())
}

#[test]
fn fresh_init_does_not_create_speccy_memory_ledger() -> TestResult {
    // A fresh `init` in a repo without `.speccy/MEMORY.md` must not create
    // one; its absence is normal and silent.
    let fx = project_with_name("memory-absent")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("claude-code")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    assert!(
        !fx.root.join(".speccy/MEMORY.md").exists(),
        "SPEC-0064 REQ-001: a fresh `speccy init` must not create .speccy/MEMORY.md",
    );
    Ok(())
}

#[test]
fn host_detection_precedence() -> TestResult {
    // 1. --host wins over filesystem signals.
    {
        let fx = project_with_name("flag-wins")?;
        mkdir(&fx.root, ".claude")?;
        mkdir(&fx.root, ".codex")?;
        let mut cmd = Command::cargo_bin("speccy")?;
        cmd.arg("init")
            .arg("--host")
            .arg("codex")
            .current_dir(fx.root.as_std_path());
        cmd.assert().success();
        assert!(
            fx.root.join(".agents/skills/speccy-init/SKILL.md").exists(),
            "--host codex must populate .agents/skills/ regardless of .claude/ presence",
        );
    }

    // 2. .claude/ wins over .codex/ when both present.
    {
        let fx = project_with_name("claude-wins")?;
        mkdir(&fx.root, ".claude")?;
        mkdir(&fx.root, ".codex")?;
        let mut cmd = Command::cargo_bin("speccy")?;
        cmd.arg("init").current_dir(fx.root.as_std_path());
        cmd.assert().success();
        assert!(
            fx.root.join(".claude/skills/speccy-init/SKILL.md").exists(),
            ".claude/ should win autodetect when both present",
        );
        assert!(
            !fx.root.join(".agents/skills/speccy-init/SKILL.md").exists(),
            ".agents/ should NOT be populated when .claude/ won detection",
        );
    }

    // 3. .cursor/ refuses without --host.
    {
        let fx = project_with_name("cursor-only")?;
        mkdir(&fx.root, ".cursor")?;
        let mut cmd = Command::cargo_bin("speccy")?;
        cmd.arg("init").current_dir(fx.root.as_std_path());
        cmd.assert()
            .failure()
            .code(1)
            .stderr(contains("Cursor"))
            .stderr(contains("--host"));
    }

    // 4. Unknown --host value exits 1 and lists supported names.
    {
        let fx = project_with_name("unknown-host")?;
        let mut cmd = Command::cargo_bin("speccy")?;
        cmd.arg("init")
            .arg("--host")
            .arg("nonsense")
            .current_dir(fx.root.as_std_path());
        cmd.assert()
            .failure()
            .code(1)
            .stderr(contains("claude-code"))
            .stderr(contains("codex"));
    }

    Ok(())
}

#[test]
fn copy_claude_code_pack_skill_md() -> TestResult {
    // Each rendered SKILL.md flows through MiniJinja. Assertions match
    // the renderer's contract: the file exists, its YAML frontmatter
    // parses with the right `name`, and the body uses slash-prefixed
    // command references (mirrors the unit-test shape in
    // `src/render.
    // rs::render_host_pack_speccy_plan_contains_slash_prefixed_command`).
    let fx = project_with_name("copy-claude")?;
    mkdir(&fx.root, ".claude")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init").current_dir(fx.root.as_std_path());
    cmd.assert().success();

    for skill in SKILL_NAMES {
        let rel = format!(".claude/skills/{skill}/SKILL.md");
        assert!(
            fx.root.join(&rel).exists(),
            "claude-code pack should drop {rel} (SPEC-0015 REQ-002 + CHK-003)",
        );
        let dest = read_file(&fx.root, &rel)?;
        let (yaml, body) = split_frontmatter(&dest)
            .ok_or_else(|| format!("rendered {rel} must have a `---` frontmatter fence"))?;
        let fm: SkillFrontmatter = serde_saphyr::from_str(yaml)
            .map_err(|err| format!("rendered {rel} frontmatter must parse as YAML: {err}"))?;
        assert_eq!(
            fm.name.as_deref(),
            Some(skill),
            "rendered {rel} `name` field must equal `{skill}`",
        );
        assert!(
            !fm.description.trim().is_empty(),
            "rendered {rel} `description` must be non-empty",
        );
        assert!(
            !body.trim().is_empty(),
            "rendered {rel} body (post-frontmatter) must be non-empty",
        );
    }

    // Slash-prefix invariant: Claude Code's `speccy-plan` skill points
    // the main agent at the `/speccy-decompose` skill as the suggested next
    // step. The renderer must substitute `{{ cmd_prefix }}` to `"/"`.
    let plan_body = read_file(&fx.root, ".claude/skills/speccy-plan/SKILL.md")?;
    assert!(
        plan_body.contains("/speccy-decompose"),
        "rendered .claude/skills/speccy-plan/SKILL.md must contain `/speccy-decompose`",
    );

    Ok(())
}

#[test]
fn copy_codex_pack_skill_md() -> TestResult {
    // Each rendered SKILL.md flows through MiniJinja. Assertions mirror
    // the unit-test shape in
    // `src/render.rs::render_host_pack_codex_speccy_plan_uses_bare_command`:
    // file exists, YAML frontmatter parses, and command references are
    // bare (no slash prefix) per Codex's convention.
    let fx = project_with_name("copy-codex")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("codex")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    assert!(
        fx.root.join(".agents/skills").is_dir(),
        ".agents/skills/ must be created when --host codex is passed",
    );
    for skill in SKILL_NAMES {
        let rel = format!(".agents/skills/{skill}/SKILL.md");
        assert!(
            fx.root.join(&rel).exists(),
            "codex pack should drop {rel} (SPEC-0015 REQ-002 + CHK-004)",
        );
        let dest = read_file(&fx.root, &rel)?;
        let (yaml, body) = split_frontmatter(&dest)
            .ok_or_else(|| format!("rendered {rel} must have a `---` frontmatter fence"))?;
        let fm: SkillFrontmatter = serde_saphyr::from_str(yaml)
            .map_err(|err| format!("rendered {rel} frontmatter must parse as YAML: {err}"))?;
        assert_eq!(
            fm.name.as_deref(),
            Some(skill),
            "rendered {rel} `name` field must equal `{skill}`",
        );
        assert!(
            !fm.description.trim().is_empty(),
            "rendered {rel} `description` must be non-empty",
        );
        assert!(
            !body.trim().is_empty(),
            "rendered {rel} body (post-frontmatter) must be non-empty",
        );
    }

    // No-slash-prefix invariant: Codex's `speccy-plan` skill references
    // `speccy-decompose` without the leading `/` (the renderer substitutes
    // `{{ cmd_prefix }}` to the empty string).
    let plan_body = read_file(&fx.root, ".agents/skills/speccy-plan/SKILL.md")?;
    assert!(
        plan_body.contains("speccy-decompose"),
        "rendered .agents/skills/speccy-plan/SKILL.md must contain `speccy-decompose`",
    );
    assert!(
        !plan_body.contains("/speccy-decompose"),
        "rendered .agents/skills/speccy-plan/SKILL.md must NOT contain `/speccy-decompose` (Codex uses bare command names)",
    );
    Ok(())
}

#[test]
fn t009_claude_code_reviewer_subagents_land_at_dot_claude_agents() -> TestResult {
    // `speccy init --host claude-code`
    // materialises one `.claude/agents/reviewer-<persona>.md` file per
    // shipped reviewer persona, each opening with `---` (YAML fence)
    // and each parseable.
    let fx = project_with_name("t009-claude-agents")?;
    mkdir(&fx.root, ".claude")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init").current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let personas: [&str; 7] = [
        "business",
        "tests",
        "security",
        "style",
        "correctness",
        "architecture",
        "docs",
    ];
    for persona in personas {
        let rel = format!(".claude/agents/reviewer-{persona}.md");
        let body = read_file(&fx.root, &rel)?;
        assert!(
            body.starts_with("---\n") || body.starts_with("---\r\n"),
            "rendered subagent {rel} must open with `---`",
        );
        let (yaml, content_body) = split_frontmatter(&body).ok_or_else(|| {
            format!("rendered subagent {rel} must have a `---` frontmatter fence")
        })?;
        let fm: SkillFrontmatter = serde_saphyr::from_str(yaml).map_err(|err| {
            format!("rendered subagent {rel} frontmatter must parse as YAML: {err}")
        })?;
        let expected_name = format!("reviewer-{persona}");
        assert_eq!(
            fm.name.as_deref(),
            Some(expected_name.as_str()),
            "rendered subagent {rel} `name` field must equal `{expected_name}`",
        );
        assert!(
            !fm.description.trim().is_empty(),
            "rendered subagent {rel} `description` must be non-empty",
        );
        assert!(
            !content_body.trim().is_empty(),
            "rendered subagent {rel} body (post-frontmatter) must be non-empty",
        );
    }

    // The security reviewer's body must carry the persona module body
    // with its `{% include %}` expanded — the structural `## Focus`
    // section heading from the persona body proves the expansion
    // happened.
    let security = read_file(&fx.root, ".claude/agents/reviewer-security.md")?;
    assert!(
        security.contains("## Focus"),
        "rendered .claude/agents/reviewer-security.md must carry the persona body's `## Focus` section (include expansion); got:\n{security}",
    );
    Ok(())
}

#[test]
fn t010_codex_reviewer_subagents_land_at_dot_codex_agents() -> TestResult {
    // `speccy init --host codex`
    // materialises one `.codex/agents/reviewer-<persona>.toml` file per
    // shipped reviewer persona, each parseable as flat TOML with the
    // three required top-level keys (`name`, `description`,
    // `developer_instructions`).
    let fx = project_with_name("t010-codex-agents")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("codex")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let personas: [&str; 7] = [
        "business",
        "tests",
        "security",
        "style",
        "correctness",
        "architecture",
        "docs",
    ];
    for persona in personas {
        let rel = format!(".codex/agents/reviewer-{persona}.toml");
        let body = read_file(&fx.root, &rel)?;
        let parsed: toml::Value = toml::from_str(&body)
            .map_err(|err| format!("rendered subagent {rel} must parse as TOML: {err}"))?;
        let table = parsed
            .as_table()
            .ok_or_else(|| format!("rendered subagent {rel} must be a top-level TOML table"))?;
        let expected_name = format!("reviewer-{persona}");
        let name = table
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("rendered subagent {rel} must have a string `name` key"))?;
        assert_eq!(
            name, expected_name,
            "rendered subagent {rel} `name` field must equal `{expected_name}`",
        );
        let description = table
            .get("description")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                format!("rendered subagent {rel} must have a string `description` key")
            })?;
        assert!(
            !description.trim().is_empty(),
            "rendered subagent {rel} `description` must be non-empty",
        );
        let dev_instructions = table
            .get("developer_instructions")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                format!("rendered subagent {rel} must have a string `developer_instructions` key")
            })?;
        assert!(
            !dev_instructions.trim().is_empty(),
            "rendered subagent {rel} `developer_instructions` must be non-empty",
        );
    }

    // The security reviewer's `developer_instructions` body must carry
    // the persona module body with its `{% include %}` expanded — the
    // structural `## Focus` section heading from the persona body
    // proves the expansion happened.
    let security_body = read_file(&fx.root, ".codex/agents/reviewer-security.toml")?;
    let security_parsed: toml::Value = toml::from_str(&security_body)
        .map_err(|err| format!("rendered reviewer-security.toml must parse as TOML: {err}"))?;
    let security_dev = security_parsed
        .as_table()
        .and_then(|t| t.get("developer_instructions"))
        .and_then(toml::Value::as_str)
        .ok_or("rendered reviewer-security.toml must have a string `developer_instructions` key")?;
    assert!(
        security_dev.contains("## Focus"),
        "rendered .codex/agents/reviewer-security.toml `developer_instructions` must carry the persona body's `## Focus` section (include expansion); got:\n{security_dev}",
    );
    Ok(())
}

// `.speccy/skills/` is no longer written by
// `speccy init`. Pre-existing files inside that subtree (from an older
// init or from manual user creation) are left alone — init
// neither rewrites nor deletes them.

#[test]
fn t003_init_does_not_create_speccy_skills_dir() -> TestResult {
    let fx = project_with_name("t003-no-speccy-skills")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("claude-code")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let skills_dir = fx.root.join(".speccy").join("skills");
    assert!(
        !skills_dir.exists(),
        "SPEC-0027 REQ-001: `speccy init` against an empty workspace must not create {skills_dir}",
    );
    Ok(())
}

#[test]
fn t003_init_plan_summary_does_not_mention_speccy_skills() -> TestResult {
    let fx = project_with_name("t003-no-speccy-skills-stdout")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("claude-code")
        .current_dir(fx.root.as_std_path());
    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output)?;
    assert!(
        !stdout.contains(".speccy/skills/"),
        "SPEC-0027 REQ-001: init plan summary must contain no `.speccy/skills/` reference; got:\n{stdout}",
    );
    Ok(())
}

#[test]
fn t003_init_force_preserves_pre_existing_speccy_skills_overrides() -> TestResult {
    let fx = project_with_name("t003-preserve-orphan-skills")?;
    let pre_existing = "pre-existing override\n";
    write_file(
        &fx.root,
        ".speccy/skills/personas/reviewer-business.md",
        pre_existing,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("claude-code")
        .arg("--force")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let after = read_file(&fx.root, ".speccy/skills/personas/reviewer-business.md")?;
    assert_eq!(
        after, pre_existing,
        "SPEC-0027 REQ-001 + DEC-003: `init --force` must leave pre-existing `.speccy/skills/` files byte-for-byte untouched; init writes nothing into the subtree and deletes nothing from it",
    );
    Ok(())
}

// Host-native reviewer files participate in the
// uniform Create / Unchanged / Conflict classification with every
// other rendered host-pack file. Under `--force`, a reviewer file
// that differs from the shipped bundle is overwritten; the prior
// Skip-on-exists carve-out is removed.

#[test]
fn t002_claude_reviewer_agent_files_overwrite_user_edits_under_force() -> TestResult {
    let fx = project_with_name("t002-claude-overwrite-edits")?;
    mkdir(&fx.root, ".claude")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init").current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let rel = ".claude/agents/reviewer-business.md";
    let initial = read_file(&fx.root, rel)?;
    assert!(
        initial.contains("# Reviewer Persona: Business"),
        "first init must drop the shipped persona body at {rel}; got:\n{initial}",
    );

    let sentinel = "\n# sentinel-edit-12345\n";
    let mut edited = initial.clone();
    edited.push_str(sentinel);
    write_file(&fx.root, rel, &edited)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--force")
        .current_dir(fx.root.as_std_path());
    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output)?;

    let after = read_file(&fx.root, rel)?;
    assert!(
        !after.contains("sentinel-edit-12345"),
        "SPEC-0044 REQ-001 / CHK-001: `speccy init --force` must overwrite the user edit at {rel}; sentinel still present in:\n{after}",
    );
    assert_eq!(
        after, initial,
        "SPEC-0044 REQ-001 / CHK-001: after `--force` the file must match the shipped bundle byte-for-byte",
    );
    assert!(
        stdout
            .lines()
            .any(|l| l.trim_start().starts_with("(!) overwritten")
                && l.contains(".claude/agents/reviewer-business.md")),
        "SPEC-0044 REQ-001 / CHK-001: plan summary must show `(!) overwritten` for {rel}; got:\n{stdout}",
    );
    Ok(())
}

#[test]
fn t002_claude_reviewer_agent_files_recreate_when_deleted() -> TestResult {
    // Deletion still triggers a fresh render under
    // the uniform classification (Create on absent applies to every
    // shipped file, with or without `--force`).
    let fx = project_with_name("t002-claude-recreate")?;
    mkdir(&fx.root, ".claude")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init").current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let rel = ".claude/agents/reviewer-business.md";
    fs_err::remove_file(fx.root.join(rel).as_std_path())?;
    assert!(
        !fx.root.join(rel).exists(),
        "pre-condition: {rel} must be removed before the re-init",
    );

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--force")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let restored = read_file(&fx.root, rel)?;
    assert!(
        restored.contains("# Reviewer Persona: Business"),
        "SPEC-0044 REQ-001: `init` after deletion must recreate {rel} from the shipped bundle (Create on absent); got:\n{restored}",
    );
    Ok(())
}

#[test]
fn t002_codex_reviewer_agent_files_overwrite_user_edits_under_force() -> TestResult {
    let fx = project_with_name("t002-codex-overwrite-edits")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("codex")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let rel = ".codex/agents/reviewer-security.toml";
    let initial = read_file(&fx.root, rel)?;
    let sentinel = "\n# sentinel-edit-67890\n";
    let mut edited = initial.clone();
    edited.push_str(sentinel);
    write_file(&fx.root, rel, &edited)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("codex")
        .arg("--force")
        .current_dir(fx.root.as_std_path());
    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output)?;

    let after = read_file(&fx.root, rel)?;
    assert!(
        !after.contains("sentinel-edit-67890"),
        "SPEC-0044 REQ-001 / CHK-002: `speccy init --force --host codex` must overwrite the user edit at {rel}; sentinel still present in:\n{after}",
    );
    assert_eq!(
        after, initial,
        "SPEC-0044 REQ-001 / CHK-002: after `--force` the file must match the shipped bundle byte-for-byte",
    );
    assert!(
        stdout
            .lines()
            .any(|l| l.trim_start().starts_with("(!) overwritten")
                && l.contains(".codex/agents/reviewer-security.toml")),
        "SPEC-0044 REQ-001 / CHK-002: plan summary must show `(!) overwritten` for {rel}; got:\n{stdout}",
    );

    fs_err::remove_file(fx.root.join(rel).as_std_path())?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("codex")
        .arg("--force")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();
    let restored = read_file(&fx.root, rel)?;
    assert!(
        restored.contains("name = \"reviewer-security\""),
        "SPEC-0044 REQ-001: `init --force --host codex` after deletion must recreate {rel} with the shipped Codex frontmatter",
    );
    Ok(())
}

#[test]
fn exit_codes() -> TestResult {
    // 0 on success.
    {
        let fx = project_with_name("exit-zero")?;
        let mut cmd = Command::cargo_bin("speccy")?;
        cmd.arg("init").current_dir(fx.root.as_std_path());
        cmd.assert().success().code(0);
    }

    // 1 when a shipped file exists with differing content (conflict).
    // Per-file conflict detection: only a differing file triggers exit 1.
    {
        let fx = project_with_name("exit-one-conflict")?;
        write_file(
            &fx.root,
            ".claude/skills/speccy-init/SKILL.md",
            "differing-content\n",
        )?;
        let mut cmd = Command::cargo_bin("speccy")?;
        cmd.arg("init").current_dir(fx.root.as_std_path());
        cmd.assert().failure().code(1);
    }

    // 1 on unknown --host value.
    {
        let fx = project_with_name("exit-one-host")?;
        let mut cmd = Command::cargo_bin("speccy")?;
        cmd.arg("init")
            .arg("--host")
            .arg("emacs")
            .current_dir(fx.root.as_std_path());
        cmd.assert().failure().code(1);
    }

    // 1 on .cursor/ detection without --host.
    {
        let fx = project_with_name("exit-one-cursor")?;
        mkdir(&fx.root, ".cursor")?;
        let mut cmd = Command::cargo_bin("speccy")?;
        cmd.arg("init").current_dir(fx.root.as_std_path());
        cmd.assert().failure().code(1);
    }

    // 2 on internal I/O failure: `.speccy` present as a regular file
    // forces create_dir_all to error out partway through scaffold.
    {
        let fx = project_with_name("exit-two-io")?;
        write_file(&fx.root, ".speccy", "not-a-directory")?;
        let mut cmd = Command::cargo_bin("speccy")?;
        cmd.arg("init").current_dir(fx.root.as_std_path());
        cmd.assert().failure().code(2);
    }

    Ok(())
}

// --------------------------------------------------------------------
// Dogfood byte-identity, renderer idempotency, and
// unsubstituted-token guard.
// --------------------------------------------------------------------

/// Workspace root, derived from `CARGO_MANIFEST_DIR` (the `speccy-cli`
/// crate dir) by walking one level up. The committed dogfood outputs
/// live under this root at `.claude/`, `.codex/`, `.agents/`, and
/// `.speccy/skills/`.
fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        Path::to_path_buf,
    )
}

#[test]
fn dogfood_outputs_match_committed_tree() -> TestResult {
    // Rendered outputs from `render_host_pack` for both hosts
    // must match the committed dogfood tree byte-for-byte; the
    // host-neutral persona and prompt bundles copied to `.speccy/skills/`
    // by `speccy init` must also match the committed `.speccy/skills/`
    // tree. Drift here is what the CI workflow's `git diff --exit-code`
    // step catches in production; this test catches it at `cargo test`
    // time.
    let root = workspace_root();

    for host in [HostChoice::ClaudeCode, HostChoice::Codex] {
        let rendered = render_host_pack(host)
            .map_err(|err| format!("render_host_pack({host:?}) should succeed: {err}"))?;
        for file in &rendered {
            let committed_path = root.join(file.rel_path.as_str());
            let committed = fs_err::read_to_string(&committed_path).map_err(|err| {
                format!(
                    "committed dogfood file `{}` must be readable (run \
                     `speccy init --force --host claude-code` and \
                     `speccy init --force --host codex` to refresh): {err}",
                    committed_path.display(),
                )
            })?;
            assert_eq!(
                committed, file.contents,
                "committed dogfood `{}` differs from the renderer output; \
                 run `speccy init --force --host claude-code` and \
                 `speccy init --force --host codex` locally and commit the \
                 resulting changes",
                file.rel_path,
            );
        }
    }

    // The persona body is delivered via
    // `.claude/agents/reviewer-<persona>.md` (and the Codex equivalent),
    // which `render_host_pack` covers above.

    Ok(())
}

#[test]
fn render_is_idempotent() -> TestResult {
    // Rendering the full host pack twice must produce the
    // same `RenderedFile` set in the same order with byte-identical
    // contents. A divergence here means the renderer is non-deterministic
    // (e.g. iterates an unordered set or interpolates a time/random
    // value); committing the output to the dogfood tree would then
    // produce spurious diffs on every CI run.
    for host in [HostChoice::ClaudeCode, HostChoice::Codex] {
        let first = render_host_pack(host)
            .map_err(|err| format!("first render_host_pack({host:?}) should succeed: {err}"))?;
        let second = render_host_pack(host)
            .map_err(|err| format!("second render_host_pack({host:?}) should succeed: {err}"))?;
        assert_eq!(
            first.len(),
            second.len(),
            "render_host_pack({host:?}) emitted a different number of files across passes (first={}, second={})",
            first.len(),
            second.len(),
        );
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(
                a.rel_path, b.rel_path,
                "render_host_pack({host:?}) re-ordered output files between passes",
            );
            assert_eq!(
                a.contents, b.contents,
                "render_host_pack({host:?}) produced different bytes for `{}` between passes",
                a.rel_path,
            );
        }
    }
    Ok(())
}

#[test]
fn rendered_outputs_have_no_unsubstituted_tokens() -> TestResult {
    // No rendered output file may contain the literal
    // substrings `{{` or `{%` outside fenced code blocks. Inside a
    // ```fenced block the substring is allowed (example template
    // syntax in documentation is legitimate); outside, it indicates
    // an unsubstituted MiniJinja directive that escaped the renderer.
    for host in [HostChoice::ClaudeCode, HostChoice::Codex] {
        let rendered = render_host_pack(host)
            .map_err(|err| format!("render_host_pack({host:?}) should succeed: {err}"))?;
        for file in &rendered {
            assert_no_unsubstituted_token(&file.contents, file.rel_path.as_str(), "{{");
            assert_no_unsubstituted_token(&file.contents, file.rel_path.as_str(), "{%");
        }
    }
    Ok(())
}

/// Assert that `needle` does not appear in `body` outside any fenced
/// code block opened with a triple-backtick. The fence tracker mirrors
/// the one in `tests/skill_packs.rs::contains_speccy_command_in_code_fence`
/// but operates as a negative check: every line outside a fence must
/// not contain `needle`.
fn assert_no_unsubstituted_token(body: &str, label: &str, needle: &str) {
    let mut in_fence = false;
    for (idx, line) in body.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        assert!(
            !line.contains(needle),
            "rendered `{label}` line {} (1-indexed) contains the unsubstituted MiniJinja token `{needle}` outside a fenced code block: `{line}`",
            idx.saturating_add(1),
        );
    }
}

// --------------------------------------------------------------------
// `speccy init` end-to-end against fresh
// tempdirs renders the per-phase model and effort pin assignments that
// the in-tree dogfood pack already encodes. The full slice covered here
// is: Claude Code phase-worker subagent files,
// Codex phase-worker TOML files, reviewer files on both hosts, and the
// four mechanical-phase SKILL.md files (plus speccy-review's SKILL.md)
// carrying no pin keys. The drift check enforced by
// `dogfood_outputs_match_committed_tree` already locks template-vs-
// rendered byte identity for these files; this test locks the
// fresh-tempdir render outcome the user actually experiences when they
// run `speccy init` in their own project.
// --------------------------------------------------------------------

/// Pinned-agent frontmatter shared by Claude Code phase-worker and
/// reviewer subagent files. The `[1m]` 1M-context-window suffix on
/// `model` parses as a plain YAML scalar via `serde_saphyr`.
#[derive(Debug, Deserialize)]
struct ClaudePinFrontmatter {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    effort: Option<String>,
}

/// Mechanical-phase and `speccy-review` SKILL.md frontmatter. These
/// files carry zero pin keys; the
/// `Option<String>` fields surface as `None` when the keys are absent
/// and as `Some` when a regression leaks one in.
#[derive(Debug, Deserialize)]
struct SkillNoPinsFrontmatter {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    effort: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    agent: Option<String>,
}

fn parse_claude_pins(root: &Utf8Path, rel: &str) -> TestResult<ClaudePinFrontmatter> {
    let body = read_file(root, rel)?;
    let (yaml, _) = split_frontmatter(&body)
        .ok_or_else(|| format!("rendered {rel} must have a `---` frontmatter fence"))?;
    let fm: ClaudePinFrontmatter = serde_saphyr::from_str(yaml)
        .map_err(|err| format!("rendered {rel} frontmatter must parse as YAML: {err}"))?;
    Ok(fm)
}

fn parse_no_pin_skill(root: &Utf8Path, rel: &str) -> TestResult<SkillNoPinsFrontmatter> {
    let body = read_file(root, rel)?;
    let (yaml, _) = split_frontmatter(&body)
        .ok_or_else(|| format!("rendered {rel} must have a `---` frontmatter fence"))?;
    let fm: SkillNoPinsFrontmatter = serde_saphyr::from_str(yaml)
        .map_err(|err| format!("rendered {rel} frontmatter must parse as YAML: {err}"))?;
    Ok(fm)
}

fn assert_no_pin_keys(rel: &str, fm: &SkillNoPinsFrontmatter) {
    assert!(
        fm.model.is_none(),
        "rendered {rel} must carry no `model:` key (REQ-001/REQ-002 unpinned SKILL.md); got {:?}",
        fm.model,
    );
    assert!(
        fm.effort.is_none(),
        "rendered {rel} must carry no `effort:` key (REQ-001/REQ-002 unpinned SKILL.md); got {:?}",
        fm.effort,
    );
    assert!(
        fm.context.is_none(),
        "rendered {rel} must carry no `context:` key (DEC-001 dropped auto-fork); got {:?}",
        fm.context,
    );
    assert!(
        fm.agent.is_none(),
        "rendered {rel} must carry no `agent:` key (DEC-001 dropped auto-fork); got {:?}",
        fm.agent,
    );
}

/// Assert a phase-worker SKILL.md rendered file carries a thin-stub
/// body: the body references the matching agent
/// file path and the `/agent speccy-<phase>` invocation pointer, and
/// it is short (no leakage of the full phase prompt body).
fn assert_thin_stub_body(root: &Utf8Path, rel: &str, agent_path: &str, phase: &str) -> TestResult {
    let body = read_file(root, rel)?;
    let (_, post_fm) = split_frontmatter(&body)
        .ok_or_else(|| format!("rendered {rel} must have a `---` frontmatter fence"))?;
    assert!(
        post_fm.contains(agent_path),
        "rendered {rel} thin-stub body must reference `{agent_path}`; got:\n{post_fm}",
    );
    let invocation = format!("/agent speccy-{phase}");
    assert!(
        post_fm.contains(&invocation),
        "rendered {rel} thin-stub body must mention the `{invocation}` invocation pointer; got:\n{post_fm}",
    );
    // Exclude lines that fall inside the `reconcile-policy` shared-partial
    // marker block from the non-empty-line count. The shared partial is
    // inlined verbatim into a handful of phase-worker stubs;
    // those marker-bounded regions are explicit, auditable exemptions
    // from the "stub body stays short" cap. The cap still polices any
    // prose outside the markers — full-body leakage is still caught.
    let non_empty_lines = common::non_blank_line_count_outside_shared_markers(post_fm);
    assert!(
        non_empty_lines < 12,
        "rendered {rel} thin-stub body must be short (< 12 non-empty lines outside shared-marker blocks), got {non_empty_lines} lines; full body has leaked",
    );
    Ok(())
}

/// Assert the `speccy-init` SKILL.md rendered file retains its full
/// body. `speccy-init` is exempt from the thin-stub
/// transformation because there is no pinned subagent to delegate to.
fn assert_init_full_body(root: &Utf8Path, rel: &str) -> TestResult {
    let body = read_file(root, rel)?;
    let (_, post_fm) = split_frontmatter(&body)
        .ok_or_else(|| format!("rendered {rel} must have a `---` frontmatter fence"))?;
    let non_empty_lines = post_fm.lines().filter(|l| !l.trim().is_empty()).count();
    assert!(
        non_empty_lines >= 20,
        "rendered {rel} must retain full body (>= 20 non-empty lines), got {non_empty_lines}; the thin-stub transformation must not apply to speccy-init",
    );
    assert!(
        !post_fm.contains("/agent speccy-init"),
        "rendered {rel} must not delegate to `/agent speccy-init` (REQ-010 / DEC-009: no speccy-init subagent on either host); got:\n{post_fm}",
    );
    Ok(())
}

const CLAUDE_PINNED_PHASES: [&str; 3] = ["decompose", "work", "ship"];
// `speccy-work` migrated from
// stub-delegate to pure-include, so its SKILL.md body now expands
// the full `modules/skills/speccy-work.md` body instead of a thin
// delegation pointer. The pinned-phase agent pins (model/effort)
// still apply to `work`; only the SKILL.md body shape changed.
// Stub-delegate body assertions therefore cover only decompose and
// ship, both of which remain stub-delegate.
const CLAUDE_STUB_DELEGATE_PHASES: [&str; 2] = ["decompose", "ship"];
const CLAUDE_OPUS_XHIGH_REVIEWERS: [&str; 3] = ["business", "tests", "architecture"];
const CLAUDE_OPUS_HIGH_REVIEWERS: [&str; 1] = ["security"];
const CLAUDE_SONNET_MEDIUM_REVIEWERS: [&str; 2] = ["style", "docs"];
const CODEX_HIGH_REVIEWERS: [&str; 4] = ["business", "tests", "architecture", "security"];
const CODEX_LOW_REVIEWERS: [&str; 2] = ["style", "docs"];

fn read_codex_toml(root: &Utf8Path, rel: &str) -> TestResult<toml::Table> {
    let body = read_file(root, rel)?;
    let parsed: toml::Table =
        toml::from_str(&body).map_err(|err| format!("rendered {rel} must parse as TOML: {err}"))?;
    Ok(parsed)
}

fn assert_codex_pin(table: &toml::Table, rel: &str, expected_effort: &str) {
    let model = table
        .get("model")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    assert_eq!(
        model, "gpt-5.5",
        "rendered {rel} must carry `model = \"gpt-5.5\"`; got `{model}`",
    );
    let effort = table
        .get("model_reasoning_effort")
        .and_then(toml::Value::as_str)
        .unwrap_or_default();
    assert_eq!(
        effort, expected_effort,
        "rendered {rel} must carry `model_reasoning_effort = \"{expected_effort}\"`; got `{effort}`",
    );
}

#[test]
fn t007_init_renders_claude_code_pin_assignments_matching_dogfood_pack() -> TestResult {
    // Claude Code half: `speccy init` against a
    // fresh empty directory must materialise the per-phase model/effort
    // pin assignments that the in-tree dogfood pack already encodes.
    let fx = project_with_name("t007-claude-pins")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("claude-code")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    // Each phase has its own pin; check them individually.
    let claude_phase_pins: &[(&str, &str, &str)] = &[
        ("decompose", "opus[1m]", "medium"),
        ("work", "opus[1m]", "high"),
        ("ship", "sonnet[1m]", "medium"),
    ];
    for (phase, expected_model, expected_effort) in claude_phase_pins {
        let rel = format!(".claude/agents/speccy-{phase}.md");
        let fm = parse_claude_pins(&fx.root, &rel)?;
        assert_eq!(
            fm.name.as_deref(),
            Some(format!("speccy-{phase}").as_str()),
            "rendered {rel} `name` field must equal `speccy-{phase}`",
        );
        assert_eq!(
            fm.model.as_deref(),
            Some(*expected_model),
            "rendered {rel} must carry `model: {expected_model}`; got {:?}",
            fm.model,
        );
        assert_eq!(
            fm.effort.as_deref(),
            Some(*expected_effort),
            "rendered {rel} must carry `effort: {expected_effort}`; got {:?}",
            fm.effort,
        );
    }

    let init_agent = fx.root.join(".claude/agents/speccy-init.md");
    assert!(
        !init_agent.exists(),
        "DEC-009 / REQ-010: speccy init must not render `.claude/agents/speccy-init.md` (no pinned init subagent); found at `{init_agent}`",
    );

    assert_claude_reviewer_pins(&fx.root)?;

    for phase in ["decompose", "work", "ship", "init"] {
        let rel = format!(".claude/skills/speccy-{phase}/SKILL.md");
        let fm = parse_no_pin_skill(&fx.root, &rel)?;
        assert_no_pin_keys(&rel, &fm);
    }
    let review_fm = parse_no_pin_skill(&fx.root, ".claude/skills/speccy-review/SKILL.md")?;
    assert_no_pin_keys(".claude/skills/speccy-review/SKILL.md", &review_fm);

    for phase in CLAUDE_STUB_DELEGATE_PHASES {
        let rel = format!(".claude/skills/speccy-{phase}/SKILL.md");
        let agent_path = format!(".claude/agents/speccy-{phase}.md");
        assert_thin_stub_body(&fx.root, &rel, &agent_path, phase)?;
    }
    assert_init_full_body(&fx.root, ".claude/skills/speccy-init/SKILL.md")?;
    Ok(())
}

fn assert_claude_reviewer_pins(root: &Utf8Path) -> TestResult {
    let cases: &[(&[&str], &str, &str)] = &[
        (&CLAUDE_OPUS_XHIGH_REVIEWERS, "opus[1m]", "xhigh"),
        (&CLAUDE_OPUS_HIGH_REVIEWERS, "opus[1m]", "high"),
        (&CLAUDE_SONNET_MEDIUM_REVIEWERS, "sonnet[1m]", "medium"),
    ];
    for (personas, expected_model, expected_effort) in cases {
        for persona in *personas {
            let rel = format!(".claude/agents/reviewer-{persona}.md");
            let fm = parse_claude_pins(root, &rel)?;
            assert_eq!(
                fm.model.as_deref(),
                Some(*expected_model),
                "rendered {rel} must carry `model: {expected_model}` (REQ-003 asymmetric pin); got {:?}",
                fm.model,
            );
            assert_eq!(
                fm.effort.as_deref(),
                Some(*expected_effort),
                "rendered {rel} must carry `effort: {expected_effort}` (REQ-003 asymmetric pin); got {:?}",
                fm.effort,
            );
        }
    }
    Ok(())
}

#[test]
fn t007_init_renders_codex_pin_assignments_matching_dogfood_pack() -> TestResult {
    // Codex half: mirror of the Claude Code test
    // against the Codex host pack. Asserts the three pinned phase-worker
    // TOMLs, the absence invariants for `speccy-review.toml` and
    // `speccy-init.toml`, the asymmetric reviewer assignment, the
    // pinned-phase thin-stub bodies, and the full-body `speccy-init`
    // SKILL.md.
    let fx = project_with_name("t007-codex-pins")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("codex")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    for phase in CLAUDE_PINNED_PHASES {
        let rel = format!(".codex/agents/speccy-{phase}.toml");
        let table = read_codex_toml(&fx.root, &rel)?;
        assert_codex_pin(&table, &rel, "medium");
    }

    let review_toml = fx.root.join(".codex/agents/speccy-review.toml");
    assert!(
        !review_toml.exists(),
        "REQ-002 / DEC-002: speccy init must not render `.codex/agents/speccy-review.toml` (orchestrator stays unpinned on Codex); found at `{review_toml}`",
    );
    let init_toml = fx.root.join(".codex/agents/speccy-init.toml");
    assert!(
        !init_toml.exists(),
        "DEC-009 / REQ-010: speccy init must not render `.codex/agents/speccy-init.toml` (no pinned init subagent); found at `{init_toml}`",
    );

    assert_codex_reviewer_pins(&fx.root)?;

    for phase in CLAUDE_STUB_DELEGATE_PHASES {
        let rel = format!(".agents/skills/speccy-{phase}/SKILL.md");
        let agent_path = format!(".codex/agents/speccy-{phase}.toml");
        assert_thin_stub_body(&fx.root, &rel, &agent_path, phase)?;
    }
    assert_init_full_body(&fx.root, ".agents/skills/speccy-init/SKILL.md")?;
    Ok(())
}

fn assert_codex_reviewer_pins(root: &Utf8Path) -> TestResult {
    let cases: &[(&[&str], &str)] = &[
        (&CODEX_HIGH_REVIEWERS, "high"),
        (&CODEX_LOW_REVIEWERS, "low"),
    ];
    for (personas, expected_effort) in cases {
        for persona in *personas {
            let rel = format!(".codex/agents/reviewer-{persona}.toml");
            let table = read_codex_toml(root, &rel)?;
            assert_codex_pin(&table, &rel, expected_effort);
        }
    }
    Ok(())
}
