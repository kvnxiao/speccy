//! `speccy review` command logic.
//!
//! Renders the Phase 4 reviewer prompt for one persona on one task. The
//! CLI never invokes a model: it locates the task across the workspace
//! via [`speccy_core::task_lookup::find`], resolves the persona content
//! via [`speccy_core::personas::resolve_file`] (project-local override
//! before embedded bundle), captures the implementer diff via
//! [`crate::git::diff_for_review`], inlines all of that plus AGENTS.md
//! into the embedded `reviewer-<persona>.md` template, applies budget
//! trimming, and writes the rendered prompt to stdout.
//!
//! See `.speccy/specs/0009-review-command/SPEC.md`.

use crate::git::diff_for_review;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::personas::ALL as PERSONAS_ALL;
use speccy_core::personas::PersonaError;
use speccy_core::personas::resolve_file as resolve_persona_file;
use speccy_core::prompt::DEFAULT_BUDGET;
use speccy_core::prompt::PromptError;
use speccy_core::prompt::TrimResult;
use speccy_core::prompt::load_agents_md;
use speccy_core::prompt::load_template;
use speccy_core::prompt::render;
use speccy_core::prompt::slice_for_task;
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

    let agents = load_agents_md(&project_root);
    let persona_content = resolve_persona_file(&args.persona, &project_root)?;
    let template_name = format!("reviewer-{}.md", args.persona);
    let template = load_template(&template_name)?;

    let diff = diff_for_review(&project_root);

    // After SPEC-0019 REQ-005, prompt slicing reads `SpecDoc` and emits
    // only the requirements this task covers (plus frontmatter, summary,
    // and decision context). Falls back to the raw SPEC.md when the
    // marker tree failed to parse.
    let spec_md_slice = location.spec_doc.map_or_else(
        || location.spec_md.raw.clone(),
        |doc| slice_for_task(doc, &location.task.covers),
    );
    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("spec_id", location.spec_id.clone());
    vars.insert("spec_md", spec_md_slice);
    vars.insert("task_id", location.task.id.clone());
    vars.insert("task_entry", location.task_entry_raw.clone());
    vars.insert("diff", diff);
    vars.insert("persona", args.persona.clone());
    vars.insert("persona_content", persona_content);
    vars.insert("agents", agents);

    let rendered = render(template, &vars);
    let TrimResult { output, .. } = trim_to_budget(rendered, DEFAULT_BUDGET);
    out.write_all(output.as_bytes())?;
    if !output.ends_with('\n') {
        out.write_all(b"\n")?;
    }
    Ok(())
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
