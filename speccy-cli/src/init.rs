//! `speccy init` command logic.
//!
//! Scaffolds a `.speccy/` workspace and copies the host skill pack into
//! the host-native location. Host detection lives in [`crate::host`];
//! the embedded skill bundle lives in [`crate::embedded`]. This module
//! owns the planning, summary, and mutation steps.
//!
//! See `.speccy/specs/0002-init-command/SPEC.md`.
//! See `.speccy/specs/0033-eject-prompt-bodies/SPEC.md` (T-008: three-way
//! classification replacing Skip-on-exists).

use crate::host::Detected;
use crate::host::HostChoice;
use crate::host::detect_host;
use crate::render::RenderError;
use crate::render::render_host_pack;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::personas::ALL as PERSONAS_ALL;
use std::io::Write;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum InitError {
    /// One or more planned files exist on disk with content that differs
    /// from what `speccy init` would write, and `--force` was not passed.
    /// Carries the list of conflicting relative paths for stderr output.
    #[error(
        "speccy init: the following files exist and differ from the planned content:\n{}\nPass --force to overwrite them.",
        paths.join("\n")
    )]
    FilesConflict {
        /// Repo-relative paths of the conflicting files.
        paths: Vec<String>,
    },
    /// User supplied a `--host` value that isn't in
    /// [`crate::host::SUPPORTED_HOSTS`].
    #[error("unknown --host value `{name}`; supported: {}", supported.join(", "))]
    UnknownHost {
        /// Raw flag value the user supplied.
        name: String,
        /// Supported host names, for error messages.
        supported: &'static [&'static str],
    },
    /// Only `.cursor/` was detected; v1 ships no Cursor skill pack.
    #[error(
        ".cursor/ detected but speccy v1 ships no Cursor pack; pass --host claude-code or --host codex"
    )]
    CursorDetected,
    /// `MiniJinja` template render failed during host-pack materialisation.
    /// Wraps the [`RenderError`] from [`crate::render`] so the dispatcher
    /// can surface the failing template name without re-walking the
    /// bundle.
    #[error("template render failed")]
    Render(#[from] RenderError),
    /// Failure during workspace mutation (`fs_err::write`, `create_dir_all`).
    #[error("I/O error during init")]
    Io(#[from] std::io::Error),
}

/// `speccy init` arguments.
#[derive(Debug, Clone, Default)]
pub struct InitArgs {
    /// Optional `--host <name>` override.
    pub host: Option<String>,
    /// `--force`: overwrite shipped files in place when `.speccy/`
    /// already exists. User-authored files in the host skill directory
    /// (any name not in the shipped bundle) are still preserved.
    pub force: bool,
}

/// Three-way per-file classification.
///
/// 1. [`Action::Create`] — destination absent; file will be written.
/// 2. [`Action::Unchanged`] — destination exists and is byte-identical to the
///    planned content; no write occurs.
/// 3. [`Action::Conflict`] — destination exists and differs from planned
///    content. Without `--force`, the entire batch is refused atomically. Under
///    `--force`, the file is overwritten.
///
/// Host-native reviewer files (`.claude/agents/reviewer-<persona>.md`
/// and `.codex/agents/reviewer-<persona>.toml`) are user-customisable
/// and classified [`Action::Unchanged`] when they already exist
/// (regardless of
/// byte equality), so user edits to the persona body survive a re-init
/// or `--force` run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// Destination does not exist; file will be written fresh.
    Create,
    /// Destination exists and is byte-identical to the planned content
    /// (or is a user-tunable reviewer file that is Skip-on-exists).
    /// No write occurs; the file is logged as `unchanged`.
    Unchanged,
    /// Destination exists and differs from the planned content.
    /// Without `--force`: the entire batch is refused atomically.
    /// Under `--force`: the file is overwritten and logged as
    /// `(!) overwritten`.
    Conflict,
}

impl Action {
    /// Human-readable plan label used in the `speccy init plan:` summary.
    fn label(self, force: bool) -> &'static str {
        match self {
            Action::Create => "created",
            Action::Unchanged => "unchanged",
            Action::Conflict if force => "(!) overwritten",
            Action::Conflict => "conflict",
        }
    }
}

/// One planned write.
#[derive(Debug)]
struct PlanItem {
    /// Absolute destination path.
    destination: Utf8PathBuf,
    /// Bytes to be written if the action is `Create` or `Overwrite`.
    content: Vec<u8>,
    /// Decision taken at plan time.
    action: Action,
}

/// Run `speccy init` from `cwd`. Writes the plan summary and final
/// count line to `out`; warnings (e.g. fallback-host notice) go to
/// `err`.
///
/// # Errors
///
/// See [`InitError`] variants. CLI exit-code mapping lives in the
/// dispatcher (`main.rs`): user errors map to `1`, I/O failures to `2`.
pub fn run(
    args: InitArgs,
    cwd: &Utf8Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), InitError> {
    let InitArgs {
        host: host_flag,
        force,
    } = args;
    let project_root = cwd;

    let Detected { host, warning } = detect_host(host_flag.as_deref(), project_root)?;
    if let Some(w) = warning {
        writeln!(err, "speccy init: warning: {w}")?;
    }

    let plan = build_plan(project_root, host)?;

    // SPEC-0033 T-008: three-way classification.
    // Without --force, if any item is `Conflict`, refuse the entire batch.
    if !force {
        let conflicts: Vec<String> = plan
            .iter()
            .filter(|item| item.action == Action::Conflict)
            .map(|item| display_relative(&item.destination, project_root))
            .collect();
        if !conflicts.is_empty() {
            return Err(InitError::FilesConflict { paths: conflicts });
        }
    }

    print_plan(&plan, project_root, force, out)?;
    let outcome = execute_plan(&plan, force)?;
    writeln!(
        out,
        "Init complete: {created} created, {overwritten} overwritten, {unchanged} unchanged.",
        created = outcome.created,
        overwritten = outcome.overwritten,
        unchanged = outcome.unchanged,
    )?;

    Ok(())
}

#[derive(Debug, Default, Clone, Copy)]
struct Outcome {
    created: u32,
    overwritten: u32,
    unchanged: u32,
}

fn build_plan(project_root: &Utf8Path, host: HostChoice) -> Result<Vec<PlanItem>, InitError> {
    let mut plan: Vec<PlanItem> = Vec::new();

    // SPEC-0040 REQ-001: `speccy init` scaffolds an empty
    // `.speccy/.gitkeep` placeholder so `workspace::find_root` keeps
    // locating `.speccy/` between init and the first spec. The file
    // content is empty bytes — stable across runs so the three-way
    // classifier reports `Unchanged` on byte-identical re-runs.
    let gitkeep_path = project_root.join(".speccy").join(".gitkeep");
    let gitkeep_content: Vec<u8> = Vec::new();
    let gitkeep_action = classify_content(&gitkeep_path, &gitkeep_content);
    plan.push(PlanItem {
        destination: gitkeep_path,
        content: gitkeep_content,
        action: gitkeep_action,
    });

    append_host_pack_items(project_root, host, &mut plan)?;

    // `.speccy/skills/` is not written by `init`. The host-native
    // reviewer files under `.claude/agents/` and `.codex/agents/` are
    // the sole canonical persona surface and are classified
    // Skip-on-exists by `append_host_pack_items`.

    Ok(plan)
}

/// Materialise the host-templated wrapper pack via [`render_host_pack`]
/// and append one [`PlanItem`] per rendered file to `plan`.
///
/// Wrappers under `resources/agents/.<install_root>/` are rendered
/// through `MiniJinja` with the host's template context (see
/// [`HostChoice::template_context`]), `.tmpl` suffixes are stripped,
/// and the resulting `rel_path` is joined onto `project_root` to give
/// the absolute destination. The three-way classification applies:
/// absent → Create, byte-identical → Unchanged, differs → Conflict.
fn append_host_pack_items(
    project_root: &Utf8Path,
    host: HostChoice,
    plan: &mut Vec<PlanItem>,
) -> Result<(), InitError> {
    let rendered = render_host_pack(host)?;
    for file in rendered {
        let destination = project_root.join(&file.rel_path);
        let content = file.contents.into_bytes();
        let action = if is_host_native_reviewer_file(&file.rel_path) {
            // SPEC-0027 REQ-002: host-native reviewer files
            // (`.claude/agents/reviewer-<persona>.md` and
            // `.codex/agents/reviewer-<persona>.toml`) are the sole
            // canonical persona surface. Treat them as user-tunable:
            // create on absent, leave alone on exists (even under
            // `--force`) so local edits to persona focus survive.
            // In the three-way scheme this maps to: absent → Create,
            // exists (regardless of content) → Unchanged.
            if destination.exists() {
                Action::Unchanged
            } else {
                Action::Create
            }
        } else {
            classify_content(&destination, &content)
        };
        plan.push(PlanItem {
            destination,
            content,
            action,
        });
    }
    Ok(())
}

/// Return `true` iff `rel_path` is a host-native reviewer-persona
/// definition file shipped by `render_host_pack`. SPEC-0027 REQ-002
/// classifies these as Skip-on-exists so user edits to the persona
/// body (or the surrounding `name`/`description` frontmatter) survive
/// `speccy init --force`.
///
/// Matching is strict: only the six personas in
/// [`speccy_core::personas::ALL`] count, only at the exact two
/// per-host directories the renderer emits to, and only with the
/// host-specific file extension.
fn is_host_native_reviewer_file(rel_path: &Utf8Path) -> bool {
    let s = rel_path.as_str().replace('\\', "/");
    for persona in PERSONAS_ALL {
        if s == format!(".claude/agents/reviewer-{persona}.md") {
            return true;
        }
        if s == format!(".codex/agents/reviewer-{persona}.toml") {
            return true;
        }
    }
    false
}

/// Three-way file classification for SPEC-0033 T-008.
///
/// - Destination absent → [`Action::Create`].
/// - Destination exists, bytes match `planned` → [`Action::Unchanged`].
/// - Destination exists, bytes differ → [`Action::Conflict`].
fn classify_content(dest: &Utf8Path, planned: &[u8]) -> Action {
    match fs_err::read(dest.as_std_path()) {
        Ok(existing) => {
            if existing == planned {
                Action::Unchanged
            } else {
                Action::Conflict
            }
        }
        Err(_) => Action::Create,
    }
}

fn print_plan(
    plan: &[PlanItem],
    project_root: &Utf8Path,
    force: bool,
    out: &mut dyn Write,
) -> Result<(), InitError> {
    writeln!(out, "speccy init plan:")?;
    for item in plan {
        let rel = display_relative(&item.destination, project_root);
        let label = item.action.label(force);
        writeln!(out, "  {label:<16} {rel}")?;
    }
    Ok(())
}

fn display_relative(dest: &Utf8Path, project_root: &Utf8Path) -> String {
    dest.strip_prefix(project_root)
        .map_or_else(|_e| dest.to_string(), ToString::to_string)
}

fn execute_plan(plan: &[PlanItem], force: bool) -> Result<Outcome, InitError> {
    let mut outcome = Outcome::default();
    for item in plan {
        match item.action {
            Action::Create => {
                write_item(item)?;
                outcome.created = outcome.created.saturating_add(1);
            }
            Action::Unchanged => {
                // No write — byte-identical or reviewer Skip-on-exists.
                outcome.unchanged = outcome.unchanged.saturating_add(1);
            }
            Action::Conflict => {
                // Only reached when force == true (non-force conflicts
                // are caught before execute_plan is called).
                if force {
                    write_item(item)?;
                    outcome.overwritten = outcome.overwritten.saturating_add(1);
                }
            }
        }
    }
    Ok(outcome)
}

fn write_item(item: &PlanItem) -> Result<(), InitError> {
    if let Some(parent) = item.destination.parent() {
        fs_err::create_dir_all(parent.as_std_path())?;
    }
    fs_err::write(item.destination.as_std_path(), &item.content)?;
    Ok(())
}
