//! REQ-* rules: requirement-to-scenario coverage graph.
//!
//! After SPEC-0020 the requirement-to-scenario graph is carried by the
//! SPEC.md raw XML element tree (scenarios are nested inside their
//! parent `<requirement>` element). The element parser already rejects
//! orphan scenarios at parse time, so REQ-002 and REQ-003 (which
//! guarded against dangling references in the old TOML graph) are no
//! longer reachable. Only REQ-001 (requirements with zero covering
//! scenarios) remains as an element-tree-derived diagnostic.

use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;

const REQ_001: &str = "REQ-001";

/// Append every REQ-* diagnostic for one spec.
pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    let Some(spec_doc) = spec.spec_doc_ok() else {
        return;
    };

    for requirement in &spec_doc.requirements {
        if requirement.scenarios.is_empty() {
            out.push(Diagnostic::with_file(
                REQ_001,
                Level::Error,
                spec.spec_id.clone(),
                spec.spec_md_path.clone(),
                format!(
                    "`{id}` has no covering scenarios; every requirement must declare at least one nested `<scenario>` element",
                    id = requirement.id,
                ),
            ));
        }
    }
}
