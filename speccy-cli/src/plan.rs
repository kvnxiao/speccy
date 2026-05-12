//! `speccy plan` command logic.
//!
//! Renders the Phase 1 prompt that an agent reads to author or amend a
//! SPEC.md + spec.toml pair. The CLI never invokes a model; it loads
//! VISION.md / AGENTS.md / SPEC.md context, substitutes placeholders
//! into an embedded markdown template, trims to the budget, and writes
//! the rendered prompt to stdout.
//!
//! Two forms:
//!
//! - `speccy plan` (no arg) -- greenfield. Reads `.speccy/VISION.md`, allocates
//!   the next available `SPEC-NNNN` ID, renders `plan-greenfield.md`.
//! - `speccy plan SPEC-NNNN` -- amendment. Reads the named SPEC.md, renders
//!   `plan-amend.md` (the agent is asked for a minimal surgical edit, not a
//!   rewrite).
//!
//! See `.speccy/specs/0005-plan-command/SPEC.md`.

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
use speccy_core::prompt::load_agents_md;
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
    /// `.speccy/VISION.md` was not present.
    #[error(".speccy/VISION.md not found at {path}; run `speccy init` first, or create the file")]
    VisionMissing {
        /// Expected path of VISION.md.
        path: Utf8PathBuf,
    },
    /// `.speccy/VISION.md` could not be read for non-NotFound reasons.
    #[error("failed to read .speccy/VISION.md at {path}")]
    VisionIo {
        /// Path of VISION.md.
        path: Utf8PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
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
    let vision_path = project_root.join(".speccy").join("VISION.md");
    let vision = match fs_err::read_to_string(vision_path.as_std_path()) {
        Ok(v) => v,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(PlanError::VisionMissing { path: vision_path });
        }
        Err(source) => {
            return Err(PlanError::VisionIo {
                path: vision_path,
                source,
            });
        }
    };

    let agents = load_agents_md(project_root);
    let specs_dir = project_root.join(".speccy").join("specs");
    let next_id = allocate_next_spec_id(&specs_dir);

    let template = load_template("plan-greenfield.md")?;
    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("vision", vision);
    vars.insert("agents", agents);
    vars.insert("next_spec_id", next_id);
    Ok(render(template, &vars))
}

fn render_amendment(project_root: &Utf8Path, raw_id: &str) -> Result<String, PlanError> {
    let canonical_id = validate_spec_id(raw_id)?;
    let spec_dir = locate_spec_dir(project_root, &canonical_id)?;
    let spec_path = spec_dir.join("SPEC.md");
    let parsed = spec_md(&spec_path).map_err(|source| PlanError::Parse {
        id: canonical_id.clone(),
        source: Box::new(source),
    })?;

    let agents = load_agents_md(project_root);
    let changelog = format_changelog(&parsed);

    let template = load_template("plan-amend.md")?;
    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("spec_id", canonical_id);
    vars.insert("spec_md", parsed.raw);
    vars.insert("agents", agents);
    vars.insert("changelog", changelog);
    Ok(render(template, &vars))
}

fn validate_spec_id(raw: &str) -> Result<String, PlanError> {
    if !spec_id_regex().is_match(raw) {
        return Err(PlanError::InvalidSpecIdFormat {
            arg: raw.to_owned(),
        });
    }
    Ok(raw.to_owned())
}

fn locate_spec_dir(project_root: &Utf8Path, canonical_id: &str) -> Result<Utf8PathBuf, PlanError> {
    let digits =
        canonical_id
            .strip_prefix("SPEC-")
            .ok_or_else(|| PlanError::InvalidSpecIdFormat {
                arg: canonical_id.to_owned(),
            })?;
    let specs_dir = project_root.join(".speccy").join("specs");
    let read =
        fs_err::read_dir(specs_dir.as_std_path()).map_err(|_io| PlanError::SpecNotFound {
            id: canonical_id.to_owned(),
        })?;
    let prefix = format!("{digits}-");
    for entry in read.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_dir() {
            continue;
        }
        let Some(name) = entry
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .map(ToOwned::to_owned)
        else {
            continue;
        };
        if name.starts_with(&prefix) {
            let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
                continue;
            };
            return Ok(path);
        }
    }
    Err(PlanError::SpecNotFound {
        id: canonical_id.to_owned(),
    })
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
