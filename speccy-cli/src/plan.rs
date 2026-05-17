//! `speccy plan` command logic.
//!
//! Renders the Phase 1 prompt that an agent reads to author or amend a
//! SPEC.md. The CLI never invokes a model; it substitutes placeholders
//! into an embedded markdown template, trims to the budget, and writes
//! the rendered prompt to stdout.
//!
//! Two forms:
//!
//! - `speccy plan` (no arg) -- greenfield. Allocates the next available
//!   `SPEC-NNNN` ID (walking nested mission folders under `.speccy/specs/`),
//!   renders `plan-greenfield.md`. No `VISION.md` is read: the noun has been
//!   retired.
//! - `speccy plan SPEC-NNNN` -- amendment. Locates the named spec directory
//!   (which may live flat under `.speccy/specs/NNNN-slug/` or grouped under
//!   `.speccy/specs/[focus]/NNNN-slug/`) and renders `plan-amend.md`. The
//!   rendered prompt names the SPEC.md repo-relative path plus a Read
//!   instruction for the nearest parent `MISSION.md`, when one exists.
//!
//! SPEC-0023 REQ-005 retired the inlined-`AGENTS.md` flow and REQ-006
//! retired the inlined-`SPEC.md` / `MISSION.md` flows: modern AI coding
//! harnesses auto-load `AGENTS.md` themselves, and every harness ships
//! a Read primitive the agent uses to fetch SPEC.md / MISSION.md by
//! path on demand. The rendered prompt names the file's repo-relative
//! path; the body is no longer inlined.
//!
//! See `.speccy/specs/0005-plan-command/SPEC.md` and
//! `.speccy/specs/0023-single-phase-skill-primitives/SPEC.md`.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use regex::Regex;
use speccy_core::ParseError;
use speccy_core::parse::SpecMd;
use speccy_core::parse::spec_md;
use speccy_core::prompt::DEFAULT_BUDGET;
use speccy_core::prompt::PromptError;
use speccy_core::prompt::TrimResult;
use speccy_core::prompt::allocate_next_spec_id;
use speccy_core::prompt::load_template;
use speccy_core::prompt::render;
use speccy_core::prompt::trim_to_budget;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write;
use std::sync::OnceLock;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PlanError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// Argument did not match the `SPEC-\d{4,}` shape.
    #[error("invalid SPEC-ID `{arg}`; expected format SPEC-NNNN (4+ digits)")]
    InvalidSpecIdFormat {
        /// The string supplied by the user.
        arg: String,
    },
    /// No spec directory matched the supplied ID.
    #[error("spec `{id}` not found under .speccy/specs/")]
    SpecNotFound {
        /// The canonical SPEC ID that was looked up.
        id: String,
    },
    /// Parsing the named SPEC.md failed. The underlying [`ParseError`]
    /// is boxed to keep [`PlanError`] within the `result_large_err`
    /// budget.
    #[error("failed to parse SPEC.md for {id}")]
    Parse {
        /// The SPEC ID we were trying to parse.
        id: String,
        /// Underlying parser error.
        #[source]
        source: Box<ParseError>,
    },
    /// Template lookup or substitution helper failed.
    #[error("prompt template error")]
    Prompt(#[from] PromptError),
    /// Working directory could not be resolved.
    #[error("failed to resolve current working directory")]
    Cwd(#[source] std::io::Error),
    /// Cwd path is not valid UTF-8.
    #[error("current working directory is not valid UTF-8")]
    CwdNotUtf8,
    /// I/O failure while writing the rendered prompt to stdout.
    #[error("failed to write rendered prompt to stdout")]
    Io(#[from] std::io::Error),
}

/// `speccy plan` arguments.
#[derive(Debug, Clone, Default)]
pub struct PlanArgs {
    /// `Some(id)` selects the amendment form; `None` selects greenfield.
    pub spec_id: Option<String>,
}

/// Run `speccy plan` from `cwd`, writing the rendered prompt to `out`.
///
/// # Errors
///
/// Returns any [`PlanError`] variant if discovery, parsing, template
/// loading, or rendering fails.
pub fn run(args: PlanArgs, cwd: &Utf8Path, out: &mut dyn Write) -> Result<(), PlanError> {
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => return Err(PlanError::ProjectRootNotFound),
        Err(other) => return Err(PlanError::Workspace(other)),
    };

    let rendered = match args.spec_id {
        None => render_greenfield(&project_root)?,
        Some(id) => render_amendment(&project_root, &id)?,
    };

    let TrimResult { output, .. } = trim_to_budget(rendered, DEFAULT_BUDGET);
    out.write_all(output.as_bytes())?;
    if !output.ends_with('\n') {
        out.write_all(b"\n")?;
    }
    Ok(())
}

fn render_greenfield(project_root: &Utf8Path) -> Result<String, PlanError> {
    let specs_dir = project_root.join(".speccy").join("specs");
    let next_id = allocate_next_spec_id(&specs_dir);

    let template = load_template("plan-greenfield.md")?;
    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("next_spec_id", next_id);
    Ok(render(template, &vars))
}

fn render_amendment(project_root: &Utf8Path, raw_id: &str) -> Result<String, PlanError> {
    let canonical_id = validate_spec_id(raw_id)?;
    let specs_root = project_root.join(".speccy").join("specs");
    let spec_dir = locate_spec_dir(&specs_root, &canonical_id)?;
    let spec_path = spec_dir.join("SPEC.md");
    let parsed = spec_md(&spec_path).map_err(|source| PlanError::Parse {
        id: canonical_id.clone(),
        source: Box::new(source),
    })?;

    // SPEC-0023 REQ-006: SPEC.md and MISSION.md are no longer inlined.
    // The rendered prompt names the SPEC.md repo-relative path and, when
    // the focus has a MISSION.md, a Read instruction for it. Flat
    // single-focus projects (no MISSION.md) emit an empty mission
    // section so the rendered prompt does not name a non-existent file.
    let spec_md_path = relative_path_string(project_root, &spec_path);
    let mission_section = mission_section(project_root, &spec_dir, &specs_root);
    let changelog = format_changelog(&parsed);

    let template = load_template("plan-amend.md")?;
    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("spec_id", canonical_id);
    vars.insert("spec_md_path", spec_md_path);
    vars.insert("mission_section", mission_section);
    vars.insert("changelog", changelog);
    Ok(render(template, &vars))
}

/// Compute the repo-relative path of `target` as a forward-slash string
/// suitable for embedding in rendered prompts. Falls back to the
/// absolute path string when `target` is not under `project_root` (a
/// configuration the workspace scanner does not produce).
fn relative_path_string(project_root: &Utf8Path, target: &Utf8Path) -> String {
    target
        .strip_prefix(project_root)
        .unwrap_or(target)
        .as_str()
        .replace('\\', "/")
}

/// Build the `## Mission context` section for the plan-amend prompt.
///
/// Returns the section heading plus a one-sentence Read instruction
/// naming the nearest enclosing `MISSION.md` when one exists. Returns an
/// empty string when no enclosing `MISSION.md` is found, so the rendered
/// prompt surfaces neither the heading nor a Read instruction for a
/// non-existent file (per SPEC-0023 REQ-006).
fn mission_section(project_root: &Utf8Path, spec_dir: &Utf8Path, specs_root: &Utf8Path) -> String {
    let Some(mission_path) = find_nearest_mission_md_path(spec_dir, specs_root) else {
        return String::new();
    };
    let rel = relative_path_string(project_root, &mission_path);
    format!(
        "## Mission context\n\n\
         Before editing, read the parent MISSION.md at `{rel}`. The CLI \
         no longer inlines the MISSION body into this prompt; load it via \
         your Read primitive.\n\n"
    )
}

/// Walk upward from `spec_dir` toward `specs_root` (inclusive) looking
/// for the nearest enclosing `MISSION.md`. Returns the absolute path
/// when found, else `None`.
fn find_nearest_mission_md_path(spec_dir: &Utf8Path, specs_root: &Utf8Path) -> Option<Utf8PathBuf> {
    let mut cursor = spec_dir.parent()?;
    loop {
        let candidate = cursor.join("MISSION.md");
        if fs_err::metadata(candidate.as_std_path()).is_ok_and(|m| m.is_file()) {
            return Some(candidate);
        }
        if cursor == specs_root {
            break;
        }
        cursor = cursor.parent()?;
    }
    None
}

fn validate_spec_id(raw: &str) -> Result<String, PlanError> {
    if !spec_id_regex().is_match(raw) {
        return Err(PlanError::InvalidSpecIdFormat {
            arg: raw.to_owned(),
        });
    }
    Ok(raw.to_owned())
}

/// Locate the directory holding the named spec. Searches `specs_root`
/// and any mission folders (subdirectories whose names do not match
/// `NNNN-slug`) one level deep. The first directory whose name starts
/// with `{digits}-` wins.
fn locate_spec_dir(specs_root: &Utf8Path, canonical_id: &str) -> Result<Utf8PathBuf, PlanError> {
    let digits =
        canonical_id
            .strip_prefix("SPEC-")
            .ok_or_else(|| PlanError::InvalidSpecIdFormat {
                arg: canonical_id.to_owned(),
            })?;
    let prefix = format!("{digits}-");
    find_spec_dir(specs_root, &prefix).ok_or_else(|| PlanError::SpecNotFound {
        id: canonical_id.to_owned(),
    })
}

/// Recursive search for a spec directory whose name starts with `prefix`.
/// Stops descending into a child once it matches the prefix (specs are
/// leaves). Mission folders (any other dir) are descended into.
fn find_spec_dir(dir: &Utf8Path, prefix: &str) -> Option<Utf8PathBuf> {
    let read = fs_err::read_dir(dir.as_std_path()).ok()?;
    for entry in read.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_dir() {
            continue;
        }
        let Some(name) = entry
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_owned)
        else {
            continue;
        };
        let Ok(child) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        if name.starts_with(prefix) {
            return Some(child);
        }
        if let Some(hit) = find_spec_dir(&child, prefix) {
            return Some(hit);
        }
    }
    None
}

fn format_changelog(spec: &SpecMd) -> String {
    if spec.changelog.is_empty() {
        return "_No Changelog rows yet._".to_owned();
    }
    let mut out = String::from("| Date | Author | Summary |\n|------|--------|---------|\n");
    for row in &spec.changelog {
        if writeln!(
            out,
            "| {date} | {author} | {summary} |",
            date = row.date,
            author = row.author,
            summary = row.summary,
        )
        .is_err()
        {
            // Writing to a String is infallible; this arm is unreachable
            // but absorbing the Result keeps `let_underscore_must_use`
            // happy without `.expect`.
            break;
        }
    }
    out
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn spec_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^SPEC-\d{4,}$").unwrap())
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`PlanError::Cwd`] if `std::env::current_dir` fails, or
/// [`PlanError::CwdNotUtf8`] if the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, PlanError> {
    let std_path = std::env::current_dir().map_err(PlanError::Cwd)?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| PlanError::CwdNotUtf8)
}

#[cfg(test)]
mod tests {
    use super::spec_id_regex;
    use super::validate_spec_id;

    #[test]
    fn valid_spec_ids_pass_regex() {
        assert!(spec_id_regex().is_match("SPEC-0001"));
        assert!(spec_id_regex().is_match("SPEC-9999"));
        assert!(spec_id_regex().is_match("SPEC-10000"));
    }

    #[test]
    fn invalid_spec_ids_rejected() {
        validate_spec_id("FOO").expect_err("`FOO` must fail format validation");
        validate_spec_id("SPEC-1").expect_err("`SPEC-1` has fewer than 4 digits");
        validate_spec_id("spec-0001").expect_err("lowercase prefix must be rejected");
        validate_spec_id("SPEC-").expect_err("missing digits must be rejected");
    }
}
