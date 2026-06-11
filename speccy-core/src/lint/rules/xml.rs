//! XML-001 rule: unbalanced foreign-tag detection in parsed artifacts.
//!
//! SPEC-0057. A parsed artifact (SPEC.md / TASKS.md / REPORT.md) that
//! leaks an orphan foreign (non-whitelisted) XML tag — a close with no
//! matching preceding open, or a non-void open with no matching following
//! close — produces exactly one `XML-001` Error diagnostic naming the
//! artifact path and the offending 1-indexed source line.
//!
//! Detection lives here, in the lint engine, not in the scanner (SPEC
//! DEC-001): the scanner keeps its SPEC-0020 DEC-002 foreign-HTML
//! passthrough untouched and only exposes the foreign-tag view via
//! [`scan_foreign_tags`]. Balance is computed name-scoped with a per-name
//! stack, fence-aware, and does not enforce cross-name nesting (DEC-002).
//!
//! This module covers the three parsed-document artifacts. The journal
//! artifact is added by SPEC-0057 T-003.

use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;
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
/// document artifacts (SPEC.md / TASKS.md / REPORT.md).
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
}

/// Run the name-scoped balance pass over one artifact's raw source and
/// emit one `XML-001` per orphan foreign tag (SPEC DEC-002).
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
            // definition (REQ-002).
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
/// by the tag name (REQ-001): the open-orphan and close-orphan cases share
/// this wording and differ solely in the substituted name and line. The
/// path and 1-indexed line are carried by the diagnostic location and
/// surfaced by the renderer (REQ-005).
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
