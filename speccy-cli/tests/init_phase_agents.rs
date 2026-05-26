#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "assert!/assert_eq! inside TestResult-returning tests is idiomatic"
)]
//! Tests for SPEC-0033 T-009: pinned phase-worker agent files and thin
//! SKILL.md stubs ejected at `speccy init`.
//!
//! Covers CHK-017 and the T-009 task scenarios.

use assert_cmd::Command;
use camino::Utf8PathBuf;
use tempfile::TempDir;

mod common;

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

fn read_file(root: &Utf8PathBuf, rel: &str) -> TestResult<String> {
    Ok(fs_err::read_to_string(root.join(rel).as_std_path())?)
}

fn run_init_claude(root: &Utf8PathBuf, extra_args: &[&str]) -> assert_cmd::assert::Assert {
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

fn run_init_codex(root: &Utf8PathBuf, extra_args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("speccy").expect("speccy binary must be available");
    cmd.arg("init")
        .arg("--host")
        .arg("codex")
        .current_dir(root.as_std_path());
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.assert()
}

/// Count non-blank lines in a string.
fn non_blank_line_count(s: &str) -> usize {
    s.lines().filter(|l| !l.trim().is_empty()).count()
}

// -----------------------------------------------------------------------
// Scenario 1: SKILL.md stub for speccy-work (claude-code) is a thin stub
// with ≤10 non-blank lines, no disallowed frontmatter keys, names the
// agent file and the /agent invocation path.
// -----------------------------------------------------------------------

#[test]
fn phase_worker_skill_stub_is_thin() -> TestResult {
    let fx = make_fixture("t009-skill-stub-thin")?;
    run_init_claude(&fx.root, &[]).success();

    for phase in &["speccy-work", "speccy-decompose", "speccy-ship"] {
        let rel = format!(".claude/skills/{phase}/SKILL.md");
        let body = read_file(&fx.root, &rel)?;
        let count = common::non_blank_line_count_outside_reconcile_partial(&body);
        assert!(
            count <= 10,
            "T-009: {rel} must have ≤10 non-blank lines outside reconcile-policy partial markers; got {count}:\n{body}",
        );
    }
    Ok(())
}

#[test]
fn phase_worker_skill_stub_has_no_disallowed_frontmatter_keys() -> TestResult {
    let fx = make_fixture("t009-skill-stub-frontmatter")?;
    run_init_claude(&fx.root, &[]).success();

    let disallowed = ["context:", "agent:", "model:", "effort:"];
    for phase in &["speccy-work", "speccy-decompose", "speccy-ship"] {
        let rel = format!(".claude/skills/{phase}/SKILL.md");
        let body = read_file(&fx.root, &rel)?;
        let fm = extract_frontmatter(&body)
            .ok_or_else(|| format!("{rel} must have YAML frontmatter"))?;
        for key in disallowed {
            assert!(
                !fm.lines().any(|l| l.trim_start().starts_with(key)),
                "T-009: {rel} frontmatter must not contain `{key}`; got frontmatter:\n{fm}",
            );
        }
    }
    Ok(())
}

#[test]
fn phase_worker_skill_stub_names_agent_file_and_invocation_path() -> TestResult {
    let fx = make_fixture("t009-skill-stub-names-agent")?;
    run_init_claude(&fx.root, &[]).success();

    for phase in &["speccy-work", "speccy-decompose", "speccy-ship"] {
        let rel = format!(".claude/skills/{phase}/SKILL.md");
        let body = read_file(&fx.root, &rel)?;
        let agent_file = format!(".claude/agents/{phase}.md");
        assert!(
            body.contains(&agent_file),
            "T-009: {rel} must name the agent file `{agent_file}`; got:\n{body}",
        );
        let invocation = format!("/agent {phase}");
        assert!(
            body.contains(&invocation),
            "T-009: {rel} must reference the `/agent {phase}` invocation path; got:\n{body}",
        );
    }
    Ok(())
}

// -----------------------------------------------------------------------
// Scenario 2: Agent file for speccy-work (claude-code) has model/effort
// frontmatter, full phase body, no MiniJinja markup.
// -----------------------------------------------------------------------

#[test]
fn phase_worker_agent_has_model_and_effort_frontmatter() -> TestResult {
    let fx = make_fixture("t009-agent-frontmatter")?;
    run_init_claude(&fx.root, &[]).success();

    // Each phase has its own model/effort pin.
    let phase_pins: &[(&str, &str, &str)] = &[
        ("speccy-work", "opus[1m]", "low"),
        ("speccy-decompose", "sonnet[1m]", "medium"),
        ("speccy-ship", "sonnet[1m]", "medium"),
    ];
    for (phase, expected_model, expected_effort) in phase_pins {
        let rel = format!(".claude/agents/{phase}.md");
        let body = read_file(&fx.root, &rel)?;
        let fm = extract_frontmatter(&body)
            .ok_or_else(|| format!("{rel} must have YAML frontmatter"))?;
        assert!(
            fm.lines()
                .any(|l| l.trim() == format!("model: {expected_model}")),
            "T-009: {rel} frontmatter must contain `model: {expected_model}`; got:\n{fm}",
        );
        assert!(
            fm.lines()
                .any(|l| l.trim() == format!("effort: {expected_effort}")),
            "T-009: {rel} frontmatter must contain `effort: {expected_effort}`; got:\n{fm}",
        );
    }
    Ok(())
}

#[test]
fn phase_worker_agent_has_full_body_with_no_minijinja_markup() -> TestResult {
    let fx = make_fixture("t009-agent-full-body")?;
    run_init_claude(&fx.root, &[]).success();

    for phase in &["speccy-work", "speccy-decompose", "speccy-ship"] {
        let rel = format!(".claude/agents/{phase}.md");
        let body = read_file(&fx.root, &rel)?;
        let (_, post_fm) = split_frontmatter(&body)
            .ok_or_else(|| format!("{rel} must have a `---` frontmatter fence"))?;
        let non_blank = non_blank_line_count(post_fm);
        assert!(
            non_blank > 10,
            "T-009: {rel} must have a substantive body (>10 non-blank lines in the body); got {non_blank} non-blank body lines",
        );
        assert!(
            !body.contains("{{"),
            "T-009: {rel} must contain no unsubstituted `{{{{` MiniJinja token; got:\n{body}",
        );
        assert!(
            !body.contains("{%"),
            "T-009: {rel} must contain no unsubstituted `{{%` MiniJinja token; got:\n{body}",
        );
        assert!(
            !body.contains("{#"),
            "T-009: {rel} must contain no unsubstituted `{{#` MiniJinja token; got:\n{body}",
        );
    }
    Ok(())
}

// -----------------------------------------------------------------------
// Scenario 3 (CHK-017): Codex path — thin SKILL.md stub names the TOML
// agent file; TOML has model and model_reasoning_effort at top level.
// -----------------------------------------------------------------------

#[test]
fn chk017_codex_skill_stub_names_toml_agent_and_invocation_path() -> TestResult {
    let fx = make_fixture("chk017-codex-stub")?;
    run_init_codex(&fx.root, &[]).success();

    for phase in &["speccy-work", "speccy-decompose", "speccy-ship"] {
        let rel = format!(".agents/skills/{phase}/SKILL.md");
        let body = read_file(&fx.root, &rel)?;
        let count = common::non_blank_line_count_outside_reconcile_partial(&body);
        assert!(
            count <= 10,
            "CHK-017: {rel} must have ≤10 non-blank lines outside reconcile-policy partial markers; got {count}:\n{body}",
        );
        let toml_file = format!(".codex/agents/{phase}.toml");
        assert!(
            body.contains(&toml_file),
            "CHK-017: {rel} must name the TOML agent file `{toml_file}`; got:\n{body}",
        );
        let invocation = format!("/agent {phase}");
        assert!(
            body.contains(&invocation),
            "CHK-017: {rel} must reference `/agent {phase}`; got:\n{body}",
        );
    }
    Ok(())
}

#[test]
fn chk017_codex_toml_has_model_and_effort_at_top_level() -> TestResult {
    let fx = make_fixture("chk017-codex-toml")?;
    run_init_codex(&fx.root, &[]).success();

    for phase in &["speccy-work", "speccy-decompose", "speccy-ship"] {
        let rel = format!(".codex/agents/{phase}.toml");
        let body = read_file(&fx.root, &rel)?;
        // Parse as TOML to verify top-level keys.
        let parsed: toml::Value =
            toml::from_str(&body).map_err(|e| format!("{rel}: invalid TOML: {e}"))?;
        let table = parsed
            .as_table()
            .ok_or_else(|| format!("{rel}: TOML must be a top-level table"))?;
        let model = table
            .get("model")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{rel}: missing top-level `model` key"))?;
        assert_eq!(
            model, "gpt-5.5",
            "CHK-017: {rel} must have `model = \"gpt-5.5\"` at top level",
        );
        let effort = table
            .get("model_reasoning_effort")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{rel}: missing top-level `model_reasoning_effort` key"))?;
        assert_eq!(
            effort, "medium",
            "CHK-017: {rel} must have `model_reasoning_effort = \"medium\"` at top level",
        );
    }
    Ok(())
}

#[test]
fn chk017_codex_toml_has_full_developer_instructions() -> TestResult {
    let fx = make_fixture("chk017-codex-body")?;
    run_init_codex(&fx.root, &[]).success();

    for phase in &["speccy-work", "speccy-decompose", "speccy-ship"] {
        let rel = format!(".codex/agents/{phase}.toml");
        let body = read_file(&fx.root, &rel)?;
        let parsed: toml::Value =
            toml::from_str(&body).map_err(|e| format!("{rel}: invalid TOML: {e}"))?;
        let table = parsed
            .as_table()
            .ok_or_else(|| format!("{rel}: TOML must be a top-level table"))?;
        let instructions = table
            .get("developer_instructions")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{rel}: missing `developer_instructions` key"))?;
        let non_blank = non_blank_line_count(instructions);
        assert!(
            non_blank > 10,
            "CHK-017: {rel} `developer_instructions` must be substantive (>10 non-blank lines); got {non_blank}",
        );
        assert!(
            !instructions.contains("{{"),
            "CHK-017: {rel} `developer_instructions` must contain no `{{{{` MiniJinja token",
        );
        assert!(
            !instructions.contains("{%"),
            "CHK-017: {rel} `developer_instructions` must contain no `{{%` MiniJinja token",
        );
    }
    Ok(())
}

// -----------------------------------------------------------------------
// Scenario 4 (CHK-022 extension): Confirm the three phase-worker agents
// ARE created and that the two interactive-skill agents are NOT.
// -----------------------------------------------------------------------

#[test]
fn phase_worker_agent_files_are_created_by_claude_init() -> TestResult {
    let fx = make_fixture("t009-phase-agents-exist")?;
    run_init_claude(&fx.root, &[]).success();

    for phase in &["speccy-work", "speccy-decompose", "speccy-ship"] {
        let path = format!(".claude/agents/{phase}.md");
        assert!(
            fx.root.join(&path).exists(),
            "T-009: `speccy init --host claude-code` must create `{path}` (phase-worker agent)",
        );
    }
    Ok(())
}

#[test]
fn phase_worker_agent_files_are_created_by_codex_init() -> TestResult {
    let fx = make_fixture("t009-codex-phase-agents-exist")?;
    run_init_codex(&fx.root, &[]).success();

    for phase in &["speccy-work", "speccy-decompose", "speccy-ship"] {
        let path = format!(".codex/agents/{phase}.toml");
        assert!(
            fx.root.join(&path).exists(),
            "T-009: `speccy init --host codex` must create `{path}` (phase-worker agent)",
        );
    }
    Ok(())
}

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

/// Extract the YAML frontmatter content (between the `---` fences).
fn extract_frontmatter(source: &str) -> Option<&str> {
    let after_open = source
        .strip_prefix("---\n")
        .or_else(|| source.strip_prefix("---\r\n"))?;
    let close_idx = after_open.find("\n---")?;
    after_open.get(..close_idx)
}

/// Split a file into (frontmatter, body). Returns `None` if no `---` fences.
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
