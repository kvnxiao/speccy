#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Cross-command guard: every `speccy` subcommand that accepts `--json`
//! emits a JSON envelope whose **first serialized field** is
//! `schema_version: 1`.
//!
//! This file is the registry for that contract. **Any 8th `--json` command
//! must enroll here** — add its `(label, args)` row to `COMMANDS` in
//! [`all_json_commands_emit_schema_version_first`]. Before this guard, only
//! `context` and `status` pinned the first-field position; this pins all
//! seven at once.
//!
//! Why a regex on the raw bytes: `schema_version` is the first struct field
//! in every envelope, so serde — serializing in declaration order — places
//! it first whether a command emits compact (`context`, `next`, `vacancy`,
//! `journal show`, `archive`) or pretty-printed (`status`, `verify`) JSON.
//! `\s*` tolerates both forms; the trailing `,` confirms a second field
//! follows (no envelope is single-field).

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::task_xml;
use common::tasks_md_xml;
use common::write_spec;
use regex::Regex;
use serde_json::Value;

/// Initialise a git repo and commit the workspace so `archive` (which
/// relocates a spec dir via `git mv`) can operate. Mirrors the helper in
/// `archive_json.rs`.
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

/// Assert that `stdout` is JSON whose first serialized field is
/// `schema_version: 1`.
fn assert_envelope(stdout: &[u8], label: &str) -> TestResult {
    let text =
        std::str::from_utf8(stdout).map_err(|e| format!("{label}: stdout is not UTF-8: {e}"))?;
    let json: Value = serde_json::from_str(text)
        .map_err(|e| format!("{label}: stdout did not parse as JSON ({e}); got: {text:?}"))?;
    assert_eq!(
        json.get("schema_version").and_then(Value::as_u64),
        Some(1),
        "{label}: schema_version must equal 1",
    );
    let first_field = Regex::new(r#"^\{\s*"schema_version"\s*:\s*1\s*,"#)
        .map_err(|e| format!("first-field regex failed to compile: {e}"))?;
    let prefix: String = text.chars().take(64).collect();
    assert!(
        first_field.is_match(text),
        "{label}: schema_version must be the first serialized field; got prefix {prefix:?}",
    );
    Ok(())
}

#[test]
fn all_json_commands_emit_schema_version_first() -> TestResult {
    let ws = Workspace::new()?;

    // SPEC-0001: in-progress spec with a task and a per-task journal —
    // exercises status / next / context / journal-show selectors.
    let spec1 = write_spec(
        &ws.root,
        "0001-alpha",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&tasks_md_xml("SPEC-0001", &task_xml("T-001", "in-review"))),
    )?;
    let journal = spec1.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    fs_err::write(
        journal.join("T-001.md").as_std_path(),
        concat!(
            "---\n",
            "spec: SPEC-0001\n",
            "task: T-001\n",
            "generated_at: 2026-05-21T18:00:00Z\n",
            "---\n\n",
            "<implementer date=\"2026-05-21T18:00:00Z\" model=\"m\" round=\"1\">\n",
            "impl note\n",
            "</implementer>\n",
        ),
    )?;

    // SPEC-0002: implemented spec — archivable without --force, so it backs
    // the mutating command we run last.
    write_spec(
        &ws.root,
        "0002-beta",
        &spec_md_template("SPEC-0002", "implemented"),
        Some(&tasks_md_xml("SPEC-0002", &task_xml("T-001", "completed"))),
    )?;

    // Lock SPEC-0001 so its context/consistency bundle is clean.
    Command::cargo_bin("speccy")?
        .args(["lock", "SPEC-0001"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success();

    // Commit so `archive`'s `git mv` has tracked files to relocate.
    init_git_repo(&ws.root)?;

    // `archive` mutates (relocates SPEC-0002 out of `specs/`), so it is last.
    // Exit codes are intentionally not asserted: every command writes its
    // envelope to stdout regardless of exit status (e.g. `verify` emits the
    // report before returning a non-zero gate code), and the contract under
    // test is the envelope shape, not command success.
    let commands: [(&str, &[&str]); 7] = [
        ("status --json", &["status", "--json"]),
        ("next --json", &["next", "--json"]),
        ("context --json", &["context", "SPEC-0001/T-001", "--json"]),
        ("verify --json", &["verify", "--json"]),
        ("vacancy --json", &["vacancy", "--json"]),
        (
            "journal show --json",
            &["journal", "show", "SPEC-0001/T-001", "--json"],
        ),
        ("archive --json", &["archive", "SPEC-0002", "--json"]),
    ];

    for (label, args) in commands {
        let output = Command::cargo_bin("speccy")?
            .args(args)
            .current_dir(ws.root.as_std_path())
            .output()?;
        assert_envelope(&output.stdout, label)?;
    }
    Ok(())
}
