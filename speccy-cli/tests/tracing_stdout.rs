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
//! well-formed JSON value with no log line interleaved, *while* TRACE
//! diagnostics are observed on stderr. Because the binary really emits
//! diagnostics at TRACE, this test distinguishes a stderr-wired subscriber
//! from a stdout-wired one: a stdout-wired subscriber would interleave the
//! formatter line ahead of the JSON envelope and break the
//! `serde_json::from_str(&stdout)` parse.

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

    // Some diagnostic output must land on stderr under RUST_LOG=trace.
    // This is what makes the stdout assertion below non-vacuous: events
    // are genuinely in flight, so a subscriber miswired to stdout would
    // interleave them there (failing the clean-JSON parse) rather than
    // here. The diagnostic's wording is deliberately not pinned.
    assert!(
        !stderr.trim().is_empty(),
        "expected TRACE diagnostics on stderr under RUST_LOG=trace; stderr was empty"
    );

    // The entire stdout payload parses as a single JSON value — the
    // in-flight diagnostics did not interleave onto stdout.
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout is not clean JSON: {e}; stdout was: {stdout:?}"))?;
    assert_eq!(parsed.get("schema_version"), Some(&serde_json::json!(1)));

    Ok(())
}
