//! `speccy review` command logic.
//!
//! Renders the Phase 4 reviewer prompt for one persona on one task. The
//! CLI never invokes a model: it locates the task across the workspace
//! via [`speccy_core::task_lookup::find`], resolves the persona content
//! via [`speccy_core::personas::resolve_file`] (project-local override
//! before embedded bundle), inlines that into the embedded
//! `reviewer-<persona>.md` template, applies budget trimming, and writes
//! the rendered prompt to stdout. SPEC-0023 REQ-003 moved diff fetching
//! out of the CLI: the rendered prompt instructs the reviewer agent to
//! run `git diff` itself, scoped to the task's suggested files.
//! SPEC-0023 REQ-005 retired the inlined-`AGENTS.md` flow and REQ-006
//! retired the inlined-`SPEC.md` flow: modern AI coding harnesses
//! auto-load `AGENTS.md` themselves, and every harness ships a Read
//! primitive the reviewer uses to fetch SPEC.md by path on demand. The
//! rendered prompt names the SPEC.md repo-relative path; the body is
//! no longer inlined.
//!
//! See `.speccy/specs/0009-review-command/SPEC.md` and
//! `.speccy/specs/0023-single-phase-skill-primitives/SPEC.md`.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::personas::ALL as PERSONAS_ALL;
use speccy_core::personas::PersonaError;
use speccy_core::personas::resolve_file as resolve_persona_file;
use speccy_core::prompt::DEFAULT_BUDGET;
use speccy_core::prompt::PromptError;
use speccy_core::prompt::TrimResult;
use speccy_core::prompt::load_template;
use speccy_core::prompt::render;
use speccy_core::prompt::trim_to_budget;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskRef;
use speccy_core::task_lookup::find;
use speccy_core::task_lookup::parse_ref;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan;
use std::collections::BTreeMap;
use std::io::Write;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReviewError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// Argument parsing or workspace lookup failed.
    #[error(transparent)]
    Lookup(#[from] LookupError),
    /// Persona registry rejected the `--persona` value or the bundle is
    /// missing the persona file.
    #[error(transparent)]
    Persona(#[from] PersonaError),
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

/// `speccy review` arguments.
#[derive(Debug, Clone)]
pub struct ReviewArgs {
    /// The `T-NNN` or `SPEC-NNNN/T-NNN` reference (required).
    pub task_ref: String,
    /// The reviewer persona name; must be in
    /// [`speccy_core::personas::ALL`].
    pub persona: String,
}

/// Run `speccy review` from `cwd`, writing the rendered prompt to
/// `out`.
///
/// # Errors
///
/// Returns any [`ReviewError`] variant if discovery, lookup, persona
/// resolution, template loading, rendering, or I/O fails.
pub fn run(args: &ReviewArgs, cwd: &Utf8Path, out: &mut dyn Write) -> Result<(), ReviewError> {
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => {
            return Err(ReviewError::ProjectRootNotFound);
        }
        Err(other) => return Err(ReviewError::Workspace(other)),
    };

    if !PERSONAS_ALL.contains(&args.persona.as_str()) {
        return Err(ReviewError::Persona(PersonaError::UnknownName {
            name: args.persona.clone(),
            valid: PERSONAS_ALL,
        }));
    }

    let task_ref: TaskRef = parse_ref(&args.task_ref)?;
    let workspace = scan(&project_root);
    let location = find(&workspace, &task_ref)?;

    let persona_content = resolve_persona_file(&args.persona, &project_root)?;
    let template_name = format!("reviewer-{}.md", args.persona);
    let template = load_template(&template_name)?;

    // SPEC-0023 REQ-006: SPEC.md is no longer inlined. The rendered
    // prompt names the repo-relative path; the agent reads the file via
    // the host's Read primitive on demand.
    let spec_md_path = spec_md_path_relative(&project_root, location.spec_dir);
    // SPEC-0023 REQ-003: the rendered prompt no longer inlines the
    // branch diff. The reviewer agent fetches it via `git diff` itself,
    // scoped to the task's suggested files.
    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("spec_id", location.spec_id.clone());
    vars.insert("spec_md_path", spec_md_path);
    vars.insert("task_id", location.task.id.clone());
    vars.insert("task_entry", location.task_entry_raw.clone());
    vars.insert("persona", args.persona.clone());
    vars.insert("persona_content", persona_content);

    let rendered = render(template, &vars);
    let TrimResult { output, .. } = trim_to_budget(rendered, DEFAULT_BUDGET);
    out.write_all(output.as_bytes())?;
    if !output.ends_with('\n') {
        out.write_all(b"\n")?;
    }
    Ok(())
}

/// Compute the repo-relative path to `<spec_dir>/SPEC.md`.
///
/// `project_root` is the absolute path to the project root (where
/// `.speccy/` lives); `spec_dir` is the absolute path to the spec
/// directory. Returns a forward-slash path string suitable for embedding
/// in the rendered prompt; falls back to the absolute spec path string
/// if the relative computation fails (which would only happen if
/// `spec_dir` were not under `project_root`, a configuration the
/// workspace scanner does not produce).
fn spec_md_path_relative(project_root: &Utf8Path, spec_dir: &Utf8Path) -> String {
    let relative = spec_dir.strip_prefix(project_root).unwrap_or(spec_dir);
    relative.join("SPEC.md").as_str().replace('\\', "/")
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`ReviewError::Cwd`] if `std::env::current_dir` fails, or
/// [`ReviewError::CwdNotUtf8`] if the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, ReviewError> {
    let std_path = std::env::current_dir().map_err(ReviewError::Cwd)?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| ReviewError::CwdNotUtf8)
}
