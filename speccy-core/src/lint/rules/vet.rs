//! VET-* rules: pre-ship vet journal (`journal/VET.md`) validation.
//!
//! SPEC-0055 REQ-007. The lints run **only** when
//! `<spec-dir>/journal/VET.md` exists — a spec without a VET.md emits no
//! `VET-*` diagnostics, because absence is the resolver's concern
//! (`speccy next`), not lint's. The two rules are:
//!
//! - `VET-001` (error): the file fails the frozen `vet_xml` grammar —
//!   missing/malformed frontmatter, a bad block shape, an attribute outside its
//!   domain, or an invalid per-section round sequence.
//! - `VET-002` (error): an invocation section's terminal-`<gate>` structure is
//!   violated — a section other than the last lacks a terminal `gate`, a `gate`
//!   is not the last block in its section, or a section holds more than one
//!   `gate`.
//!
//! The split is driven off the typed [`ParseError`] the strict
//! [`vet_xml::parse`] returns: a [`ParseError::VetGateStructure`] is the
//! gate-structure family (`VET-002`); every other parse failure is a
//! grammar failure (`VET-001`). Both codes are errors, since
//! `speccy next` gates shipping on this artifact.

use crate::error::ParseError;
use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;
use crate::parse::vet_xml::parse as parse_vet;

const VET_001: &str = "VET-001";
const VET_002: &str = "VET-002";

/// Append every VET-* diagnostic for one spec.
pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    let vet_path = spec.dir.join("journal").join("VET.md");
    // A spec without a VET.md emits nothing: absence is handled by
    // `speccy next`'s resolver, not by lint.
    if !vet_path.exists() {
        return;
    }

    let raw = match fs_err::read_to_string(vet_path.as_std_path()) {
        Ok(s) => s,
        Err(e) => {
            out.push(Diagnostic::with_file(
                VET_001,
                Level::Error,
                spec.spec_id.clone(),
                vet_path.clone(),
                format!("could not read vet journal `{vet_path}`: {e}"),
            ));
            return;
        }
    };

    match parse_vet(&raw, &vet_path) {
        Ok(_) => {}
        Err(err) => match err.as_ref() {
            ParseError::VetGateStructure { reason, .. } => {
                out.push(Diagnostic::with_file(
                    VET_002,
                    Level::Error,
                    spec.spec_id.clone(),
                    vet_path.clone(),
                    format!("`{vet_path}` violates the gate-structure rule: {reason}"),
                ));
            }
            other => {
                out.push(Diagnostic::with_file(
                    VET_001,
                    Level::Error,
                    spec.spec_id.clone(),
                    vet_path.clone(),
                    format!("`{vet_path}` fails the vet journal grammar: {other}"),
                ));
            }
        },
    }
}
