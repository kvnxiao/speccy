//! Shared task-context resolution walk.
//!
//! Hosts the single `covers → requirements → scenarios` resolution used
//! by both `speccy check` and `speccy context`. Extracting it here (per
//! SPEC-0056 REQ-003 / DEC-002) keeps the two commands from drifting:
//! they resolve a task's covering requirements through one function
//! rather than each maintaining its own copy of the walk.
//!
//! See `.speccy/specs/0056-task-context-bundle/SPEC.md`.

use crate::parse::Requirement;
use crate::parse::SpecDoc;
use crate::parse::task_xml::Task;

/// Resolve the requirements a task covers, deduplicated, in the order the
/// task's `covers` attribute first names them.
///
/// For each id in `task.covers`, the matching `SpecDoc.requirements`
/// entry (matched by `Requirement::id`) is collected once. Ordering
/// follows the `covers` list — the same traversal order the walk
/// previously inlined in `check::run_task` used — so a
/// `covers="REQ-003 REQ-001"` resolves to `[REQ-003, REQ-001]`. Each
/// returned [`Requirement`] carries its own `scenarios` field unchanged,
/// so callers reach scenarios through the requirement they belong to.
///
/// Semantics preserved verbatim from the previously-inlined walk:
///
/// - An empty `covers` yields an empty result.
/// - A `covers` token whose id matches no `Requirement::id` is silently skipped
///   at this layer — the lint engine's `TSK-001` owns surfacing that absence,
///   so resolution stays infallible.
/// - A requirement named more than once in `covers` appears once, at its first
///   occurrence.
#[must_use = "the resolved requirements are the bundle/check payload"]
pub fn resolve_covering_requirements<'doc>(
    task: &Task,
    spec_doc: &'doc SpecDoc,
) -> Vec<&'doc Requirement> {
    let mut collected: Vec<&Requirement> = Vec::new();
    let mut seen_ids: Vec<&str> = Vec::new();
    for req_id in &task.covers {
        let Some(req) = spec_doc.requirements.iter().find(|r| &r.id == req_id) else {
            continue;
        };
        if seen_ids.contains(&req.id.as_str()) {
            continue;
        }
        seen_ids.push(req.id.as_str());
        collected.push(req);
    }
    collected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::ElementSpan;
    use crate::parse::TaskState;
    use crate::parse::parse_spec_xml;
    use crate::parse::parse_task_xml;
    use camino::Utf8Path;

    /// Render one `<requirement>` block with a single scenario.
    fn req_block(n: u32) -> String {
        format!(
            "<requirement id=\"REQ-{n:03}\">\n\
             Requirement {n} body.\n\
             \n\
             <done-when>\n- placeholder.\n</done-when>\n\
             \n\
             <behavior>\n- placeholder.\n</behavior>\n\
             \n\
             <scenario id=\"CHK-{n:03}\">\n\
             Given req {n}, when X, then Y.\n\
             </scenario>\n\
             </requirement>\n\n"
        )
    }

    /// Build a five-requirement spec, each with one scenario.
    fn five_req_spec() -> SpecDoc {
        let mut src = String::from(
            "---\n\
             id: SPEC-0042\n\
             slug: example\n\
             title: Example\n\
             status: in-progress\n\
             created: 2026-06-10\n\
             ---\n\n\
             # Example\n\n\
             <goals>\nGoals body.\n</goals>\n\n\
             <non-goals>\nNon-goals body.\n</non-goals>\n\n\
             <user-stories>\n- A story.\n</user-stories>\n\n",
        );
        for n in 1..=5 {
            src.push_str(&req_block(n));
        }
        src.push_str("<changelog>\n| Date | Author | Summary |\n</changelog>\n");
        parse_spec_xml(&src, Utf8Path::new("SPEC.md"))
            .expect("five-requirement fixture spec parses")
    }

    /// Parse a single `<task>` covering the given requirement ids.
    fn task_covering(covers: &str) -> Task {
        let src = format!(
            r#"---
spec: SPEC-0042
---
# Tasks

<task id="T-001" state="pending" covers="{covers}">
## A task

<task-scenarios>
Given x, when y, then z.
</task-scenarios>
</task>
"#
        );
        let doc = parse_task_xml(&src, Utf8Path::new("TASKS.md")).expect("task fixture parses");
        doc.tasks
            .into_iter()
            .next()
            .expect("fixture declares one task")
    }

    #[test]
    fn resolves_exactly_covered_requirements_in_covers_order() {
        let spec = five_req_spec();
        // covers names them out of declared order; result follows the
        // covers list's order, matching the previously-inlined walk.
        let task = task_covering("REQ-003 REQ-001");
        let resolved = resolve_covering_requirements(&task, &spec);

        let ids: Vec<&str> = resolved.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            ["REQ-003", "REQ-001"],
            "covered requirements resolve in covers-list order"
        );

        // The other three never appear.
        for absent in ["REQ-002", "REQ-004", "REQ-005"] {
            assert!(
                !ids.contains(&absent),
                "uncovered {absent} must be absent from the resolution"
            );
        }

        // Each carries its own scenarios through, in the same order.
        let chk_ids: Vec<&str> = resolved
            .iter()
            .flat_map(|r| r.scenarios.iter().map(|s| s.id.as_str()))
            .collect();
        assert_eq!(
            chk_ids,
            ["CHK-003", "CHK-001"],
            "each resolved requirement carries its own scenarios"
        );
    }

    #[test]
    fn empty_covers_yields_empty_set() {
        let spec = five_req_spec();
        // The TASKS.md parser forbids an empty `covers=""` attribute, so
        // an empty covers vec is only reachable by constructing the Task
        // directly. The resolver must still treat it as covering nothing,
        // matching the defensive empty-covers branch in `check::run_task`.
        let empty_span = ElementSpan { start: 0, end: 0 };
        let task = Task {
            id: "T-001".to_owned(),
            state: TaskState::Pending,
            covers: Vec::new(),
            scenarios_body: String::new(),
            scenarios_span: empty_span,
            body: String::new(),
            span: empty_span,
        };
        let resolved = resolve_covering_requirements(&task, &spec);
        assert!(
            resolved.is_empty(),
            "empty covers must resolve to no requirements"
        );
    }

    #[test]
    fn missing_requirement_id_is_silently_skipped() {
        let spec = five_req_spec();
        // REQ-999 is absent from the spec; REQ-002 is present.
        let task = task_covering("REQ-999 REQ-002");
        let resolved = resolve_covering_requirements(&task, &spec);
        let ids: Vec<&str> = resolved.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            ["REQ-002"],
            "an unmatched covers token is skipped without error, \
             leaving the matched requirement"
        );
    }

    #[test]
    fn duplicate_covers_token_resolves_once_at_first_occurrence() {
        let spec = five_req_spec();
        let task = task_covering("REQ-002 REQ-001 REQ-002");
        let resolved = resolve_covering_requirements(&task, &spec);
        let ids: Vec<&str> = resolved.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            ["REQ-002", "REQ-001"],
            "a requirement named twice in covers appears once, at first occurrence"
        );
    }
}
