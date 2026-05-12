//! REQ-* rules: requirement-to-check coverage graph.

use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;
use std::collections::HashSet;

const REQ_001: &str = "REQ-001";
const REQ_002: &str = "REQ-002";

/// Append every REQ-* diagnostic for one spec.
pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    let Some(spec_toml) = spec.spec_toml_ok() else {
        return;
    };

    let check_ids: HashSet<&str> = spec_toml.checks.iter().map(|c| c.id.as_str()).collect();

    for requirement in &spec_toml.requirements {
        if requirement.checks.is_empty() {
            out.push(Diagnostic::with_file(
                REQ_001,
                Level::Error,
                spec.spec_id.clone(),
                spec.spec_toml_path.clone(),
                format!(
                    "`{id}` has no covering checks; every requirement must declare at least one CHK-NNN",
                    id = requirement.id,
                ),
            ));
            continue;
        }

        for referenced in &requirement.checks {
            if !check_ids.contains(referenced.as_str()) {
                out.push(Diagnostic::with_file(
                    REQ_002,
                    Level::Error,
                    spec.spec_id.clone(),
                    spec.spec_toml_path.clone(),
                    format!(
                        "`{req}` references `{chk}` but no `[[checks]] id = \"{chk}\"` entry exists",
                        req = requirement.id,
                        chk = referenced,
                    ),
                ));
            }
        }
    }
}
