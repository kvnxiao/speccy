#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert!/assert_eq! macros and return Result for ? propagation in setup"
)]
//! Meta-test: iterate every directory under `tests/fixtures/lint/`,
//! parse + lint, and check the diagnostic set against the fixture's
//! header expectations.
//!
//! Each fixture's SPEC.md must include an HTML comment in its body
//! (after the frontmatter fence) like:
//!
//! ```text
//! <!--
//! expects: CODE-NNN, CODE-NNN
//! not: CODE-NNN
//! -->
//! ```

mod lint_common;

use camino::Utf8PathBuf;
use lint_common::TestResult;
use lint_common::parse_fixture;
use lint_common::run_lint;
use speccy_core::lint::types::Diagnostic;
use std::collections::HashSet;
use std::path::Path;

#[test]
fn every_fixture_produces_expected_diagnostics() -> TestResult {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixtures_root = Path::new(manifest_dir).join("tests/fixtures/lint");
    assert!(
        fixtures_root.exists(),
        "lint fixtures root must exist: {}",
        fixtures_root.display()
    );

    let mut visited = 0usize;
    for entry in fs_err::read_dir(&fixtures_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        visited = visited.saturating_add(1);
        check_one_fixture(&path)?;
    }
    assert!(visited > 0, "no fixtures discovered");
    Ok(())
}

fn check_one_fixture(dir: &Path) -> TestResult {
    let utf8_dir = Utf8PathBuf::from_path_buf(dir.to_path_buf())
        .map_err(|p| format!("fixture path must be UTF-8: {}", p.display()))?;
    let spec_md_path = utf8_dir.join("SPEC.md");

    let spec_md_content = fs_err::read_to_string(spec_md_path.as_std_path())?;
    let (expects, forbids) = parse_expectations(&spec_md_content);

    let tasks_path = utf8_dir.join("TASKS.md");
    let tasks_md_path = if tasks_path.exists() {
        Some(tasks_path)
    } else {
        None
    };

    let placeholder_dir = tempfile::tempdir()?;
    let fx = lint_common::Fixture {
        _dir: placeholder_dir,
        spec_md_path,
        tasks_md_path,
        dir_path: utf8_dir.clone(),
    };
    let parsed = parse_fixture(&fx);
    let diags = run_lint(&[parsed]);
    let emitted: HashSet<&str> = diags.iter().map(|d| d.code).collect();

    for required in &expects {
        assert!(
            emitted.contains(required.as_str()),
            "fixture `{name}` expected `{required}` but got: {got:?}",
            name = utf8_dir.file_name().unwrap_or(""),
            got = collect_codes(&diags),
        );
    }
    for forbidden in &forbids {
        assert!(
            !emitted.contains(forbidden.as_str()),
            "fixture `{name}` forbade `{forbidden}` but got: {got:?}",
            name = utf8_dir.file_name().unwrap_or(""),
            got = collect_codes(&diags),
        );
    }
    Ok(())
}

fn collect_codes(diags: &[Diagnostic]) -> Vec<&'static str> {
    diags.iter().map(|d| d.code).collect()
}

fn parse_expectations(content: &str) -> (Vec<String>, Vec<String>) {
    let Some(start) = content.find("<!--") else {
        return (Vec::new(), Vec::new());
    };
    let after = content.get(start.saturating_add(4)..).unwrap_or("");
    let Some(end) = after.find("-->") else {
        return (Vec::new(), Vec::new());
    };
    let block = after.get(..end).unwrap_or("");

    let mut expects = Vec::new();
    let mut forbids = Vec::new();
    for line in block.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("expects:") {
            expects.extend(split_codes(rest));
        } else if let Some(rest) = line.strip_prefix("not:") {
            forbids.extend(split_codes(rest));
        }
    }
    (expects, forbids)
}

fn split_codes(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_owned())
        .filter(|x| !x.is_empty())
        .collect()
}
