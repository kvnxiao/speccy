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
use crate::render::render_speccy_examples_pack;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::personas::ALL as PERSONAS_ALL;
use std::io::Write;
use thiserror::Error;

const SPECCY_TOML_TEMPLATE: &str = include_str!("templates/speccy.toml.tmpl");

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

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`InitError::Io`] if `std::env::current_dir` fails or the
/// path is not valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, InitError> {
    let std_path = std::env::current_dir()?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|path| {
        InitError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "current working directory is not valid UTF-8: {}",
                path.display()
            ),
        ))
    })
}

/// Three-way per-file classification used by the T-008 init redesign.
///
/// SPEC-0033 T-008 replaced the old binary Create/Overwrite/Skip scheme
/// with this three-way model:
///
/// 1. [`Action::Create`] — destination absent; file will be written.
/// 2. [`Action::Unchanged`] — destination exists and is byte-identical to the
///    planned content; no write occurs.
/// 3. [`Action::Conflict`] — destination exists and differs from planned
///    content. Without `--force`, the entire batch is refused atomically. Under
///    `--force`, the file is overwritten.
///
/// Host-native reviewer files (`.claude/agents/reviewer-<persona>.md`
/// and `.codex/agents/reviewer-<persona>.toml`) retain their
/// Skip-on-exists semantic from SPEC-0027 REQ-002: they are classified
/// [`Action::Unchanged`] when they already exist (regardless of
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

    let speccy_toml_path = project_root.join(".speccy").join("speccy.toml");
    let project_name = project_name_from(project_root);
    let speccy_toml_body = render_speccy_toml(&project_name);
    let content = speccy_toml_body.into_bytes();
    let speccy_toml_action = classify_content(&speccy_toml_path, &content);
    plan.push(PlanItem {
        destination: speccy_toml_path,
        content,
        action: speccy_toml_action,
    });

    append_host_pack_items(project_root, host, &mut plan)?;
    append_speccy_examples_items(project_root, &mut plan)?;

    // SPEC-0027 REQ-001: `.speccy/skills/` is no longer written by
    // `init`. The host-native reviewer files under `.claude/agents/`
    // / `.codex/agents/` are now the sole canonical persona surface
    // (classified Skip-on-exists by `append_host_pack_items`); the
    // CLI-rendered reviewer prompt no longer carries `{{persona_content}}`
    // (SPEC-0027 REQ-003); the legacy `.speccy/skills/prompts/`
    // override directory had no consumer in `speccy_core::prompt`.

    Ok(plan)
}

/// Materialise the host-templated wrapper pack via [`render_host_pack`]
/// and append one [`PlanItem`] per rendered file to `plan`.
///
/// SPEC-0016 T-007 replaced the previous per-host SKILL.md filesystem
/// walk: wrappers under `resources/agents/.<install_root>/` are now
/// rendered through `MiniJinja` with the host's template context (see
/// [`HostChoice::template_context`]), `.tmpl` suffixes are stripped,
/// and the resulting `rel_path` is joined onto `project_root` to give
/// the absolute destination. The three-way classification from
/// SPEC-0033 T-008 applies: absent → Create, byte-identical → Unchanged,
/// differs → Conflict.
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

/// Materialise the host-agnostic Speccy examples pack via
/// [`render_speccy_examples_pack`] and append one [`PlanItem`] per
/// rendered file to `plan`.
///
/// SPEC-0031 REQ-004 + DEC-004: example bodies under
/// `resources/modules/examples/*` are emitted to `.speccy/examples/*`
/// regardless of the chosen [`HostChoice`]. They are template-rendered
/// files, not user-tunable persona definitions, so they follow the
/// three-way classification via [`classify_content`]: Create on absent,
/// Unchanged when byte-identical, Conflict when differs (atomic batch
/// refuse without `--force`, overwrite with `(!) overwritten` log
/// under `--force`) — no Skip-on-exists override.
fn append_speccy_examples_items(
    project_root: &Utf8Path,
    plan: &mut Vec<PlanItem>,
) -> Result<(), InitError> {
    let rendered = render_speccy_examples_pack()?;
    for file in rendered {
        let destination = project_root.join(&file.rel_path);
        let content = file.contents.into_bytes();
        let action = classify_content(&destination, &content);
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

fn project_name_from(project_root: &Utf8Path) -> String {
    project_root.file_name().map_or_else(
        || "speccy-project".to_owned(),
        std::borrow::ToOwned::to_owned,
    )
}

fn render_speccy_toml(project_name: &str) -> String {
    let escaped = project_name.replace('\\', "\\\\").replace('"', "\\\"");
    SPECCY_TOML_TEMPLATE.replace("{{name}}", &escaped)
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

#[cfg(test)]
mod tests {
    use super::project_name_from;
    use super::render_speccy_toml;
    use camino::Utf8PathBuf;

    #[test]
    fn project_name_uses_parent_directory() {
        let root = Utf8PathBuf::from("/foo/bar");
        assert_eq!(project_name_from(&root), "bar");
    }

    #[test]
    fn project_name_falls_back_on_empty() {
        let root = Utf8PathBuf::from("/");
        let name = project_name_from(&root);
        assert!(!name.is_empty());
    }

    #[test]
    fn render_speccy_toml_substitutes_name() {
        let body = render_speccy_toml("acme");
        assert!(body.contains("name = \"acme\""));
        assert!(body.contains("schema_version = 1"));
    }

    #[test]
    fn render_speccy_toml_escapes_quotes() {
        let body = render_speccy_toml("foo\"bar");
        assert!(
            body.contains(r#"name = "foo\"bar""#),
            "embedded quote should be backslash-escaped, got: {body}",
        );
    }
}
