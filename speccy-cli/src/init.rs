//! `speccy init` command logic.
//!
//! Scaffolds a `.speccy/` workspace and copies the host skill pack into
//! the host-native location. Host detection lives in [`crate::host`];
//! the embedded skill bundle lives in [`crate::embedded`]. This module
//! owns the planning, summary, and mutation steps.
//!
//! See `.speccy/specs/0002-init-command/SPEC.md`.

use crate::embedded::SKILLS;
use crate::host::Detected;
use crate::host::HostChoice;
use crate::host::detect_host;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use include_dir::Dir;
use std::io::Write;
use std::path::Component;
use thiserror::Error;

const SPECCY_TOML_TEMPLATE: &str = include_str!("templates/speccy.toml.tmpl");

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum InitError {
    /// `.speccy/` already exists and `--force` was not passed.
    #[error(".speccy/ already exists at {path}; pass --force to refresh shipped files in place")]
    WorkspaceExists {
        /// Path to the existing `.speccy/` directory.
        path: Utf8PathBuf,
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
    /// Embedded bundle is missing a sub-path the build is supposed to
    /// guarantee. Reachable only if the workspace `skills/` tree is
    /// edited in a way that strips a required sub-directory before the
    /// next release.
    #[error("embedded skill bundle is missing sub-path `{subpath}`; this is a build bug")]
    BundleSubpathMissing {
        /// Sub-path inside [`SKILLS`] that came back `None`.
        subpath: &'static str,
    },
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

/// Action a planned file write will take when executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// Destination does not exist; file will be written fresh.
    Create,
    /// Destination exists and is a shipped file; will be replaced.
    Overwrite,
}

impl Action {
    const fn label(self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Overwrite => "overwrite",
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

    let speccy_dir = project_root.join(".speccy");
    let speccy_exists = fs_err::metadata(speccy_dir.as_std_path()).is_ok_and(|m| m.is_dir());
    if speccy_exists && !force {
        return Err(InitError::WorkspaceExists { path: speccy_dir });
    }

    let plan = build_plan(project_root, host)?;
    print_plan(&plan, project_root, out)?;
    let outcome = execute_plan(&plan)?;
    writeln!(
        out,
        "Init complete: {created} created, {overwritten} overwritten.",
        created = outcome.created,
        overwritten = outcome.overwritten,
    )?;

    Ok(())
}

#[derive(Debug, Default, Clone, Copy)]
struct Outcome {
    created: u32,
    overwritten: u32,
}

fn build_plan(project_root: &Utf8Path, host: HostChoice) -> Result<Vec<PlanItem>, InitError> {
    let mut plan: Vec<PlanItem> = Vec::new();

    let speccy_toml_path = project_root.join(".speccy").join("speccy.toml");
    let project_name = project_name_from(project_root);
    let speccy_toml_body = render_speccy_toml(&project_name);
    let speccy_toml_action = classify(&speccy_toml_path);
    plan.push(PlanItem {
        destination: speccy_toml_path,
        content: speccy_toml_body.into_bytes(),
        action: speccy_toml_action,
    });

    let dest_segs = host.destination_segments();
    let host_dest_root = project_root.join(dest_segs[0]).join(dest_segs[1]);
    append_bundle_items(host.bundle_subpath(), &host_dest_root, &mut plan)?;

    let personas_dest = project_root.join(".speccy").join("skills").join("personas");
    append_bundle_items("shared/personas", &personas_dest, &mut plan)?;

    let prompts_dest = project_root.join(".speccy").join("skills").join("prompts");
    append_bundle_items("shared/prompts", &prompts_dest, &mut plan)?;

    Ok(plan)
}

fn append_bundle_items(
    subpath: &'static str,
    dest_root: &Utf8Path,
    plan: &mut Vec<PlanItem>,
) -> Result<(), InitError> {
    let Some(dir) = SKILLS.get_dir(subpath) else {
        return Err(InitError::BundleSubpathMissing { subpath });
    };
    let mut entries: Vec<(Utf8PathBuf, &'static [u8])> = Vec::new();
    collect_bundle_files(dir, subpath, &mut entries);
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    for (rel, content) in entries {
        let dest = dest_root.join(&rel);
        let action = classify(&dest);
        plan.push(PlanItem {
            destination: dest,
            content: content.to_vec(),
            action,
        });
    }
    Ok(())
}

fn collect_bundle_files(
    dir: &Dir<'static>,
    prefix: &str,
    out: &mut Vec<(Utf8PathBuf, &'static [u8])>,
) {
    for file in dir.files() {
        let path = file.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !has_md_extension(file_name) {
            continue;
        }
        let Ok(rel) = path.strip_prefix(prefix) else {
            continue;
        };
        let mut rel_buf = Utf8PathBuf::new();
        for comp in rel.components() {
            if let Component::Normal(seg) = comp
                && let Some(s) = seg.to_str()
            {
                rel_buf.push(s);
            }
        }
        if rel_buf.as_str().is_empty() {
            continue;
        }
        out.push((rel_buf, file.contents()));
    }
    for sub in dir.dirs() {
        collect_bundle_files(sub, prefix, out);
    }
}

fn classify(dest: &Utf8Path) -> Action {
    if fs_err::metadata(dest.as_std_path()).is_ok() {
        Action::Overwrite
    } else {
        Action::Create
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
    out: &mut dyn Write,
) -> Result<(), InitError> {
    writeln!(out, "speccy init plan:")?;
    for item in plan {
        let rel = display_relative(&item.destination, project_root);
        writeln!(out, "  {label:<9} {rel}", label = item.action.label())?;
    }
    Ok(())
}

fn display_relative(dest: &Utf8Path, project_root: &Utf8Path) -> String {
    dest.strip_prefix(project_root)
        .map_or_else(|_e| dest.to_string(), ToString::to_string)
}

fn execute_plan(plan: &[PlanItem]) -> Result<Outcome, InitError> {
    let mut outcome = Outcome::default();
    for item in plan {
        write_item(item)?;
        match item.action {
            Action::Create => {
                outcome.created = outcome.created.saturating_add(1);
            }
            Action::Overwrite => {
                outcome.overwritten = outcome.overwritten.saturating_add(1);
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

fn has_md_extension(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
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
