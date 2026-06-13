//! XML-001 rule: unbalanced foreign-tag detection in parsed artifacts.
//!
//! A parsed artifact (SPEC.md / TASKS.md / REPORT.md) that
//! leaks an orphan foreign (non-whitelisted) XML tag — a close with no
//! matching preceding open, or a non-void open with no matching following
//! close — produces exactly one `XML-001` Error diagnostic naming the
//! artifact path and the offending 1-indexed source line.
//!
//! Detection lives here, in the lint engine, not in the scanner: the
//! scanner keeps its foreign-HTML
//! passthrough untouched and only exposes the foreign-tag view via
//! [`scan_foreign_tags`]. Balance is computed name-scoped with a per-name
//! stack, fence-aware, and does not enforce cross-name nesting.
//!
//! This module covers the three parsed-document artifacts plus the
//! on-demand `journal/T-NNN.md` files: journals are
//! defense-in-depth, reached on demand via the `JNL-*` path-derivation
//! pattern rather than a `ParsedSpec` field.

use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;
use crate::parse::journal_xml::JOURNAL_ELEMENT_NAMES;
use crate::parse::report_xml::REPORT_ELEMENT_NAMES;
use crate::parse::spec_xml::SPECCY_ELEMENT_NAMES;
use crate::parse::task_xml::TASKS_ELEMENT_NAMES;
use crate::parse::xml_scanner::collect_code_fence_byte_ranges;
use crate::parse::xml_scanner::is_void_element_name;
use crate::parse::xml_scanner::scan_foreign_tags;
use camino::Utf8Path;
use std::collections::HashMap;

const XML_001: &str = "XML-001";

/// Append every `XML-001` diagnostic for one spec across its parsed
/// document artifacts (SPEC.md / TASKS.md / REPORT.md) and its existing
/// per-task `journal/T-NNN.md` files.
pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    if let Some(spec_doc) = spec.spec_doc_ok() {
        balance_pass(
            spec,
            &spec_doc.raw,
            &spec.spec_md_path,
            SPECCY_ELEMENT_NAMES,
            out,
        );
    }
    if let Some(tasks_md) = spec.tasks_md_ok()
        && let Some(tasks_path) = spec.tasks_md_path.as_deref()
    {
        balance_pass(spec, &tasks_md.raw, tasks_path, TASKS_ELEMENT_NAMES, out);
    }
    if let Some(report_md) = spec.report_md_ok() {
        // `ParsedSpec` has no `report_md_path` field; REPORT.md sits
        // beside SPEC.md and TASKS.md in the spec directory.
        balance_pass(
            spec,
            &report_md.raw,
            &spec.dir.join("REPORT.md"),
            REPORT_ELEMENT_NAMES,
            out,
        );
    }
    journal_pass(spec, out);
}

/// Run the balance pass over each existing `journal/T-NNN.md`.
/// Journal paths are derived from the parsed tasks the same way
/// the `JNL-*` rules do — `spec.dir.join("journal")` plus
/// `T-NNN.md` per task — and read on demand; no journal field is added to
/// `ParsedSpec`. Missing journal files are simply skipped (their
/// absence/presence is the `JNL-*` family's concern, not `XML-001`'s).
fn journal_pass(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    let Some(tasks_md) = spec.tasks_md_ok() else {
        return;
    };
    let journal_dir = spec.dir.join("journal");
    for task in &tasks_md.tasks {
        let journal_path = journal_dir.join(format!("{}.md", task.id));
        if !journal_path.exists() {
            continue;
        }
        let Ok(raw) = fs_err::read_to_string(journal_path.as_std_path()) else {
            // An unreadable journal file is the `JNL-*` family's concern;
            // `XML-001` only balances files it can read.
            continue;
        };
        balance_pass(spec, &raw, &journal_path, JOURNAL_ELEMENT_NAMES, out);
    }
}

/// Run the name-scoped balance pass over one artifact's raw source and
/// emit one `XML-001` per orphan foreign tag.
fn balance_pass(
    spec: &ParsedSpec,
    raw: &str,
    path: &Utf8Path,
    whitelist: &[&str],
    out: &mut Vec<Diagnostic>,
) {
    let fences = collect_code_fence_byte_ranges(raw);
    let foreign = scan_foreign_tags(raw, &fences, whitelist);

    // Per-name stack of open-tag lines. A close pops its name's stack; an
    // empty stack at a close marks a dangling-close orphan. Any line left
    // on a stack after the walk is a dangling-open orphan.
    let mut open_lines: HashMap<String, Vec<u32>> = HashMap::new();

    for tag in &foreign {
        if tag.is_close {
            // An empty (or absent) stack means this close has no matching
            // preceding open: dangling close.
            let matched = open_lines.get_mut(&tag.name).and_then(Vec::pop).is_some();
            if !matched {
                out.push(orphan_diagnostic(spec, path, tag.line, &tag.name));
            }
        } else if !is_void_element_name(&tag.name) {
            // Void-named opens are never pushed: they have no close by
            // definition.
            open_lines
                .entry(tag.name.clone())
                .or_default()
                .push(tag.line);
        }
    }

    // Every line still on a stack is a dangling open with no matching close.
    for (name, lines) in &open_lines {
        for &line in lines {
            out.push(orphan_diagnostic(spec, path, line, name));
        }
    }
}

/// Build the shared `XML-001` diagnostic. One template, parameterized only
/// by the tag name: the open-orphan and close-orphan cases share
/// this wording and differ solely in the substituted name and line. The
/// path and 1-indexed line are carried by the diagnostic location and
/// surfaced by the renderer.
fn orphan_diagnostic(spec: &ParsedSpec, path: &Utf8Path, line: u32, name: &str) -> Diagnostic {
    Diagnostic::with_location(
        XML_001,
        Level::Error,
        spec.spec_id.clone(),
        path.to_path_buf(),
        line,
        format!("unbalanced foreign XML tag `{name}` (no matching open/close pair)"),
    )
}
