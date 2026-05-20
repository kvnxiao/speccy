#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "assert!/assert_eq! inside TestResult-returning tests is idiomatic"
)]
//! Tests for SPEC-0033 T-008: three-way init classification and
//! interactive skill body ejection.
//!
//! Covers CHK-011, CHK-019, CHK-020, CHK-021, and CHK-022.

use assert_cmd::Command;
use camino::Utf8PathBuf;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Fixture {
    _dir: TempDir,
    root: Utf8PathBuf,
}

fn make_fixture(name: &str) -> TestResult<Fixture> {
    let parent = tempfile::tempdir()?;
    let root_std = parent.path().join(name);
    fs_err::create_dir_all(&root_std)?;
    let root = Utf8PathBuf::from_path_buf(root_std)
        .map_err(|p| format!("project root must be UTF-8: {}", p.display()))?;
    Ok(Fixture { _dir: parent, root })
}

fn write_file(root: &Utf8PathBuf, rel: &str, body: &str) -> TestResult {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent.as_std_path())?;
    }
    fs_err::write(path.as_std_path(), body)?;
    Ok(())
}

fn read_file(root: &Utf8PathBuf, rel: &str) -> TestResult<String> {
    Ok(fs_err::read_to_string(root.join(rel).as_std_path())?)
}

fn run_init(root: &Utf8PathBuf, extra_args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("speccy").expect("speccy binary must be available");
    cmd.arg("init")
        .arg("--host")
        .arg("claude-code")
        .current_dir(root.as_std_path());
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.assert()
}

// -----------------------------------------------------------------------
// CHK-011: fresh init creates a speccy-plan SKILL.md with substantive
// body and no MiniJinja markup.
// -----------------------------------------------------------------------

#[test]
fn chk011_fresh_init_creates_speccy_plan_skill_md_with_substantive_body() -> TestResult {
    let fx = make_fixture("chk011-plan-body")?;
    run_init(&fx.root, &[]).success();

    let rel = ".claude/skills/speccy-plan/SKILL.md";
    let body = read_file(&fx.root, rel)?;
    assert!(
        fx.root.join(rel).exists(),
        "CHK-011: {rel} must be created by `speccy init --host claude-code`",
    );
    let (_, post_fm) = split_frontmatter(&body)
        .ok_or_else(|| format!("{rel} must have a `---` frontmatter fence"))?;
    assert!(
        post_fm.lines().filter(|l| !l.trim().is_empty()).count() > 5,
        "CHK-011: {rel} body must be substantive (more than 5 non-blank lines); got:\n{post_fm}",
    );
    assert!(
        !body.contains("{{"),
        "CHK-011: {rel} must contain no unsubstituted `{{` MiniJinja token; got:\n{body}",
    );
    assert!(
        !body.contains("{%"),
        "CHK-011: {rel} must contain no unsubstituted `{{%` MiniJinja token; got:\n{body}",
    );
    Ok(())
}

// -----------------------------------------------------------------------
// CHK-019: all files byte-identical → exit 0, every file logged `unchanged`.
// -----------------------------------------------------------------------

#[test]
fn chk019_byte_identical_files_log_unchanged_and_exit_zero() -> TestResult {
    let fx = make_fixture("chk019-unchanged")?;
    // First run creates everything.
    run_init(&fx.root, &[]).success();

    // Second run without --force: everything is byte-identical → should exit 0
    // and log every file as `unchanged`.
    let output = run_init(&fx.root, &[])
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;
    // Check plan lines: none should say "created" or "(!) overwritten" as action
    // label. Plan lines are indented and start with the action label, e.g. "
    // unchanged        .claude/..."
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        assert!(
            !trimmed.starts_with("created ") && !trimmed.starts_with("(!) overwritten"),
            "CHK-019: plan must not include `created` or `(!) overwritten` lines when all files are byte-identical; got line: `{line}`",
        );
    }
    assert!(
        stdout.contains("unchanged"),
        "CHK-019: re-init with byte-identical files must log `unchanged` entries; got:\n{stdout}",
    );
    Ok(())
}

#[test]
fn chk019_byte_identical_no_mtime_change() -> TestResult {
    let fx = make_fixture("chk019-mtime")?;
    run_init(&fx.root, &[]).success();

    let rel = ".claude/skills/speccy-plan/SKILL.md";
    let path = fx.root.join(rel);
    let mtime_before = fs_err::metadata(path.as_std_path())
        .map_err(|e| format!("stat {rel} before: {e}"))?
        .modified()
        .map_err(|e| format!("mtime {rel}: {e}"))?;

    // Second run: byte-identical → no write, mtime must not change.
    run_init(&fx.root, &[]).success();

    let mtime_after = fs_err::metadata(path.as_std_path())
        .map_err(|e| format!("stat {rel} after: {e}"))?
        .modified()
        .map_err(|e| format!("mtime {rel}: {e}"))?;

    assert_eq!(
        mtime_before, mtime_after,
        "CHK-019: mtime of {rel} must not change when re-init finds byte-identical content",
    );
    Ok(())
}

// -----------------------------------------------------------------------
// CHK-020: one differing file → exit non-zero, stderr names the file
// and `--force`, offending file unchanged, no other file written.
// -----------------------------------------------------------------------

#[test]
fn chk020_differing_file_causes_nonzero_exit() -> TestResult {
    let fx = make_fixture("chk020-nonzero")?;
    run_init(&fx.root, &[]).success();

    // Append a user line to one shipped file.
    let rel = ".claude/skills/speccy-plan/SKILL.md";
    let original = read_file(&fx.root, rel)?;
    let modified = format!("{original}\n# user-appended custom prose\n");
    write_file(&fx.root, rel, &modified)?;

    // Re-init without --force must fail.
    run_init(&fx.root, &[]).failure();
    Ok(())
}

#[test]
fn chk020_differing_file_stderr_names_path_and_force_flag() -> TestResult {
    let fx = make_fixture("chk020-stderr")?;
    run_init(&fx.root, &[]).success();

    let rel = ".claude/skills/speccy-plan/SKILL.md";
    let original = read_file(&fx.root, rel)?;
    let modified = format!("{original}\n# user-appended custom prose\n");
    write_file(&fx.root, rel, &modified)?;

    let output = run_init(&fx.root, &[])
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output)?;

    assert!(
        stderr.contains("speccy-plan") || stderr.contains("SKILL.md"),
        "CHK-020: stderr must name the differing file path; got:\n{stderr}",
    );
    assert!(
        stderr.contains("--force"),
        "CHK-020: stderr must mention `--force`; got:\n{stderr}",
    );
    Ok(())
}

#[test]
fn chk020_differing_file_is_unchanged_after_refuse() -> TestResult {
    let fx = make_fixture("chk020-offending-unchanged")?;
    run_init(&fx.root, &[]).success();

    let rel = ".claude/skills/speccy-plan/SKILL.md";
    let original = read_file(&fx.root, rel)?;
    let user_line = "\n# user-appended custom prose\n";
    let modified = format!("{original}{user_line}");
    write_file(&fx.root, rel, &modified)?;

    run_init(&fx.root, &[]).failure();

    let after = read_file(&fx.root, rel)?;
    assert_eq!(
        after, modified,
        "CHK-020: the differing file must be byte-identical to its pre-invocation state after the refused init",
    );
    Ok(())
}

#[test]
fn chk020_atomic_refuse_no_other_file_written() -> TestResult {
    // Scenario: a fresh tempdir has NO files from a previous init.
    // One file is pre-created with different content than what init
    // would write.  Without --force, init must refuse entirely — it
    // must not write the other files either.
    let fx = make_fixture("chk020-atomic")?;

    // Pre-populate one shipped file with stale content.
    write_file(
        &fx.root,
        ".claude/skills/speccy-plan/SKILL.md",
        "---\nname: speccy-plan\ndescription: stale\n---\nstale body\n",
    )?;

    // Run init without --force in a workspace where SKILL.md exists
    // but differs from what the renderer would produce.
    run_init(&fx.root, &[]).failure();

    // The .speccy/speccy.toml file must NOT have been created — atomic
    // refuse means no other planned target is written.
    assert!(
        !fx.root.join(".speccy/speccy.toml").exists(),
        "CHK-020: atomic batch refuse must leave .speccy/speccy.toml uncreated when one planned file conflicts",
    );
    // The speccy-init SKILL.md must also not be created.
    assert!(
        !fx.root.join(".claude/skills/speccy-init/SKILL.md").exists(),
        "CHK-020: atomic batch refuse must leave other planned targets uncreated",
    );
    Ok(())
}

// -----------------------------------------------------------------------
// CHK-021: --force with differing file → exit 0, file overwritten as
// `(!) overwritten`, byte-identical files logged `unchanged`.
// -----------------------------------------------------------------------

#[test]
fn chk021_force_overwrites_differing_file_logs_overwritten() -> TestResult {
    let fx = make_fixture("chk021-overwrite")?;
    run_init(&fx.root, &[]).success();

    let rel = ".claude/skills/speccy-plan/SKILL.md";
    let original = read_file(&fx.root, rel)?;
    let modified = format!("{original}\n# user-appended custom prose\n");
    write_file(&fx.root, rel, &modified)?;

    let output = run_init(&fx.root, &["--force"])
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;

    assert!(
        stdout.contains("overwritten"),
        "CHK-021: --force with differing file must log `overwritten` for the differing file; got:\n{stdout}",
    );
    Ok(())
}

#[test]
fn chk021_force_identical_files_logged_unchanged_not_overwritten() -> TestResult {
    let fx = make_fixture("chk021-unchanged-flag")?;
    run_init(&fx.root, &[]).success();

    // Append user content to exactly one file.
    let differing_rel = ".claude/skills/speccy-plan/SKILL.md";
    let original = read_file(&fx.root, differing_rel)?;
    let modified = format!("{original}\n# appended\n");
    write_file(&fx.root, differing_rel, &modified)?;

    let output = run_init(&fx.root, &["--force"])
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output)?;

    // Every line that is NOT the differing file must say `unchanged`, not `(!)
    // overwritten`.
    for line in stdout.lines() {
        if line.contains("speccy-plan") && line.contains("SKILL.md") {
            assert!(
                line.contains("overwritten"),
                "CHK-021: the differing file line must say `overwritten`; got: `{line}`",
            );
        } else if line.trim_start().starts_with('.') || line.trim_start().starts_with(".speccy") {
            assert!(
                line.contains("unchanged"),
                "CHK-021: byte-identical files must be logged `unchanged`, not `overwritten`; got line: `{line}`",
            );
        }
    }
    Ok(())
}

#[test]
fn chk021_force_differing_file_content_matches_planned() -> TestResult {
    let fx = make_fixture("chk021-content")?;
    run_init(&fx.root, &[]).success();

    let rel = ".claude/skills/speccy-plan/SKILL.md";
    let original = read_file(&fx.root, rel)?;
    let modified = format!("{original}\n# user-appended custom prose\n");
    write_file(&fx.root, rel, &modified)?;

    run_init(&fx.root, &["--force"]).success();

    let after = read_file(&fx.root, rel)?;
    assert_eq!(
        after, original,
        "CHK-021: --force must restore the differing file to the planned content",
    );
    Ok(())
}

// -----------------------------------------------------------------------
// CHK-022: no .claude/agents/speccy-init.md, no
// .claude/agents/speccy-review.md, no .codex/agents/speccy-init.toml, no
// .codex/agents/speccy-review.toml.
// -----------------------------------------------------------------------

#[test]
fn chk022_no_interactive_skill_agent_files_created() -> TestResult {
    let fx = make_fixture("chk022-no-init-review-agent")?;

    // Claude Code.
    run_init(&fx.root, &[]).success();

    let forbidden_claude = [
        ".claude/agents/speccy-init.md",
        ".claude/agents/speccy-review.md",
    ];
    for path in forbidden_claude {
        assert!(
            !fx.root.join(path).exists(),
            "CHK-022 / DEC-008: `speccy init --host claude-code` must NOT create `{path}` (interactive skill; no agent counterpart)",
        );
    }

    // Codex.
    let mut cmd = Command::cargo_bin("speccy")
        .map_err(|e| format!("speccy binary must be available: {e}"))?;
    cmd.arg("init")
        .arg("--host")
        .arg("codex")
        .current_dir(fx.root.as_std_path());
    cmd.assert().success();

    let forbidden_codex = [
        ".codex/agents/speccy-init.toml",
        ".codex/agents/speccy-review.toml",
    ];
    for path in forbidden_codex {
        assert!(
            !fx.root.join(path).exists(),
            "CHK-022 / DEC-008: `speccy init --host codex` must NOT create `{path}` (interactive skill; no agent counterpart)",
        );
    }
    Ok(())
}

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

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
