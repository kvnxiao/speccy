#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for the JSON output contract. Covers SPEC-0004 CHK-009.

mod common;

use common::TestResult;
use common::Workspace;
use common::bootstrap_tasks_md;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use speccy_cli::status::StatusArgs;
use speccy_cli::status::run;

fn render_json(root: &camino::Utf8Path) -> TestResult<String> {
    let mut buf: Vec<u8> = Vec::new();
    run(StatusArgs { json: true }, root, &mut buf)?;
    Ok(String::from_utf8(buf)?)
}

#[test]
fn contract() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;
    // Even a 'dropped' spec must appear in JSON regardless of status.
    write_spec(
        &ws.root,
        "0002-dropped",
        &spec_md_template("SPEC-0002", "dropped"),
        &valid_spec_toml(),
        None,
    )?;

    let json_text = render_json(&ws.root)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_text)?;

    // Schema version is the first field.
    assert_eq!(parsed.get("schema_version"), Some(&serde_json::json!(1)));
    // repo_sha may be empty or a 40-char hex; both are acceptable.
    let sha = parsed
        .get("repo_sha")
        .and_then(|v| v.as_str())
        .expect("repo_sha must be a string");
    assert!(sha.is_empty() || sha.len() == 40, "repo_sha: {sha:?}");

    // specs array contains BOTH specs (no filtering by status).
    let specs = parsed
        .get("specs")
        .and_then(|v| v.as_array())
        .expect("specs must be an array");
    assert_eq!(specs.len(), 2);

    // Each entry has the required fields per SPEC.md REQ-007.
    for spec in specs {
        for key in [
            "id",
            "slug",
            "title",
            "status",
            "supersedes",
            "superseded_by",
            "tasks",
            "stale",
            "stale_reasons",
            "open_questions",
            "lint",
        ] {
            assert!(
                spec.get(key).is_some(),
                "spec entry missing key `{key}`: {spec}",
            );
        }
        // tasks object has the four count fields.
        let tasks = spec.get("tasks").expect("tasks object");
        for key in ["open", "in_progress", "awaiting_review", "done"] {
            assert!(
                tasks.get(key).is_some(),
                "tasks missing key `{key}`: {tasks}",
            );
        }
        // lint object has the three arrays.
        let lint = spec.get("lint").expect("lint object");
        for key in ["errors", "warnings", "info"] {
            assert!(
                lint.get(key).is_some_and(serde_json::Value::is_array),
                "lint missing array `{key}`: {lint}",
            );
        }
    }

    // Workspace-level lint block exists.
    let lint = parsed.get("lint").expect("workspace lint block");
    for key in ["errors", "warnings", "info"] {
        assert!(
            lint.get(key).is_some_and(serde_json::Value::is_array),
            "workspace lint missing array `{key}`: {lint}",
        );
    }

    Ok(())
}

#[test]
fn output_is_deterministic_across_runs() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-active",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;
    write_spec(
        &ws.root,
        "0002-active",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        Some(&bootstrap_tasks_md("SPEC-0002")),
    )?;

    let first = render_json(&ws.root)?;
    let second = render_json(&ws.root)?;
    assert_eq!(
        first, second,
        "two consecutive --json renders must be byte-identical"
    );
    Ok(())
}

#[test]
fn lint_diagnostics_are_structured_objects() -> TestResult {
    let ws = Workspace::new()?;
    // After SPEC-0019 SPC-001 fires when a stray per-spec `spec.toml`
    // is present (the marker tree is the new spec carrier).
    let dir = ws.root.join(".speccy").join("specs").join("0001-broken");
    fs_err::create_dir_all(dir.as_std_path())?;
    fs_err::write(
        dir.join("SPEC.md").as_std_path(),
        spec_md_template("SPEC-0001", "in-progress"),
    )?;
    fs_err::write(dir.join("spec.toml").as_std_path(), "schema_version = 1\n")?;

    let json_text = render_json(&ws.root)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_text)?;
    let specs = parsed
        .get("specs")
        .and_then(|v| v.as_array())
        .expect("specs array");
    let only = specs.first().expect("one spec entry");
    let errors = only
        .pointer("/lint/errors")
        .and_then(|v| v.as_array())
        .expect("lint.errors array");
    assert!(!errors.is_empty(), "expected at least one error: {only}");
    let first = errors.first().expect("first error");
    // Structured object, not a string.
    assert!(first.is_object(), "diagnostic must be a JSON object");
    for key in ["code", "level", "message"] {
        assert!(
            first.get(key).is_some(),
            "diagnostic missing key `{key}`: {first}",
        );
    }
    Ok(())
}

#[test]
fn stale_reasons_in_declared_order() -> TestResult {
    let ws = Workspace::new()?;
    // Hash mismatch with no mtime drift -> only HashDrift.
    let spec_md = spec_md_template("SPEC-0001", "in-progress");
    let tasks_md = "---\nspec: SPEC-0001\nspec_hash_at_generation: 0000000000000000000000000000000000000000000000000000000000000000\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: SPEC-0001\n\n<tasks spec=\"SPEC-0001\">\n</tasks>\n".to_owned();
    write_spec(
        &ws.root,
        "0001-stale",
        &spec_md,
        &valid_spec_toml(),
        Some(&tasks_md),
    )?;

    let json_text = render_json(&ws.root)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_text)?;
    let specs = parsed
        .get("specs")
        .and_then(|v| v.as_array())
        .expect("specs array");
    let only = specs.first().expect("one spec entry");
    let reasons = only
        .get("stale_reasons")
        .and_then(|v| v.as_array())
        .expect("stale_reasons array");
    let reason_strings: Vec<&str> = reasons.iter().filter_map(|v| v.as_str()).collect();
    assert!(reason_strings.contains(&"hash-drift"));
    Ok(())
}
