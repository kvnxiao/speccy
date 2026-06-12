//! `speccy task transition <selector> --to <state>` command logic.
//!
//! Resolves the selector via the same `task_lookup` seam `speccy check`
//! uses, classifies the requested `from -> to` edge against the closed
//! legal state graph (SPEC-0055 REQ-002), and — when the edge is legal —
//! byte-surgically rewrites the task's `state` attribute in TASKS.md,
//! preserving every other byte (SPEC-0055 REQ-001). A same-state request
//! is an idempotent no-op (DEC-003); an illegal edge or an unresolved
//! selector exits non-zero with the file untouched.
//!
//! The rewrite delegates to [`speccy_core::parse::splice_task_state`],
//! which never round-trips through the TASKS.md renderer.

use camino::Utf8Path;
use speccy_core::parse::TaskState;
use speccy_core::parse::TransitionKind;
use speccy_core::parse::classify_transition;
use speccy_core::parse::splice_task_state;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskRef;
use speccy_core::task_lookup::find as find_task;
use speccy_core::task_lookup::parse_ref;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::scan;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TransitionError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// Selector failed to parse or resolve via `task_lookup`. Carries the
    /// existing `LookupError` `Display` wording byte-for-byte so the
    /// message matches `speccy check` against the same reference.
    #[error(transparent)]
    TaskLookup(#[from] LookupError),
    /// The requested edge is not in the legal state graph (REQ-002). The
    /// diagnostic names both states and the fact that the edge is illegal.
    #[error(
        "illegal transition: `{from}` -> `{to}` is not in the legal state graph; \
         no file was modified"
    )]
    IllegalEdge {
        /// The task's current on-disk state.
        from: String,
        /// The requested target state.
        to: String,
    },
    /// The resolved task's open tag carried no `state` attribute to
    /// rewrite (a corrupt parse). The file is left untouched.
    #[error(transparent)]
    Splice(#[from] speccy_core::parse::SpliceError),
    /// Reading or writing TASKS.md failed.
    #[error("failed to write TASKS.md at {path}")]
    Io {
        /// Path of the TASKS.md that could not be written.
        path: camino::Utf8PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// `speccy task transition` arguments.
#[derive(Debug, Clone)]
pub struct TransitionArgs {
    /// Positional selector: `T-NNN` or `SPEC-NNNN/T-NNN`.
    pub selector: String,
    /// Validated target state. The CLI value parser already rejected any
    /// value outside the four legal states at argument-parse time, so
    /// this is always a known [`TaskState`].
    pub to: TaskState,
}

/// Run `speccy task transition` from `cwd`.
///
/// Resolves the selector, classifies the requested edge, and — on a legal
/// edge — splices the new state into TASKS.md byte-surgically. A
/// same-state request returns `Ok(())` without writing (DEC-003).
///
/// # Errors
///
/// Returns a [`TransitionError`] when discovery fails, the selector does
/// not resolve, the edge is illegal, or the rewrite cannot be written. On
/// every error path TASKS.md is left byte-identical.
pub fn run(args: TransitionArgs, cwd: &Utf8Path) -> Result<(), TransitionError> {
    let TransitionArgs { selector, to } = args;

    let project_root = crate::cwd::resolve_root(cwd, TransitionError::ProjectRootNotFound)?;

    let task_ref: TaskRef = parse_ref(&selector)?;
    let ws = scan(&project_root);
    let location = find_task(&ws, &task_ref)?;

    let from = location.task.state;
    match classify_transition(from, to) {
        TransitionKind::NoOp => {
            // DEC-003: same-state target is an idempotent success that
            // leaves the file byte-identical. Write nothing.
            Ok(())
        }
        TransitionKind::Illegal => Err(TransitionError::IllegalEdge {
            from: from.as_str().to_owned(),
            to: to.as_str().to_owned(),
        }),
        TransitionKind::Legal => {
            let rewritten = splice_task_state(&location.tasks_md.raw, location.task, to)?;
            let tasks_md_path = location.spec_dir.join("TASKS.md");
            fs_err::write(tasks_md_path.as_std_path(), rewritten).map_err(|source| {
                TransitionError::Io {
                    path: tasks_md_path,
                    source,
                }
            })?;
            // SPEC-0058 REQ-001 / REQ-003: the `--to completed` edge is a
            // terminal lifecycle boundary, so after the state rewrite lands
            // reap the task journal's advisory-lock sidecar. The reap is
            // guarded by a `try_lock` and is infallible by design (it runs
            // only after the load-bearing TASKS.md write succeeded), so a
            // held or absent sidecar is a safe no-op and never fails the
            // command. No other transition edge touches the sidecar, and the
            // journal `.md` itself is never opened.
            if to == TaskState::Completed {
                let lock_path = location
                    .spec_dir
                    .join("journal")
                    .join(format!("{}.md.lock", location.task.id));
                crate::journal::reap_lock_sidecar(&lock_path);
            }
            Ok(())
        }
    }
}
