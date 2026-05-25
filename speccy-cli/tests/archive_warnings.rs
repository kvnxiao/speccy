#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! End-to-end tests for `speccy archive` supersession-chain orphan
//! warnings. Covers SPEC-0042 T-004 / REQ-008 / CHK-020, CHK-021, CHK-022.

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use common::TestResult;
use common::Workspace;
use common::write_spec;
use serde_json::Value;

fn init_git_repo(root: &Utf8Path) -> TestResult {
    let run = |args: &[&str]| -> TestResult {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(root.as_std_path())
            .status()?;
        if !status.success() {
            return Err(format!("git {args:?} failed").into());
        }
        Ok(())
    };
    run(&["init", "-q"])?;
    run(&["config", "user.email", "test@example.com"])?;
    run(&["config", "user.name", "Test"])?;
    run(&["config", "commit.gpgsign", "false"])?;
    run(&["add", "-A"])?;
    run(&["commit", "-q", "-m", "init"])?;
    Ok(())
}

/// SPEC.md body whose frontmatter declares `id`, `slug`, `title`,
/// `status`, `created`, and a `supersedes` block. Mirrors
/// `spec_md_template` from `common::` but with a `supersedes` field.
fn spec_md_with_supersedes(id: &str, status: &str, supersedes: &[&str]) -> String {
    let supersedes_yaml = if supersedes.is_empty() {
        "supersedes: []".to_owned()
    } else {
        let mut s = String::from("supersedes:\n");
        for sup in supersedes {
            s.push_str("  - ");
            s.push_str(sup);
            s.push('\n');
        }
        s.trim_end().to_owned()
    };
    format!(
        "---\nid: {id}\nslug: x\ntitle: Example {id}\nstatus: {status}\ncreated: 2026-05-11\n{supersedes_yaml}\n---\n\n# {id}\n\n<goals>\nExample goals.\n</goals>\n\n<non-goals>\nExample non-goals.\n</non-goals>\n\n<user-stories>\n- Example user story.\n</user-stories>\n\n<requirement id=\"REQ-001\">\n### REQ-001: First\nBody.\n\n<done-when>\n- placeholder.\n</done-when>\n\n<behavior>\n- placeholder.\n</behavior>\n\n<scenario id=\"CHK-001\">\nGiven REQ-001, when the suite runs, then it covers REQ-001.\n</scenario>\n</requirement>\n\n## Changelog\n\n<changelog>\n| Date | Author | Summary |\n|------|--------|---------|\n| 2026-05-11 | t | init |\n</changelog>\n",
    )
}

#[test]
fn archive_emits_orphan_warning_when_sole_declarer() -> TestResult {
    // CHK-020: SPEC-0019 active superseded; SPEC-0021 sole declarer.
    // Archiving SPEC-0021 must warn about SPEC-0019.
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0019-old",
        &spec_md_with_supersedes("SPEC-0019", "superseded", &[]),
        None,
    )?;
    write_spec(
        &ws.root,
        "0021-new",
        &spec_md_with_supersedes("SPEC-0021", "implemented", &["SPEC-0019"]),
        None,
    )?;
    init_git_repo(&ws.root)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0021")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().success();
    let output = assert.get_output();
    let stdout = std::str::from_utf8(&output.stdout)?.trim();
    let stderr = std::str::from_utf8(&output.stderr)?;

    assert!(
        stderr.contains("SPEC-0019") && stderr.contains("SPEC-0021"),
        "stderr should name both specs: {stderr}"
    );
    assert!(
        stderr.contains("warning:"),
        "stderr should carry warning prefix: {stderr}"
    );

    let v: Value = serde_json::from_str(stdout)?;
    let warnings = v.pointer("/warnings").expect("warnings present");
    assert_eq!(
        warnings,
        &serde_json::json!([{"spec":"SPEC-0019","reason":"orphaned-supersession"}]),
        "warnings array shape: {stdout}"
    );
    Ok(())
}

#[test]
fn archive_older_spec_in_pair_emits_no_warning() -> TestResult {
    // CHK-021: archiving SPEC-0019 (the older, superseded one) — the
    // natural archive case. No warnings.
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0019-old",
        &spec_md_with_supersedes("SPEC-0019", "superseded", &[]),
        None,
    )?;
    write_spec(
        &ws.root,
        "0021-new",
        &spec_md_with_supersedes("SPEC-0021", "implemented", &["SPEC-0019"]),
        None,
    )?;
    init_git_repo(&ws.root)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0019")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().success();
    let output = assert.get_output();
    let stdout = std::str::from_utf8(&output.stdout)?.trim();
    let stderr = std::str::from_utf8(&output.stderr)?;

    assert!(
        !stderr.contains("warning:"),
        "stderr should not carry warning: {stderr}"
    );
    let v: Value = serde_json::from_str(stdout)?;
    assert_eq!(v.pointer("/warnings"), Some(&serde_json::json!([])));
    Ok(())
}

#[test]
fn archive_multi_declarer_emits_no_warning() -> TestResult {
    // CHK-022: SPEC-0019 superseded; SPEC-0021 and SPEC-0022 both
    // declare supersedes: [SPEC-0019]. Archiving SPEC-0021 leaves
    // SPEC-0022 as an explainer — no orphan.
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0019-old",
        &spec_md_with_supersedes("SPEC-0019", "superseded", &[]),
        None,
    )?;
    write_spec(
        &ws.root,
        "0021-new-a",
        &spec_md_with_supersedes("SPEC-0021", "implemented", &["SPEC-0019"]),
        None,
    )?;
    write_spec(
        &ws.root,
        "0022-new-b",
        &spec_md_with_supersedes("SPEC-0022", "implemented", &["SPEC-0019"]),
        None,
    )?;
    init_git_repo(&ws.root)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0021")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().success();
    let output = assert.get_output();
    let stdout = std::str::from_utf8(&output.stdout)?.trim();
    let stderr = std::str::from_utf8(&output.stderr)?;

    assert!(
        !stderr.contains("warning:"),
        "stderr should not carry warning when multiple declarers exist: {stderr}"
    );
    let v: Value = serde_json::from_str(stdout)?;
    assert_eq!(v.pointer("/warnings"), Some(&serde_json::json!([])));
    Ok(())
}

#[test]
fn archive_with_empty_supersedes_emits_no_warning() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0030-solo",
        &spec_md_with_supersedes("SPEC-0030", "implemented", &[]),
        None,
    )?;
    init_git_repo(&ws.root)?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("archive")
        .arg("SPEC-0030")
        .arg("--json")
        .current_dir(ws.root.as_std_path());
    let assert = cmd.assert().success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout)?.trim();
    let v: Value = serde_json::from_str(stdout)?;
    assert_eq!(v.pointer("/warnings"), Some(&serde_json::json!([])));
    Ok(())
}
