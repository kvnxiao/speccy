#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![allow(
    clippy::panic_in_result_fn,
    reason = "assert!/assert_eq! inside TestResult-returning tests is idiomatic"
)]
//! End-to-end tests for `speccy init`.
//! Exercises SPEC-0002 REQ-001..REQ-005 through the binary entry point.

use assert_cmd::Command;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use predicates::str::contains;
use serde::Deserialize;
use speccy_cli::host::HostChoice;
use speccy_cli::render::render_host_pack;
use speccy_core::personas::PERSONAS;
use speccy_core::prompt::PROMPTS;
use std::path::Path;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SHIPPED_PERSONA_SECURITY: &str =
    include_str!("../../resources/modules/personas/reviewer-security.md");

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
/// `skill_packs::SKILL_NAMES`. Per SPEC-0015 each ships at
/// `<host>/<name>/SKILL.md` in the bundle and at
/// `<dest>/<name>/SKILL.md` in the user's project.
const SKILL_NAMES: [&str; 7] = [
    "speccy-init",
    "speccy-plan",
    "speccy-tasks",
    "speccy-work",
    "speccy-review",
    "speccy-ship",
    "speccy-amend",
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
fn scaffold_speccy_toml() -> TestResult {
    let fx = project_with_name("acme")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init").current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let body = read_file(&fx.root, ".speccy/speccy.toml")?;
    assert!(
        body.contains("schema_version = 1"),
        "speccy.toml should declare schema_version = 1; got: {body}",
    );
    assert!(
        body.contains("name = \"acme\""),
        "speccy.toml should set project name to parent dir name `acme`; got: {body}",
    );
    Ok(())
}

#[test]
fn does_not_scaffold_vision_md() -> TestResult {
    let fx = project_with_name("no-vision-project")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init").current_dir(fx.root.as_std_path());
    cmd.assert().success();

    assert!(
        !fx.root.join(".speccy/VISION.md").exists(),
        "speccy init must not scaffold .speccy/VISION.md (the noun has been retired; the product north star lives in AGENTS.md instead)",
    );
    Ok(())
}

#[test]
fn refuse_without_force() -> TestResult {
    let fx = project_with_name("refuse")?;
    mkdir(&fx.root, ".speccy")?;
    write_file(&fx.root, ".speccy/speccy.toml", "schema_version = 1\n")?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init").current_dir(fx.root.as_std_path());
    cmd.assert().failure().code(1).stderr(contains(".speccy/"));
    Ok(())
}

#[test]
fn force_overwrites_shipped_files() -> TestResult {
    let fx = project_with_name("force-shipped")?;
    mkdir(&fx.root, ".speccy")?;
    write_file(&fx.root, ".speccy/speccy.toml", "OLD-TOML")?;
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

    let toml_body = read_file(&fx.root, ".speccy/speccy.toml")?;
    assert_ne!(
        toml_body, "OLD-TOML",
        "--force should refresh .speccy/speccy.toml",
    );
    assert!(
        toml_body.contains("schema_version = 1"),
        "refreshed speccy.toml should match template",
    );

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
    // SPEC-0016 T-008 retired the legacy per-host byte-equality oracle.
    // Each rendered SKILL.md now goes through MiniJinja, so the
    // assertions below match the renderer's contract: the file exists,
    // its YAML frontmatter parses with the right `name`, and the body
    // uses slash-prefixed command references (mirrors the unit-test
    // shape in `src/render.
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
    // the main agent at the `/speccy-tasks` skill as the suggested next
    // step. The renderer must substitute `{{ cmd_prefix }}` to `"/"`.
    let plan_body = read_file(&fx.root, ".claude/skills/speccy-plan/SKILL.md")?;
    assert!(
        plan_body.contains("/speccy-tasks"),
        "rendered .claude/skills/speccy-plan/SKILL.md must contain `/speccy-tasks`",
    );

    let persona = read_file(&fx.root, ".speccy/skills/personas/reviewer-security.md")?;
    assert_eq!(
        persona, SHIPPED_PERSONA_SECURITY,
        "shared persona file must be copied byte-identical into .speccy/skills/personas/",
    );
    Ok(())
}

#[test]
fn copy_codex_pack_skill_md() -> TestResult {
    // SPEC-0016 T-008 retired the legacy per-host byte-equality oracle.
    // Each rendered SKILL.md now flows through MiniJinja, so the
    // assertions below mirror the unit-test shape in `src/render.
    // rs::render_host_pack_codex_speccy_plan_uses_bare_command`: file exists,
    // YAML frontmatter parses, and command references are bare (no slash
    // prefix) per Codex's convention.
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
    // `speccy-tasks` without the leading `/` (the renderer substitutes
    // `{{ cmd_prefix }}` to the empty string).
    let plan_body = read_file(&fx.root, ".agents/skills/speccy-plan/SKILL.md")?;
    assert!(
        plan_body.contains("speccy-tasks"),
        "rendered .agents/skills/speccy-plan/SKILL.md must contain `speccy-tasks`",
    );
    assert!(
        !plan_body.contains("/speccy-tasks"),
        "rendered .agents/skills/speccy-plan/SKILL.md must NOT contain `/speccy-tasks` (Codex uses bare command names)",
    );
    Ok(())
}

#[test]
fn t009_claude_code_reviewer_subagents_land_at_dot_claude_agents() -> TestResult {
    // SPEC-0016 T-009 obligation: `speccy init --host claude-code`
    // materialises six `.claude/agents/reviewer-<persona>.md` files,
    // each opening with `---` (YAML fence), each parseable, and the
    // security file in particular carrying the documented focus
    // bullet from the persona body.
    let fx = project_with_name("t009-claude-agents")?;
    mkdir(&fx.root, ".claude")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init").current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let personas: [&str; 6] = [
        "business",
        "tests",
        "security",
        "style",
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

    // The security reviewer's body must carry the documented focus
    // bullet drawn verbatim from the persona module file (this is the
    // observable assertion REQ-003 specifies for the security
    // persona).
    let security = read_file(&fx.root, ".claude/agents/reviewer-security.md")?;
    assert!(
        security.contains("Authentication and authorization boundaries"),
        "rendered .claude/agents/reviewer-security.md must contain the focus bullet drawn from the persona body; got:\n{security}",
    );
    Ok(())
}

#[test]
fn t010_codex_reviewer_subagents_land_at_dot_codex_agents() -> TestResult {
    // SPEC-0016 T-010 obligation: `speccy init --host codex`
    // materialises six `.codex/agents/reviewer-<persona>.toml` files,
    // each parseable as flat TOML with the three required top-level
    // keys (`name`, `description`, `developer_instructions`), and the
    // security file in particular carrying the documented focus
    // bullet from the persona body inside its `developer_instructions`
    // string.
    let fx = project_with_name("t010-codex-agents")?;
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("init")
        .arg("--host")
        .arg("codex")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let personas: [&str; 6] = [
        "business",
        "tests",
        "security",
        "style",
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

    // The security reviewer's `developer_instructions` body must
    // carry the documented focus bullet drawn verbatim from the
    // persona module file (this is the observable assertion REQ-003
    // specifies for the security persona on the Codex host).
    let security_body = read_file(&fx.root, ".codex/agents/reviewer-security.toml")?;
    let security_parsed: toml::Value = toml::from_str(&security_body)
        .map_err(|err| format!("rendered reviewer-security.toml must parse as TOML: {err}"))?;
    let security_dev = security_parsed
        .as_table()
        .and_then(|t| t.get("developer_instructions"))
        .and_then(toml::Value::as_str)
        .ok_or("rendered reviewer-security.toml must have a string `developer_instructions` key")?;
    assert!(
        security_dev.contains("Authentication and authorization boundaries"),
        "rendered .codex/agents/reviewer-security.toml `developer_instructions` must contain the focus bullet drawn from the persona body; got:\n{security_dev}",
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

    // 1 on existing workspace without --force.
    {
        let fx = project_with_name("exit-one-workspace")?;
        mkdir(&fx.root, ".speccy")?;
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
// SPEC-0016 T-013 / CHK-008 / CHK-009 / CHK-010: dogfood byte-identity,
// renderer idempotency, and unsubstituted-token guard.
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
    // CHK-008: rendered outputs from `render_host_pack` for both hosts
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

    // Host-neutral persona / prompt bundles are copied verbatim by
    // `init` into `.speccy/skills/personas/` and `.speccy/skills/prompts/`;
    // walk each bundle and compare bytes against the committed
    // workspace files.
    for (bundle_name, dest_segment, bundle) in [
        ("personas", "personas", &PERSONAS),
        ("prompts", "prompts", &PROMPTS),
    ] {
        for file in bundle.files() {
            let name = file
                .path()
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("{bundle_name} bundle entry must have a UTF-8 name"))?;
            if !has_md_ext(name) {
                continue;
            }
            let committed_path = root
                .join(".speccy")
                .join("skills")
                .join(dest_segment)
                .join(name);
            let committed = fs_err::read_to_string(&committed_path).map_err(|err| {
                format!("committed `.speccy/skills/{dest_segment}/{name}` must be readable: {err}")
            })?;
            let bundle_bytes = file.contents();
            let bundle_str = std::str::from_utf8(bundle_bytes).map_err(|err| {
                format!("{bundle_name} bundle entry `{name}` must be UTF-8: {err}")
            })?;
            assert_eq!(
                committed, bundle_str,
                "committed `.speccy/skills/{dest_segment}/{name}` differs from the embedded bundle; \
                 run `speccy init --force --host claude-code` (or codex) locally and commit",
            );
        }
    }

    Ok(())
}

#[test]
fn render_is_idempotent() -> TestResult {
    // CHK-009: rendering the full host pack twice must produce the
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
    // CHK-010: no rendered output file may contain the literal
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

fn has_md_ext(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
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
// SPEC-0018 T-006 / REQ-005: shipped host packs teach the new
// scenario-only check schema. The renderer outputs must contain no
// legacy check-authoring examples (`kind =`, `command =`, `prompt =`,
// `proves =`) so downstream agents cannot copy-paste the pre-SPEC-0018
// shape into a fresh `spec.toml`. Historical SPEC.md / TASKS.md /
// REPORT.md records under `.speccy/specs/` are excluded from this
// guard — those carry the audit trail of the migration itself.
// --------------------------------------------------------------------

/// Field-assignment needles for legacy `[[checks]]` rows that
/// SPEC-0018 retired. Each one is tight enough that it only matches
/// example code (the trailing `"` on `command = "` / `prompt = "`
/// avoids prose like "prompt for the user", and the historical-note
/// blockquotes in `.speccy/ARCHITECTURE.md` use backticks rather than
/// the literal quoted assignment).
const SPEC_0018_LEGACY_NEEDLES: [&str; 6] = [
    "kind = \"test\"",
    "kind = \"command\"",
    "kind = \"manual\"",
    "proves =",
    "command = \"",
    "prompt = \"",
];

#[test]
fn rendered_outputs_have_no_legacy_check_authoring_examples() -> TestResult {
    // CHK-005 sweep: walk every file rendered by `render_host_pack`
    // (both hosts) and assert it does not contain a legacy
    // check-authoring assignment. The needles are the field names
    // SPEC-0018 retired from `[[checks]]` rows. They never appear in
    // post-migration `spec.toml` files, so any hit in the shipped
    // prompts/personas/skills means stale guidance leaked through.
    for host in [HostChoice::ClaudeCode, HostChoice::Codex] {
        let rendered = render_host_pack(host)
            .map_err(|err| format!("render_host_pack({host:?}) should succeed: {err}"))?;
        for file in &rendered {
            for needle in SPEC_0018_LEGACY_NEEDLES {
                assert!(
                    !file.contents.contains(needle),
                    "rendered `{}` for host {host:?} contains legacy check-authoring snippet `{needle}` (SPEC-0018 removed these fields)",
                    file.rel_path,
                );
            }
        }
    }
    Ok(())
}

/// `.speccy/ARCHITECTURE.md` is not rendered through host packs, so
/// `rendered_outputs_have_no_legacy_check_authoring_examples` does not
/// cover it. Lint it directly: the design doc must teach the
/// post-SPEC-0018 `{id, scenario}` shape, and the only legitimate
/// mentions of the retired field names live inside `> Historical
/// note:` blockquotes that name them in prose with backticks, not as
/// quoted-literal assignments. The tight needles in
/// `SPEC_0018_LEGACY_NEEDLES` only fire on example code.
#[test]
fn architecture_doc_has_no_legacy_check_authoring_examples() {
    const ARCHITECTURE: &str = include_str!("../../.speccy/ARCHITECTURE.md");
    for needle in SPEC_0018_LEGACY_NEEDLES {
        assert!(
            !ARCHITECTURE.contains(needle),
            ".speccy/ARCHITECTURE.md contains legacy check-authoring snippet `{needle}` (SPEC-0018 removed these fields; historical notes should use backticked prose, not literal assignments)",
        );
    }
}

/// Source-of-truth sweep over `resources/modules/personas/**` and
/// `resources/modules/prompts/**`. The host-pack-rendered guard above
/// catches what reaches `.claude/` / `.codex/` / `.speccy/skills/`,
/// but only via the renderer; this test pins the upstream sources
/// directly so a regression that adds a legacy example in a persona
/// or prompt body fails before any `speccy init` runs.
#[test]
fn persona_and_prompt_sources_have_no_legacy_check_authoring_examples() {
    for entry in PERSONAS.files() {
        let path = entry.path().display();
        let body = entry.contents_utf8().unwrap_or("");
        for needle in SPEC_0018_LEGACY_NEEDLES {
            assert!(
                !body.contains(needle),
                "resources/modules/personas/{path} contains legacy check-authoring snippet `{needle}` (SPEC-0018 removed these fields)",
            );
        }
    }
    for entry in PROMPTS.files() {
        let path = entry.path().display();
        let body = entry.contents_utf8().unwrap_or("");
        for needle in SPEC_0018_LEGACY_NEEDLES {
            assert!(
                !body.contains(needle),
                "resources/modules/prompts/{path} contains legacy check-authoring snippet `{needle}` (SPEC-0018 removed these fields)",
            );
        }
    }
}

/// Positive content pins for SPEC-0018 / T-005. These assert that
/// the load-bearing post-migration sentences survive in their source
/// files verbatim. A regression that silently softens or removes any
/// of them flips the suite red.
///
/// The reviewer-tests anti-instruction is the most load-bearing: it
/// is the new reviewer contract that replaced "run `speccy check` and
/// trust the exit code". The ARCHITECTURE.md pins lock the "Feedback,
/// Not Enforcement" stance, the render-only `check` row, and the
/// shape-only `verify` row.
#[test]
fn reviewer_tests_persona_pins_no_check_exit_code_evidence() {
    const REVIEWER_TESTS: &str = include_str!("../../resources/modules/personas/reviewer-tests.md");
    // Verbatim sentence from `reviewer-tests.md:33-37`. The leading
    // dash + space are the literal markdown bullet glyph; if the
    // paragraph is rewrapped or rephrased this assertion fails and
    // the reviewer must re-pin it explicitly.
    let needle = "Do not treat `speccy check` exit codes (or any command exit code)\n  as evidence that a scenario is satisfied.";
    assert!(
        REVIEWER_TESTS.contains(needle),
        "reviewer-tests persona must keep the post-SPEC-0018 anti-instruction \
         that `speccy check` exit codes are not evidence; the load-bearing \
         sentence at lines 33-37 is missing or has drifted",
    );
}

#[test]
fn architecture_doc_pins_feedback_not_enforcement_contract() {
    const ARCHITECTURE: &str = include_str!("../../.speccy/ARCHITECTURE.md");
    // Heading that names the stance.
    assert!(
        ARCHITECTURE.contains("# Stance: Feedback, Not Enforcement"),
        ".speccy/ARCHITECTURE.md is missing the `Stance: Feedback, Not Enforcement` heading",
    );
    // CI-owns-tests clause.
    assert!(
        ARCHITECTURE.contains("**Speccy does not run project tests.**"),
        ".speccy/ARCHITECTURE.md is missing the `Speccy does not run project tests` clause",
    );
    // Reviewer-personas-own-semantic-judgment clause.
    assert!(
        ARCHITECTURE.contains("**Reviewer personas own semantic judgment.**"),
        ".speccy/ARCHITECTURE.md is missing the `Reviewer personas own semantic judgment` clause",
    );
}

#[test]
fn architecture_doc_pins_check_command_is_render_only() {
    const ARCHITECTURE: &str = include_str!("../../.speccy/ARCHITECTURE.md");
    // CLI Surface row for `speccy check`: must describe it as
    // scenario rendering with no execution.
    assert!(
        ARCHITECTURE
            .contains("speccy check [SELECTOR]           Render check scenarios (no execution)"),
        ".speccy/ARCHITECTURE.md `speccy check` row must describe it as render-only (no execution)",
    );
}

#[test]
fn architecture_doc_pins_verify_command_is_shape_only() {
    const ARCHITECTURE: &str = include_str!("../../.speccy/ARCHITECTURE.md");
    // CLI Surface row for `speccy verify`: must scope it to proof
    // shape only and explicitly disclaim running project tests.
    assert!(
        ARCHITECTURE
            .contains("speccy verify                     CI gate: proof-shape validation only"),
        ".speccy/ARCHITECTURE.md `speccy verify` row must describe it as proof-shape validation only",
    );
    assert!(
        ARCHITECTURE.contains("Does NOT run project tests; that's CI's job."),
        ".speccy/ARCHITECTURE.md `speccy verify` row must disclaim running project tests",
    );
}
