//! SPC-* rules: structural / frontmatter consistency.

use crate::error::ParseError;
use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;
use crate::lint::types::Workspace;
use crate::parse::TaskState;
use crate::parse::cross_ref::cross_ref;

const SPC_001: &str = "SPC-001";
const SPC_002: &str = "SPC-002";
const SPC_003: &str = "SPC-003";
const SPC_004: &str = "SPC-004";
const SPC_005: &str = "SPC-005";
const SPC_006: &str = "SPC-006";
const SPC_007: &str = "SPC-007";

const REQUIRED_FRONTMATTER_FIELDS: &[&str] = &["id", "slug", "title", "status", "created"];

/// Append every SPC-* diagnostic for one spec.
pub fn lint(spec: &ParsedSpec, workspace: &Workspace<'_>, out: &mut Vec<Diagnostic>) {
    spc_001_spec_doc_parse(spec, out);
    let (spc_004_fired, spc_005_fired) = spc_004_005_spec_md_parse(spec, out);

    if !spc_004_fired && !spc_005_fired {
        spc_002_003_cross_ref(spec, out);
    }

    spc_006_supersession(spec, workspace, out);
    spc_007_implemented_open_tasks(spec, out);
}

fn spc_001_spec_doc_parse(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    if let Err(err) = &spec.spec_doc {
        let message = match err {
            ParseError::StraySpecToml { path } => format!(
                "stray per-spec spec.toml at {path}: SPEC-0019 removed spec.toml; remove the file and rely on SPEC.md elements"
            ),
            ParseError::Io { source, .. } => {
                format!("SPEC.md element tree could not be read: {source}")
            }
            other => format!("SPEC.md element tree is invalid: {other}"),
        };
        out.push(Diagnostic::with_file(
            SPC_001,
            Level::Error,
            spec.spec_id.clone(),
            spec.spec_md_path.clone(),
            message,
        ));
    }
}

/// Returns `(spc_004_fired, spc_005_fired)` so callers can suppress
/// downstream rules that depend on a successfully parsed SPEC.md.
fn spc_004_005_spec_md_parse(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) -> (bool, bool) {
    let mut spc_004_fired = false;
    let mut spc_005_fired = false;
    if let Err(err) = &spec.spec_md {
        match err {
            ParseError::MissingField { field, .. } if field == "frontmatter" => {
                out.push(Diagnostic::with_file(
                    SPC_004,
                    Level::Error,
                    spec.spec_id.clone(),
                    spec.spec_md_path.clone(),
                    "SPEC.md has no YAML frontmatter; required fields are: id, slug, title, status, created"
                        .to_owned(),
                ));
                spc_004_fired = true;
            }
            ParseError::Yaml { message, .. } => {
                let missing: Vec<&&str> = REQUIRED_FRONTMATTER_FIELDS
                    .iter()
                    .filter(|f| message.contains(**f))
                    .collect();
                if missing.is_empty() {
                    out.push(Diagnostic::with_file(
                        SPC_004,
                        Level::Error,
                        spec.spec_id.clone(),
                        spec.spec_md_path.clone(),
                        format!("SPEC.md frontmatter failed to parse: {message}"),
                    ));
                } else {
                    for field in missing {
                        out.push(Diagnostic::with_file(
                            SPC_004,
                            Level::Error,
                            spec.spec_id.clone(),
                            spec.spec_md_path.clone(),
                            format!("SPEC.md frontmatter is missing required field `{field}`"),
                        ));
                    }
                }
                spc_004_fired = true;
            }
            ParseError::InvalidEnumValue {
                field,
                value,
                allowed,
                ..
            } if field == "status" => {
                out.push(Diagnostic::with_file(
                    SPC_005,
                    Level::Error,
                    spec.spec_id.clone(),
                    spec.spec_md_path.clone(),
                    format!("SPEC.md frontmatter `status: {value}` is not one of {{{allowed}}}"),
                ));
                spc_005_fired = true;
            }
            other => {
                out.push(Diagnostic::with_file(
                    SPC_004,
                    Level::Error,
                    spec.spec_id.clone(),
                    spec.spec_md_path.clone(),
                    format!("SPEC.md could not be parsed: {other}"),
                ));
                spc_004_fired = true;
            }
        }
    }
    (spc_004_fired, spc_005_fired)
}

fn spc_002_003_cross_ref(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    let (Some(spec_md), Some(spec_doc)) = (spec.spec_md_ok(), spec.spec_doc_ok()) else {
        return;
    };

    let diff = cross_ref(spec_md, spec_doc);

    for id in &diff.only_in_spec_md {
        out.push(Diagnostic::with_file(
            SPC_002,
            Level::Error,
            spec.spec_id.clone(),
            spec.spec_md_path.clone(),
            format!(
                "SPEC.md heading declares `{id}` but no matching `<requirement>` element exists"
            ),
        ));
    }

    for id in &diff.only_in_markers {
        out.push(Diagnostic::with_file(
            SPC_003,
            Level::Error,
            spec.spec_id.clone(),
            spec.spec_md_path.clone(),
            format!(
                "`<requirement id=\"{id}\">` element exists but SPEC.md has no matching `### REQ-NNN` heading"
            ),
        ));
    }
}

fn spc_006_supersession(spec: &ParsedSpec, workspace: &Workspace<'_>, out: &mut Vec<Diagnostic>) {
    let Some(spec_md) = spec.spec_md_ok() else {
        return;
    };
    if !matches!(
        spec_md.frontmatter.status,
        crate::parse::SpecStatus::Superseded
    ) {
        return;
    }
    let id = &spec_md.frontmatter.id;
    if workspace.supersession.superseded_by(id).is_empty() {
        out.push(Diagnostic::with_file(
            SPC_006,
            Level::Error,
            spec.spec_id.clone(),
            spec.spec_md_path.clone(),
            format!(
                "{id} has status `superseded` but no other spec in the workspace declares `supersedes: [{id}]`"
            ),
        ));
    }
}

fn spc_007_implemented_open_tasks(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    let Some(spec_md) = spec.spec_md_ok() else {
        return;
    };
    if !matches!(
        spec_md.frontmatter.status,
        crate::parse::SpecStatus::Implemented
    ) {
        return;
    }
    let Some(tasks_md) = spec.tasks_md_ok() else {
        return;
    };
    let open_count = tasks_md
        .tasks
        .iter()
        .filter(|t| !matches!(t.state, TaskState::Done))
        .count();
    if open_count > 0 {
        out.push(Diagnostic::with_file(
            SPC_007,
            Level::Info,
            spec.spec_id.clone(),
            spec.spec_md_path.clone(),
            format!(
                "{id} has status `implemented` but {open_count} task(s) are not `[x]`",
                id = spec_md.frontmatter.id,
            ),
        ));
    }
}
