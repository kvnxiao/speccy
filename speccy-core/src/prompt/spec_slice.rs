//! Task-scoped slice of a marker-structured `SpecDoc` for prompt rendering.
//!
//! Before SPEC-0019 the implementer and reviewer prompts inlined the full
//! `SpecMd.raw` text. That spliced both unrelated requirements and the
//! per-spec `spec.toml` graph into every task's prompt. After SPEC-0019
//! the canonical contract lives in `SpecDoc`, and the prompt should
//! include only what the task covers.
//!
//! [`slice_for_task`] takes a parsed [`SpecDoc`] plus the REQ ids the
//! task's `Covers:` line declares, and returns a deterministic Markdown
//! string built from typed marker state:
//!
//! - YAML frontmatter (verbatim).
//! - The level-1 heading.
//! - The `speccy:summary` marker block when present.
//! - Each requirement covered by the task, rendered as a `speccy:requirement`
//!   marker block with its nested `speccy:scenario` marker blocks. Requirements
//!   outside the `covers` set are excluded.
//! - Every `speccy:decision` marker block (full Design context; scoping
//!   decisions to specific requirements isn't tractable from the typed model
//!   alone, and the per-spec decision list is small).
//!
//! Unknown REQ ids in `covers` are skipped silently — the lint engine
//! (TSK-001 against the SPEC.md heading set) is the right surface for
//! "this task lists a REQ that no longer exists".
//!
//! See `.speccy/specs/0019-xml-canonical-spec-md/SPEC.md` REQ-005.

use crate::parse::DecisionStatus;
use crate::parse::SpecDoc;

/// Render a task-scoped slice of `doc` for the requirements listed in
/// `covers`.
///
/// The output is deterministic and built only from typed model fields;
/// inter-marker free prose (Goals, Non-goals, narrative Design text) is
/// intentionally omitted because `SpecDoc` does not retain it. The
/// canonical structural contract — requirement bodies, nested scenarios,
/// decisions, and the summary — is what reviewers and implementers need
/// to act on the task.
///
/// `covers` is treated as an ordered set: requirements are emitted in
/// `covers` order, and a REQ id listed twice is rendered once.
#[must_use = "the rendered slice is the function's output"]
pub fn slice_for_task(doc: &SpecDoc, covers: &[String]) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&doc.frontmatter_raw);
    if !doc.frontmatter_raw.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("---\n\n");
    out.push_str("# ");
    out.push_str(&doc.heading);
    out.push('\n');

    if let Some(summary) = &doc.summary {
        out.push('\n');
        push_marker_block(&mut out, "summary", &[], summary);
    }

    let mut emitted: Vec<&str> = Vec::with_capacity(covers.len());
    for req_id in covers {
        if emitted.iter().any(|id| id == req_id) {
            continue;
        }
        let Some(req) = doc.requirements.iter().find(|r| &r.id == req_id) else {
            continue;
        };
        emitted.push(req_id.as_str());

        out.push('\n');
        let attrs = [("id", req.id.as_str())];
        push_marker_start(&mut out, "requirement", &attrs);
        let prose = strip_nested_scenario_blocks(&req.body);
        push_body(&mut out, &prose);
        for sc in &req.scenarios {
            let sc_attrs = [("id", sc.id.as_str())];
            push_marker_start(&mut out, "scenario", &sc_attrs);
            push_body(&mut out, &sc.body);
            push_marker_end(&mut out, "scenario");
        }
        push_marker_end(&mut out, "requirement");
    }

    for dec in &doc.decisions {
        out.push('\n');
        let status_str = dec.status.map(DecisionStatus::as_str);
        let mut attrs: Vec<(&str, &str)> = Vec::with_capacity(2);
        attrs.push(("id", dec.id.as_str()));
        if let Some(s) = status_str.as_ref() {
            attrs.push(("status", s));
        }
        push_marker_start(&mut out, "decision", &attrs);
        push_body(&mut out, &dec.body);
        push_marker_end(&mut out, "decision");
    }

    out
}

/// Remove nested `speccy:scenario` marker blocks from a requirement
/// body. The parser stores `Requirement.body` as the verbatim slice
/// between the requirement's start and end markers, which includes the
/// nested scenario markers as literal text. The slicer re-emits
/// scenarios from typed state, so the scenario marker lines must be
/// stripped from the surrounding prose first.
fn strip_nested_scenario_blocks(body: &str) -> String {
    let start_marker = "<!-- speccy:scenario";
    let end_marker = "<!-- /speccy:scenario";
    let mut out = String::with_capacity(body.len());
    let mut in_scenario = false;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if !in_scenario {
            if trimmed.starts_with(start_marker) {
                in_scenario = true;
                continue;
            }
            out.push_str(line);
        } else if trimmed.starts_with(end_marker) {
            in_scenario = false;
        }
    }
    out
}

fn push_marker_block(out: &mut String, name: &str, attrs: &[(&str, &str)], body: &str) {
    push_marker_start(out, name, attrs);
    push_body(out, body);
    push_marker_end(out, name);
}

fn push_marker_start(out: &mut String, name: &str, attrs: &[(&str, &str)]) {
    out.push_str("<!-- speccy:");
    out.push_str(name);
    for (k, v) in attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(v);
        out.push('"');
    }
    out.push_str(" -->\n");
}

fn push_marker_end(out: &mut String, name: &str) {
    out.push_str("<!-- /speccy:");
    out.push_str(name);
    out.push_str(" -->\n");
}

fn push_body(out: &mut String, body: &str) {
    let interior = trim_blank_boundary_lines(body);
    if interior.is_empty() {
        return;
    }
    out.push_str(interior);
    if !interior.ends_with('\n') {
        out.push('\n');
    }
}

fn trim_blank_boundary_lines(body: &str) -> &str {
    let bytes = body.as_bytes();
    let mut start: usize = 0;
    let mut cursor: usize = 0;
    while cursor < bytes.len() {
        let line_start = cursor;
        let mut all_ws = true;
        while cursor < bytes.len() && bytes.get(cursor) != Some(&b'\n') {
            match bytes.get(cursor) {
                Some(b' ' | b'\t' | b'\r') => {}
                _ => all_ws = false,
            }
            cursor = cursor.saturating_add(1);
        }
        if cursor < bytes.len() {
            cursor = cursor.saturating_add(1);
        }
        if all_ws {
            start = cursor;
        } else {
            start = line_start;
            break;
        }
    }
    if start >= bytes.len() {
        return "";
    }

    let mut end: usize = bytes.len();
    let mut cursor: usize = bytes.len();
    while cursor > start {
        let mut probe = cursor;
        let mut line_end = cursor;
        if probe > start && bytes.get(probe.saturating_sub(1)) == Some(&b'\n') {
            probe = probe.saturating_sub(1);
            line_end = probe;
        }
        let mut line_start = probe;
        while line_start > start && bytes.get(line_start.saturating_sub(1)) != Some(&b'\n') {
            line_start = line_start.saturating_sub(1);
        }
        let line = bytes.get(line_start..line_end).unwrap_or(&[]);
        let all_ws = line.iter().all(|b| matches!(b, b' ' | b'\t' | b'\r'));
        if all_ws {
            end = line_start;
            cursor = line_start;
        } else {
            break;
        }
    }
    bytes
        .get(start..end)
        .and_then(|s| std::str::from_utf8(s).ok())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::slice_for_task;
    use crate::parse::spec_markers::parse;
    use camino::Utf8Path;
    use indoc::indoc;

    fn fixture() -> &'static str {
        indoc! {r#"
            ---
            id: SPEC-0099
            slug: x
            title: Slice fixture
            status: in-progress
            created: 2026-05-15
            ---

            # SPEC-0099: Slice fixture

            <!-- speccy:summary -->
            Spec-level summary prose.
            <!-- /speccy:summary -->
            <!-- speccy:requirement id="REQ-001" -->
            ### REQ-001: First
            Body of REQ-001.
            <!-- speccy:scenario id="CHK-001" -->
            Scenario body for CHK-001.
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->
            <!-- speccy:requirement id="REQ-002" -->
            ### REQ-002: Second
            Body of REQ-002.
            <!-- speccy:scenario id="CHK-002" -->
            Scenario body for CHK-002.
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->
            <!-- speccy:requirement id="REQ-003" -->
            ### REQ-003: Third
            Body of REQ-003.
            <!-- speccy:scenario id="CHK-003" -->
            Scenario body for CHK-003.
            <!-- /speccy:scenario -->
            <!-- /speccy:requirement -->
            <!-- speccy:decision id="DEC-001" status="accepted" -->
            #### DEC-001
            Decision body.
            <!-- /speccy:decision -->
            <!-- speccy:changelog -->
            | Date | Author | Summary |
            |------|--------|---------|
            | 2026-05-15 | t | init |
            <!-- /speccy:changelog -->
        "#}
    }

    #[test]
    fn slice_includes_only_covered_requirements() {
        let doc = parse(fixture(), Utf8Path::new("SPEC.md")).expect("fixture must parse");
        let out = slice_for_task(&doc, &["REQ-002".to_owned()]);
        assert!(
            out.contains("speccy:requirement id=\"REQ-002\""),
            "covered REQ-002 marker must be present:\n{out}",
        );
        assert!(
            out.contains("Body of REQ-002."),
            "covered REQ-002 body must be present:\n{out}",
        );
        assert!(
            out.contains("Scenario body for CHK-002."),
            "covered REQ-002 scenario body must be present:\n{out}",
        );
        assert!(
            !out.contains("Body of REQ-001."),
            "uncovered REQ-001 body must be excluded:\n{out}",
        );
        assert!(
            !out.contains("Body of REQ-003."),
            "uncovered REQ-003 body must be excluded:\n{out}",
        );
        assert!(
            !out.contains("speccy:requirement id=\"REQ-001\""),
            "uncovered REQ-001 marker must be excluded:\n{out}",
        );
        assert!(
            !out.contains("speccy:requirement id=\"REQ-003\""),
            "uncovered REQ-003 marker must be excluded:\n{out}",
        );
    }

    #[test]
    fn slice_includes_frontmatter_heading_summary_and_decisions() {
        let doc = parse(fixture(), Utf8Path::new("SPEC.md")).expect("fixture must parse");
        let out = slice_for_task(&doc, &["REQ-001".to_owned()]);
        assert!(
            out.starts_with("---\n"),
            "must begin with frontmatter fence"
        );
        assert!(
            out.contains("id: SPEC-0099"),
            "frontmatter id must be preserved:\n{out}",
        );
        assert!(
            out.contains("# SPEC-0099: Slice fixture"),
            "level-1 heading must be preserved:\n{out}",
        );
        assert!(
            out.contains("Spec-level summary prose."),
            "summary body must be included:\n{out}",
        );
        assert!(
            out.contains("speccy:decision id=\"DEC-001\""),
            "decision marker must be included for context:\n{out}",
        );
        assert!(
            out.contains("Decision body."),
            "decision body must be included for context:\n{out}",
        );
    }

    #[test]
    fn slice_scenario_body_bytes_match_source() {
        let src = fixture();
        let doc = parse(src, Utf8Path::new("SPEC.md")).expect("fixture must parse");
        let out = slice_for_task(&doc, &["REQ-002".to_owned()]);
        // The scenario body literal as written in the source.
        let needle = "Scenario body for CHK-002.";
        assert!(
            out.contains(needle),
            "slice must preserve scenario body bytes from source:\nslice={out}",
        );
        assert!(
            src.contains(needle),
            "sanity: fixture itself must contain the scenario body bytes",
        );
    }

    #[test]
    fn slice_dedups_repeated_covers_and_skips_unknown() {
        let doc = parse(fixture(), Utf8Path::new("SPEC.md")).expect("fixture must parse");
        let out = slice_for_task(
            &doc,
            &[
                "REQ-002".to_owned(),
                "REQ-999".to_owned(),
                "REQ-002".to_owned(),
            ],
        );
        let count = out.matches("speccy:requirement id=\"REQ-002\"").count();
        assert_eq!(count, 1, "REQ-002 must render exactly once:\n{out}");
        assert!(
            !out.contains("REQ-999"),
            "unknown REQ id must be skipped silently:\n{out}",
        );
    }
}
