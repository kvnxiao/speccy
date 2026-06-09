//! `speccy journal append <task-selector> --block
//! {implementer|review|blockers}` command logic (SPEC-0055 REQ-003, REQ-005).
//!
//! Appends exactly one validated block to a per-task journal at
//! `<spec-dir>/journal/<task-id>.md`. The caller supplies only judgment and
//! identity (`--model`, `--persona`, `--verdict`, and the body on stdin); the
//! CLI is the sole authority for `date` (UTC now, ISO8601 seconds + `Z`) and
//! `round` (derived from existing file state). There is no flag to override
//! either — DEC-001.
//!
//! The derive→validate→append sequence runs under an advisory per-journal
//! file lock (REQ-005, DEC-007), so concurrent appenders serialize and each
//! observes a consistent round. Acquisition blocks until free with a
//! 10-second timeout (DEC-002); on timeout the command exits non-zero with
//! the journal byte-identical. Validation runs before any write, so a
//! malformed block leaves the journal untouched (or still absent).

use camino::Utf8Path;
use camino::Utf8PathBuf;
use fs4::FileExt;
use jiff::Timestamp;
use speccy_core::parse::BlockInputs;
use speccy_core::parse::SerializeError;
use speccy_core::parse::TaskBlockKind;
use speccy_core::parse::derive_round;
use speccy_core::parse::parse_journal_xml;
use speccy_core::parse::render_fresh_frontmatter;
use speccy_core::parse::validate_and_render_block;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskRef;
use speccy_core::task_lookup::find as find_task;
use speccy_core::task_lookup::parse_ref;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan;
use std::time::Duration;
use std::time::Instant;
use thiserror::Error;

/// How long lock acquisition blocks before giving up (DEC-002).
const LOCK_TIMEOUT: Duration = Duration::from_secs(10);

/// Poll interval while waiting for a contended lock.
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum JournalError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// Selector failed to parse or resolve via `task_lookup`.
    #[error(transparent)]
    TaskLookup(#[from] LookupError),
    /// The block body could not be read from stdin.
    #[error("failed to read block body from stdin")]
    Stdin {
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// An attaching block (`review`/`blockers`) was requested against a
    /// journal with no `implementer` block opening a round.
    #[error(
        "`{block}` requires an existing `<implementer>` block to attach to; \
         append an `implementer` block first"
    )]
    NoRoundToAttach {
        /// The attaching block's element name.
        block: &'static str,
    },
    /// Block validation failed; the journal was not modified.
    #[error(transparent)]
    Validation(#[from] SerializeError),
    /// The existing journal file failed to parse, so the round could not be
    /// derived safely. The file is left untouched.
    #[error("existing journal at {path} failed to parse; refusing to append")]
    ExistingJournalUnparseable {
        /// Path of the unparseable journal.
        path: Utf8PathBuf,
        /// Underlying parse error.
        #[source]
        source: Box<speccy_core::error::ParseError>,
    },
    /// Lock acquisition timed out after [`LOCK_TIMEOUT`].
    #[error(
        "timed out after {timeout_secs}s waiting for the journal lock at {path}; \
         no bytes were written"
    )]
    LockTimeout {
        /// Path of the journal whose lock could not be acquired.
        path: Utf8PathBuf,
        /// The configured timeout in whole seconds.
        timeout_secs: u64,
    },
    /// Reading or writing a journal-related file failed.
    #[error("journal I/O failed at {path}")]
    Io {
        /// The path that could not be read or written.
        path: Utf8PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// `speccy journal append` arguments.
#[derive(Debug, Clone)]
pub struct AppendArgs {
    /// Positional task selector: `T-NNN` or `SPEC-NNNN/T-NNN`.
    pub selector: String,
    /// Block type to append.
    pub block: TaskBlockKind,
    /// `--model` value (required for `implementer` and `review`).
    pub model: Option<String>,
    /// `--persona` value (required for `review`).
    pub persona: Option<String>,
    /// `--verdict` value (required for `review`).
    pub verdict: Option<String>,
}

/// Run `speccy journal append` from `cwd`, reading the block body from
/// `body_source`.
///
/// Resolves the selector to a per-task journal path, acquires the advisory
/// file lock, derives the round, validates and renders the block, and appends
/// it (creating the file with frontmatter on first append). The body is read
/// from `body_source` so tests can inject a reader without a real stdin.
///
/// # Errors
///
/// Returns a [`JournalError`] when discovery fails, the selector does not
/// resolve, the body cannot be read, validation fails, the lock times out, or
/// a file operation fails. On every error path the journal is left
/// byte-identical (or still absent).
pub fn run(
    args: AppendArgs,
    cwd: &Utf8Path,
    body_source: &mut impl std::io::Read,
) -> Result<(), JournalError> {
    let AppendArgs {
        selector,
        block,
        model,
        persona,
        verdict,
    } = args;

    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => return Err(JournalError::ProjectRootNotFound),
        Err(other) => return Err(JournalError::Workspace(other)),
    };

    let task_ref: TaskRef = parse_ref(&selector)?;
    let ws = scan(&project_root);
    let location = find_task(&ws, &task_ref)?;

    let task_id = location.task.id.clone();
    let spec_id = location.spec_id.clone();
    let journal_dir = location.spec_dir.join("journal");
    let journal_path = journal_dir.join(format!("{task_id}.md"));

    // Read the whole body before taking the lock — stdin reads should not
    // hold the lock open against other appenders.
    let mut body = String::new();
    body_source
        .read_to_string(&mut body)
        .map_err(|source| JournalError::Stdin { source })?;

    fs_err::create_dir_all(journal_dir.as_std_path()).map_err(|source| JournalError::Io {
        path: journal_dir.clone(),
        source,
    })?;

    // Acquire the advisory lock on a sidecar file, so locking works even
    // before the journal itself exists and never opens the journal merely to
    // lock it.
    let lock_path = journal_dir.join(format!("{task_id}.md.lock"));
    let _guard = LockGuard::acquire(&lock_path)?;

    // --- critical section: derive → validate → append → write ---
    let inputs = AppendInputs {
        journal_path: &journal_path,
        spec_id: &spec_id,
        task_id: &task_id,
        block,
        model: model.as_deref(),
        persona: persona.as_deref(),
        verdict: verdict.as_deref(),
        body: &body,
    };
    append_under_lock(&inputs)
}

/// Resolved, borrowed inputs for the critical-section append.
struct AppendInputs<'a> {
    journal_path: &'a Utf8Path,
    spec_id: &'a str,
    task_id: &'a str,
    block: TaskBlockKind,
    model: Option<&'a str>,
    persona: Option<&'a str>,
    verdict: Option<&'a str>,
    body: &'a str,
}

/// The derive→validate→render→write sequence, run with the lock held.
fn append_under_lock(inputs: &AppendInputs<'_>) -> Result<(), JournalError> {
    let &AppendInputs {
        journal_path,
        spec_id,
        task_id,
        block,
        model,
        persona,
        verdict,
        body,
    } = inputs;
    let existing = match fs_err::read_to_string(journal_path.as_std_path()) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(JournalError::Io {
                path: journal_path.to_path_buf(),
                source,
            });
        }
    };

    let parsed = match &existing {
        Some(src) => Some(parse_journal_xml(src, journal_path).map_err(|source| {
            JournalError::ExistingJournalUnparseable {
                path: journal_path.to_path_buf(),
                source,
            }
        })?),
        None => None,
    };

    // A `NoRoundError` means an attaching block was requested with no
    // `implementer` round to attach to; surface it with the block name the
    // user passed.
    let Ok(round) = derive_round(parsed.as_ref(), block) else {
        return Err(JournalError::NoRoundToAttach {
            block: block.element_name(),
        });
    };

    let now = Timestamp::now();
    let date = format_iso_z(now);

    let rendered = validate_and_render_block(&BlockInputs {
        kind: block,
        date: &date,
        round,
        model,
        persona,
        verdict,
        body,
    })?;

    // Existing files already end in a newline (the parser-accepted shape and
    // the renderer's own output), so a fresh block appends directly. A new
    // file gets CLI-stamped frontmatter first.
    let new_content = if let Some(prior) = existing {
        format!("{prior}{rendered}")
    } else {
        let frontmatter = render_fresh_frontmatter(spec_id, task_id, &date);
        format!("{frontmatter}{rendered}")
    };

    fs_err::write(journal_path.as_std_path(), new_content).map_err(|source| JournalError::Io {
        path: journal_path.to_path_buf(),
        source,
    })
}

/// Format `ts` as ISO8601 with seconds and a `Z` designator
/// (`YYYY-MM-DDTHH:MM:SSZ`), the shape the journal parser accepts.
fn format_iso_z(ts: Timestamp) -> String {
    let secs = ts.as_second();
    Timestamp::from_second(secs).map_or_else(|_| ts.to_string(), |t| t.to_string())
}

/// RAII holder for the advisory file lock. The lock is released when the
/// guard drops (explicitly via `unlock`, with the OS releasing it on close as
/// a backstop).
struct LockGuard {
    file: std::fs::File,
}

impl LockGuard {
    /// Acquire the exclusive advisory lock on `lock_path`, polling
    /// [`FileExt::try_lock`] until the [`LOCK_TIMEOUT`] deadline.
    ///
    /// Blocking-with-timeout (DEC-002) rather than `lock()`'s unbounded
    /// block, so a wedged holder errors loudly instead of hanging forever.
    fn acquire(lock_path: &Utf8Path) -> Result<Self, JournalError> {
        let file = fs_err::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .read(true)
            .open(lock_path.as_std_path())
            .map_err(|source| JournalError::Io {
                path: lock_path.to_path_buf(),
                source,
            })?;
        // fs4's lock methods extend std::fs::File; unwrap the fs-err wrapper.
        let (file, _path) = file.into_parts();

        let deadline = Instant::now() + LOCK_TIMEOUT;
        loop {
            match FileExt::try_lock(&file) {
                Ok(()) => return Ok(LockGuard { file }),
                Err(fs4::TryLockError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(JournalError::LockTimeout {
                            path: lock_path.to_path_buf(),
                            timeout_secs: LOCK_TIMEOUT.as_secs(),
                        });
                    }
                    std::thread::sleep(LOCK_POLL_INTERVAL);
                }
                Err(fs4::TryLockError::Error(source)) => {
                    return Err(JournalError::Io {
                        path: lock_path.to_path_buf(),
                        source,
                    });
                }
            }
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // Best-effort release; the OS releases the advisory lock on close
        // regardless, so a failure here is not actionable.
        if FileExt::unlock(&self.file).is_err() {
            // lock already released or file closed; nothing more to do.
        }
    }
}
