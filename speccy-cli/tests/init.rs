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
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SHIPPED_CLAUDE_SPECCY_INIT: &str =
    include_str!("../../skills/claude-code/speccy-init/SKILL.md");
const SHIPPED_CODEX_SPECCY_INIT: &str = include_str!("../../skills/codex/speccy-init/SKILL.md");
const SHIPPED_PERSONA_SECURITY: &str =
    include_str!("../../skills/shared/personas/reviewer-security.md");

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
    assert_eq!(
        shipped, SHIPPED_CLAUDE_SPECCY_INIT,
        "--force should restore .claude/skills/speccy-init/SKILL.md to embedded content",
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
        let src_rel = format!("../skills/claude-code/{skill}/SKILL.md");
        // Re-read the bundle source through the test binary's own
        // filesystem rather than `include_str!`-ing all 7 because rust
        // requires the macro argument to be a literal. We rely on the
        // project layout that `Command::cargo_bin` runs against.
        let src_abs = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(&src_rel)
            .canonicalize()?;
        let src = fs_err::read_to_string(&src_abs)?;
        assert_eq!(
            dest, src,
            "copied {rel} must be byte-identical to the embedded source at {src_rel}",
        );
    }

    let persona = read_file(&fx.root, ".speccy/skills/personas/reviewer-security.md")?;
    assert_eq!(
        persona, SHIPPED_PERSONA_SECURITY,
        "shared persona file must be copied byte-identical into .speccy/skills/personas/",
    );
    Ok(())
}

#[test]
fn copy_codex_pack_skill_md() -> TestResult {
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
        let src_rel = format!("../skills/codex/{skill}/SKILL.md");
        let src_abs = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(&src_rel)
            .canonicalize()?;
        let src = fs_err::read_to_string(&src_abs)?;
        assert_eq!(
            dest, src,
            "copied {rel} must be byte-identical to the embedded source at {src_rel}",
        );
    }

    // Sanity check: the include_str!-pulled init.md is byte-identical to
    // the on-disk file. If this trips, the bundle and the include_str!
    // path are out of sync, which a stale path-edit would cause.
    let init_dest = read_file(&fx.root, ".agents/skills/speccy-init/SKILL.md")?;
    assert_eq!(
        init_dest, SHIPPED_CODEX_SPECCY_INIT,
        "speccy-init/SKILL.md (codex) must match the include_str! constant",
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
