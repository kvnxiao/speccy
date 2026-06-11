#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! The `tracing` subscriber writes only to stderr, never stdout.
//!
//! Drives the built `speccy` binary so the real `fn main()` subscriber init
//! is exercised under `RUST_LOG=trace`, then asserts a contracted-stdout
//! command (`speccy status --json`) emits stdout that parses as a single
//! well-formed JSON value with no log line interleaved, *while* the binary's
//! startup TRACE diagnostic is observed on stderr. Because the binary really
//! emits a diagnostic at TRACE, this test distinguishes a stderr-wired
//! subscriber from a stdout-wired one: a stdout-wired subscriber would
//! interleave the formatter line ahead of the JSON envelope and break the
//! `serde_json::from_str(&stdout)` parse. Covers SPEC-0058 CHK-008.

mod common;

use assert_cmd::Command;
use common::TestResult;
use common::Workspace;
use common::bootstrap_tasks_md;
use common::spec_md_template;
use common::write_spec;

#[test]
fn status_json_diagnostic_lands_on_stderr_not_stdout() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    // `RUST_LOG=trace` lowers the filter below the startup TRACE the binary
    // emits, so a real diagnostic is in flight during the command. The
    // contract: it appears on stderr, and stdout stays clean JSON.
    cmd.env("RUST_LOG", "trace")
        .arg("status")
        .arg("--json")
        .current_dir(ws.root.as_std_path());

    let output = cmd.assert().success().get_output().clone();
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;

    // The startup diagnostic the binary emits at TRACE must appear on
    // stderr. This is what makes the stdout assertion below non-vacuous: the
    // event genuinely exists, so a subscriber miswired to stdout would put it
    // on stdout (failing the clean-JSON parse) rather than here.
    assert!(
        stderr.contains("speccy starting"),
        "expected the startup TRACE diagnostic on stderr under RUST_LOG=trace; \
         stderr was: {stderr:?}"
    );

    // The entire stdout payload parses as a single JSON value — the in-flight
    // diagnostic did not interleave onto stdout.
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not clean JSON: {e}; stdout was: {stdout:?}"))?;
    assert_eq!(parsed.get("schema_version"), Some(&serde_json::json!(1)));
    // The diagnostic text must be wholly absent from stdout.
    assert!(
        !stdout.contains("speccy starting"),
        "diagnostic leaked onto stdout; stdout was: {stdout:?}"
    );

    Ok(())
}
