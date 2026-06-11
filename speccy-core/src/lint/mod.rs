//! Pure lint engine.
//!
//! Takes parsed artifacts from [`crate::parse`] plus a workspace-wide
//! supersession index and emits structured [`Diagnostic`] values with
//! stable codes (`SPC-*`, `REQ-*`, `VAL-*`, `TSK-*`, `QST-*`, `RPT-*`,
//! `JNL-*`, `VET-*`, `XML-*`).
//!
//! All semantic judgement of quality stays in review. Lint catches only
//! mechanical inconsistencies. See
//! `.speccy/specs/0003-lint-engine/SPEC.md` for the complete contract.

pub mod registry;
pub mod rules;
pub mod types;

pub use registry::REGISTRY;
pub use types::Diagnostic;
pub use types::Level;
pub use types::ParsedSpec;
pub use types::Workspace;

/// Run every lint rule against `workspace` and return diagnostics sorted
/// deterministically.
///
/// The ordering key is `(spec_id, code, file, line)` ascending, with
/// `None` sorting before `Some` to keep workspace-level diagnostics
/// (none of those fields) above per-spec diagnostics.
#[must_use = "the returned diagnostics are the entire output of the lint engine"]
pub fn run(workspace: &Workspace<'_>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for spec in workspace.specs {
        rules::spc::lint(spec, workspace, &mut diagnostics);
        rules::req::lint(spec, &mut diagnostics);
        rules::tsk::lint(spec, &mut diagnostics);
        rules::qst::lint(spec, &mut diagnostics);
        rules::rpt::lint(spec, &mut diagnostics);
        // SPEC-0037 REQ-002: JNL-* validates per-task journal files
        // gated by task state. The activation gate (skip when
        // state is `in-progress` or `in-review`) lives inside the
        // rule itself; per-task state is read from the parsed
        // TasksDoc.
        rules::jnl::lint(spec, &mut diagnostics);
        // SPEC-0055 REQ-007: VET-* validates `journal/VET.md` against the
        // frozen vet grammar. The lints run only when the file exists;
        // the existence gate lives inside the rule.
        rules::vet::lint(spec, &mut diagnostics);
        // SPEC-0057: XML-* flags unbalanced foreign (non-whitelisted) XML
        // tags leaked into the parsed SPEC.md / TASKS.md / REPORT.md
        // artifacts. Balance is name-scoped and fence-aware; the rule reads
        // the raw source already retained on each parsed document.
        rules::xml::lint(spec, &mut diagnostics);
    }

    diagnostics.sort_by(|a, b| {
        a.spec_id
            .cmp(&b.spec_id)
            .then_with(|| a.code.cmp(b.code))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::Workspace;
    use super::run;
    use crate::parse::supersession::SupersessionIndex;
    use crate::parse::supersession::supersession_index;

    #[test]
    fn empty_workspace_yields_no_diagnostics() {
        let index: SupersessionIndex = supersession_index(&[]);
        let workspace = Workspace {
            specs: &[],
            supersession: &index,
        };
        let diags = run(&workspace);
        assert!(diags.is_empty());
    }

    #[test]
    fn run_is_deterministic_on_empty_input() {
        let index: SupersessionIndex = supersession_index(&[]);
        let workspace = Workspace {
            specs: &[],
            supersession: &index,
        };
        let a = run(&workspace);
        let b = run(&workspace);
        assert_eq!(a, b);
    }
}
