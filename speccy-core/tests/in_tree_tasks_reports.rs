#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]

//! SPEC-0022 T-006 corpus integration test.
//!
//! After the SPEC-0022 migration runs across every in-tree
//! `.speccy/specs/NNNN-*/TASKS.md` and `REPORT.md`, three invariants must
//! hold:
//!
//! 1. Every TASKS.md parses cleanly with
//!    [`speccy_core::parse::parse_task_xml`], and every task carries a
//!    non-empty `<task-scenarios>` body (no empty bodies smuggled through).
//! 2. Every REPORT.md parses cleanly with
//!    [`speccy_core::parse::parse_report_xml`].
//! 3. Every coverage element's `req` resolves to a `<requirement id=...>` in
//!    the parent SPEC.md, and every scenario id resolves to a `<scenario
//!    id=...>` nested under that requirement.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::parse::parse_report_xml;
use speccy_core::parse::parse_spec_xml;
use speccy_core::parse::parse_task_xml;

fn workspace_root() -> Utf8PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let manifest = Utf8PathBuf::from(manifest_dir);
    manifest
        .parent()
        .expect("speccy-core has a parent")
        .to_path_buf()
}

fn spec_dirs(root: &Utf8Path) -> Vec<Utf8PathBuf> {
    let specs_dir = root.join(".speccy").join("specs");
    let mut out = Vec::new();
    for entry in fs_err::read_dir(specs_dir.as_std_path()).expect("read .speccy/specs") {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        let utf8 =
            Utf8PathBuf::from_path_buf(path).expect("non-utf8 spec dir name should not exist");
        if utf8.is_dir() && utf8.join("SPEC.md").is_file() {
            out.push(utf8);
        }
    }
    out.sort();
    out
}

#[test]
fn every_in_tree_tasks_md_parses_and_has_populated_scenarios() {
    let root = workspace_root();
    let dirs = spec_dirs(&root);
    assert!(
        !dirs.is_empty(),
        "expected at least one spec under .speccy/specs/",
    );
    let mut parse_failures: Vec<String> = Vec::new();
    let mut empty_scenarios: Vec<String> = Vec::new();
    let mut tasks_md_count: usize = 0;
    for d in &dirs {
        let tasks_path = d.join("TASKS.md");
        if !tasks_path.is_file() {
            continue;
        }
        tasks_md_count = tasks_md_count.saturating_add(1);
        let source = fs_err::read_to_string(tasks_path.as_std_path())
            .expect("reading TASKS.md should succeed");
        let doc = match parse_task_xml(&source, &tasks_path) {
            Ok(doc) => doc,
            Err(e) => {
                parse_failures.push(format!("{tasks_path}: {e}"));
                continue;
            }
        };
        for task in &doc.tasks {
            if task.scenarios_body.trim().is_empty() {
                empty_scenarios.push(format!(
                    "{tasks_path}: task {} has empty <task-scenarios> body",
                    task.id,
                ));
            }
        }
    }
    assert!(
        parse_failures.is_empty(),
        "TASKS.md files failed to parse with parse_task_xml:\n{}",
        parse_failures.join("\n"),
    );
    assert!(
        empty_scenarios.is_empty(),
        "TASKS.md files contain tasks with empty <task-scenarios> bodies:\n{}",
        empty_scenarios.join("\n"),
    );
    assert!(
        tasks_md_count > 0,
        "expected at least one TASKS.md under .speccy/specs/, found none",
    );
}

#[test]
fn every_in_tree_report_md_parses_and_resolves_against_parent_spec() {
    let root = workspace_root();
    let dirs = spec_dirs(&root);
    let mut parse_failures: Vec<String> = Vec::new();
    let mut dangling: Vec<String> = Vec::new();
    let mut report_md_count: usize = 0;
    for d in &dirs {
        let report_path = d.join("REPORT.md");
        if !report_path.is_file() {
            continue;
        }
        report_md_count = report_md_count.saturating_add(1);
        let spec_path = d.join("SPEC.md");
        let spec_source = fs_err::read_to_string(spec_path.as_std_path())
            .expect("reading SPEC.md should succeed");
        let spec = match parse_spec_xml(&spec_source, &spec_path) {
            Ok(s) => s,
            Err(e) => {
                parse_failures.push(format!("{spec_path}: spec parse failed: {e}"));
                continue;
            }
        };
        let report_source = fs_err::read_to_string(report_path.as_std_path())
            .expect("reading REPORT.md should succeed");
        let report = match parse_report_xml(&report_source, &report_path) {
            Ok(r) => r,
            Err(e) => {
                parse_failures.push(format!("{report_path}: {e}"));
                continue;
            }
        };
        for cov in &report.coverage {
            let Some(req) = spec.requirements.iter().find(|r| r.id == cov.req) else {
                dangling.push(format!(
                    "{report_path}: <coverage req=\"{}\"> does not resolve in {spec_path}",
                    cov.req,
                ));
                continue;
            };
            for chk in &cov.scenarios {
                if !req.scenarios.iter().any(|s| &s.id == chk) {
                    dangling.push(format!(
                        "{report_path}: <coverage req=\"{}\"> scenario `{chk}` does not resolve under {} in {spec_path}",
                        cov.req, req.id,
                    ));
                }
            }
        }
    }
    assert!(
        parse_failures.is_empty(),
        "REPORT.md or sibling SPEC.md files failed to parse:\n{}",
        parse_failures.join("\n"),
    );
    assert!(
        dangling.is_empty(),
        "REPORT.md files contain coverage rows that do not resolve against the parent SPEC.md:\n{}",
        dangling.join("\n"),
    );
    assert!(
        report_md_count > 0,
        "expected at least one REPORT.md under .speccy/specs/, found none",
    );
}
