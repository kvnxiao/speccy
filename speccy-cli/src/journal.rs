//! `speccy journal append <selector> --block
//! {implementer|review|blockers|drift-review|holistic-fix|simplifier-scan|simplifier-apply|gate}`
//! command logic (SPEC-0055 REQ-003, REQ-004, REQ-005).
//!
//! Appends exactly one validated block to a journal. Target inference follows
//! DEC-004: task block types require a task selector (`T-NNN` /
//! `SPEC-NNNN/T-NNN`) and route to `<spec-dir>/journal/<task-id>.md`; vet
//! block types require a bare spec selector (`SPEC-NNNN`) and route to
//! `<spec-dir>/journal/VET.md`. A mismatched block-type/selector pairing is
//! an argument error — there is no `--vet` flag.
//!
//! The caller supplies only judgment and identity (`--model`, `--persona`,
//! `--verdict`, and the body on stdin); the CLI is the sole authority for
//! every environment-derivable value — `date` (UTC now, ISO8601 seconds +
//! `Z`), `round` (derived from existing file state), the vet journal's
//! invocation sectioning, and a `gate` block's `tasks_hash` (lowercase hex
//! SHA-256 of the sibling TASKS.md read at append time). There is no flag to
//! override any of these — DEC-001 / DEC-004.
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
use sha2::Digest as _;
use sha2::Sha256;
use speccy_core::parse::AppendPlan;
use speccy_core::parse::AppendPlanError;
use speccy_core::parse::BlockInputs;
use speccy_core::parse::SerializeError;
use speccy_core::parse::TaskBlockKind;
use speccy_core::parse::VetBlockInputs;
use speccy_core::parse::VetBlockKind;
use speccy_core::parse::VetDoc;
use speccy_core::parse::VetSerializeError;
use speccy_core::parse::derive_round;
use speccy_core::parse::parse_journal_xml;
use speccy_core::parse::parse_vet_in_flight;
use speccy_core::parse::plan_vet_append;
use speccy_core::parse::render_fresh_frontmatter;
use speccy_core::parse::render_fresh_vet_frontmatter;
use speccy_core::parse::render_vet_section_heading;
use speccy_core::parse::validate_and_render_block;
use speccy_core::parse::validate_and_render_vet_block;
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

/// Regex matching a bare `SPEC-NNNN` selector (4+ digits) with no trailing
/// task component — the DEC-004 routing discriminant. Shared with
/// `journal show` to keep both commands' target inference in lockstep.
pub(crate) fn bare_spec_selector_regex() -> &'static regex::Regex {
    use std::sync::OnceLock;
    static CELL: OnceLock<regex::Regex> = OnceLock::new();
    #[expect(
        clippy::unwrap_used,
        reason = "compile-time literal regex; covered by tests"
    )]
    CELL.get_or_init(|| regex::Regex::new(r"^SPEC-\d{4,}$").unwrap())
}

/// The block type a `journal append` invocation names, partitioned by which
/// journal it targets. The CLI value-parser maps `--block <name>` to one of
/// these; target inference (DEC-004) then checks the selector shape against
/// the variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalBlock {
    /// A per-task journal block; requires a task selector.
    Task(TaskBlockKind),
    /// A pre-ship vet journal block; requires a bare spec selector.
    Vet(VetBlockKind),
}

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
    /// A task block type was requested with a bare spec selector, or a vet
    /// block type with a task selector (DEC-004). The block type carries the
    /// target journal, so a mismatched pairing is an argument error.
    #[error(
        "`{block}` is a {expected} block type but the selector `{selector}` is a {got} selector; \
         {hint}"
    )]
    SelectorBlockMismatch {
        /// The `--block` value the caller passed.
        block: &'static str,
        /// What the block type requires (`vet`/`task`).
        expected: &'static str,
        /// What the selector actually is (`task`/`spec`).
        got: &'static str,
        /// The offending selector string.
        selector: String,
        /// A one-line corrective hint.
        hint: &'static str,
    },
    /// A bare spec selector did not match the `SPEC-NNNN` shape.
    #[error("invalid spec selector `{selector}`; expected `SPEC-NNNN` (4+ digits)")]
    InvalidSpecSelector {
        /// The offending selector string.
        selector: String,
    },
    /// A bare spec selector resolved to no spec in the workspace.
    #[error("spec `{spec_id}` not found in any spec directory")]
    SpecNotFound {
        /// The canonical spec id that resolved to nothing.
        spec_id: String,
    },
    /// A vet block could not be placed in the current VET.md shape.
    #[error(transparent)]
    VetPlan(#[from] AppendPlanError),
    /// Vet block validation failed; the journal was not modified.
    #[error(transparent)]
    VetValidation(#[from] VetSerializeError),
    /// The would-be new VET.md content (existing bytes + the freshly rendered
    /// block) does not parse under the VET parser, so the append is refused
    /// before any write (DEC-008). The parser is the single authority over
    /// what lands on disk: any body that would produce an unparseable file —
    /// e.g. one whose own line is a vet tag the scanner reads as a nested
    /// block — is rejected at write time, leaving VET.md byte-identical (or
    /// still absent). The round-trip uses the in-flight parser so a block
    /// that legitimately leaves an open trailing section (every non-`gate`
    /// block) is accepted, while still rejecting structural corruption.
    #[error(
        "the appended block would make VET.md at {path} unparseable; \
         refusing to write (VET.md left unchanged)"
    )]
    ProducedVetUnparseable {
        /// Path of the VET.md that would have become unparseable.
        path: Utf8PathBuf,
        /// Underlying parse error from the round-trip.
        #[source]
        source: Box<speccy_core::error::ParseError>,
    },
    /// The existing VET.md on disk does not parse under the in-flight VET
    /// parser, so the round/invocation state cannot be derived safely and the
    /// append is refused with the file left untouched. Symmetric to
    /// [`JournalError::ExistingJournalUnparseable`] for the per-task journal:
    /// a hand-corrupted or grammar-violating VET.md is surfaced loudly rather
    /// than appended to.
    #[error("existing VET.md at {path} failed to parse; refusing to append")]
    ExistingVetUnparseable {
        /// Path of the unparseable VET.md.
        path: Utf8PathBuf,
        /// Underlying parse error.
        #[source]
        source: Box<speccy_core::error::ParseError>,
    },
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
    /// Positional selector: a task selector (`T-NNN` / `SPEC-NNNN/T-NNN`) for
    /// task block types, or a bare spec selector (`SPEC-NNNN`) for vet block
    /// types.
    pub selector: String,
    /// Block type to append (carries the target journal — DEC-004).
    pub block: JournalBlock,
    /// `--model` value (required for `implementer`/`review`, and for the
    /// round-bearing vet blocks `drift-review`/`holistic-fix`).
    pub model: Option<String>,
    /// `--persona` value (required for `review`).
    pub persona: Option<String>,
    /// `--verdict` value (required for `review` and every vet block).
    pub verdict: Option<String>,
}

/// Run `speccy journal append` from `cwd`, reading the block body from
/// `body_source`.
///
/// Routes by block type (DEC-004): task block types resolve a task selector
/// to `<spec-dir>/journal/<task-id>.md`; vet block types resolve a bare spec
/// selector to `<spec-dir>/journal/VET.md`. In both cases it acquires the
/// advisory file lock, derives state (round / invocation), validates and
/// renders the block, and appends it (creating the file with frontmatter on
/// first append). The body is read from `body_source` so tests can inject a
/// reader without a real stdin.
///
/// # Errors
///
/// Returns a [`JournalError`] when discovery fails, the selector does not
/// resolve or mismatches the block type, the body cannot be read, validation
/// fails, the lock times out, or a file operation fails. On every error path
/// the journal is left byte-identical (or still absent).
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

    // Read the whole body before taking any lock — stdin reads should not
    // hold the lock open against other appenders.
    let mut body = String::new();
    body_source
        .read_to_string(&mut body)
        .map_err(|source| JournalError::Stdin { source })?;

    match block {
        JournalBlock::Task(kind) => run_task_append(
            &project_root,
            &selector,
            kind,
            model.as_deref(),
            persona.as_deref(),
            verdict.as_deref(),
            &body,
        ),
        JournalBlock::Vet(kind) => run_vet_append(
            &project_root,
            &selector,
            kind,
            model.as_deref(),
            verdict.as_deref(),
            &body,
        ),
    }
}

/// Resolve a task selector and append a task-journal block.
fn run_task_append(
    project_root: &Utf8Path,
    selector: &str,
    kind: TaskBlockKind,
    model: Option<&str>,
    persona: Option<&str>,
    verdict: Option<&str>,
    body: &str,
) -> Result<(), JournalError> {
    // A vet selector with a task block type is the DEC-004 mismatch.
    if bare_spec_selector_regex().is_match(selector) {
        return Err(JournalError::SelectorBlockMismatch {
            block: kind.element_name(),
            expected: "task",
            got: "spec",
            selector: selector.to_owned(),
            hint: "task block types need a task selector like `SPEC-NNNN/T-NNN`",
        });
    }

    let task_ref: TaskRef = parse_ref(selector)?;
    let ws = scan(project_root);
    let location = find_task(&ws, &task_ref)?;

    let task_id = location.task.id.clone();
    let spec_id = location.spec_id.clone();
    let journal_dir = location.spec_dir.join("journal");
    let journal_path = journal_dir.join(format!("{task_id}.md"));

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
        block: kind,
        model,
        persona,
        verdict,
        body,
    };
    append_under_lock(&inputs)
}

/// Resolve a bare spec selector and append a vet-journal block to VET.md.
fn run_vet_append(
    project_root: &Utf8Path,
    selector: &str,
    kind: VetBlockKind,
    model: Option<&str>,
    verdict: Option<&str>,
    body: &str,
) -> Result<(), JournalError> {
    // A task selector with a vet block type is the DEC-004 mismatch.
    if !bare_spec_selector_regex().is_match(selector) {
        // Distinguish "looks like a task selector" from "not a spec id at all"
        // for a clearer message.
        let got = if selector.contains('/') || selector.starts_with("T-") {
            "task"
        } else {
            return Err(JournalError::InvalidSpecSelector {
                selector: selector.to_owned(),
            });
        };
        return Err(JournalError::SelectorBlockMismatch {
            block: kind.element_name(),
            expected: "vet",
            got,
            selector: selector.to_owned(),
            hint: "vet block types need a bare spec selector like `SPEC-NNNN`",
        });
    }

    let ws = scan(project_root);
    let spec = ws
        .specs
        .iter()
        .find(|s| s.spec_id.as_deref() == Some(selector))
        .ok_or_else(|| JournalError::SpecNotFound {
            spec_id: selector.to_owned(),
        })?;
    let spec_dir = spec.dir.clone();
    let tasks_md_path = spec_dir.join("TASKS.md");
    let journal_dir = spec_dir.join("journal");
    let journal_path = journal_dir.join("VET.md");

    fs_err::create_dir_all(journal_dir.as_std_path()).map_err(|source| JournalError::Io {
        path: journal_dir.clone(),
        source,
    })?;

    let lock_path = journal_dir.join("VET.md.lock");
    let _guard = LockGuard::acquire(&lock_path)?;

    // --- critical section: derive → validate → append → write ---
    let inputs = VetAppendInputs {
        journal_path: &journal_path,
        tasks_md_path: &tasks_md_path,
        spec_id: selector,
        block: kind,
        model,
        verdict,
        body,
    };
    append_vet_under_lock(&inputs)
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

/// Resolved, borrowed inputs for a vet-journal append.
struct VetAppendInputs<'a> {
    journal_path: &'a Utf8Path,
    tasks_md_path: &'a Utf8Path,
    spec_id: &'a str,
    block: VetBlockKind,
    model: Option<&'a str>,
    verdict: Option<&'a str>,
    body: &'a str,
}

/// The parse → plan → validate → render → round-trip → write sequence for a
/// vet block, run with the lock held.
///
/// The VET parser is the single authority throughout (DEC-008): the existing
/// file is parsed with [`parse_vet_in_flight`] (which tolerates the open
/// trailing section that exists mid-vet-run), the invocation/round placement
/// is derived from that typed [`VetDoc`] — mirroring how the per-task path
/// derives `round` from a typed `parse_journal_xml` — a `gate` block's
/// `tasks_hash` is computed from the sibling TASKS.md, and the would-be-new
/// file is re-parsed through the same parser before any byte is written.
/// Both round-trips happen inside the lock critical section, so a concurrent
/// appender cannot slip between derive/validate and write.
fn append_vet_under_lock(inputs: &VetAppendInputs<'_>) -> Result<(), JournalError> {
    let &VetAppendInputs {
        journal_path,
        tasks_md_path,
        spec_id,
        block,
        model,
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

    // Parse the existing file once with the in-flight parser; a corrupt
    // existing VET.md is surfaced loudly rather than appended to. The plan is
    // derived from the typed document, so derivation reuses the parser's own
    // section/block/round structure (no separate text scan).
    let parsed: Option<VetDoc> = match &existing {
        Some(src) => Some(parse_vet_in_flight(src, journal_path).map_err(|source| {
            JournalError::ExistingVetUnparseable {
                path: journal_path.to_path_buf(),
                source,
            }
        })?),
        None => None,
    };

    let plan: AppendPlan = plan_vet_append(parsed.as_ref(), block)?;

    let now = Timestamp::now();
    let date = format_iso_z(now);

    // A `gate` carries the lowercase hex SHA-256 of the sibling TASKS.md read
    // at append time — the exact bytes and encoding `speccy next`'s freshness
    // check recomputes (`Sha256` + `const_hex::encode`).
    let tasks_hash = if matches!(block, VetBlockKind::Gate) {
        let bytes =
            fs_err::read(tasks_md_path.as_std_path()).map_err(|source| JournalError::Io {
                path: tasks_md_path.to_path_buf(),
                source,
            })?;
        Some(const_hex::encode(Sha256::digest(&bytes)))
    } else {
        None
    };

    let rendered = validate_and_render_vet_block(&VetBlockInputs {
        kind: block,
        date: &date,
        round: plan.round,
        verdict,
        model,
        tasks_hash: tasks_hash.as_deref(),
        body,
    })?;

    // Assemble the new content: optional frontmatter (fresh file), optional
    // new section heading (plan says open one), then the block. Existing
    // content already ends in a newline.
    let mut new_content = String::new();
    if let Some(prior) = &existing {
        new_content.push_str(prior);
    } else {
        new_content.push_str(&render_fresh_vet_frontmatter(spec_id, &date));
    }
    if plan.open_new_section {
        new_content.push_str(&render_vet_section_heading(plan.invocation_number, &date));
    }
    new_content.push_str(&rendered);

    // Round-trip the COMPLETE would-be new file through the VET parser before
    // writing a byte (DEC-008). This is the single authority over what lands
    // on disk: any body that would produce an unparseable file — e.g. one
    // whose own line is a vet tag the scanner reads as a nested block — is
    // rejected here, so no separate body-markup guard is needed. The in-flight
    // parser is used so the open trailing section a non-`gate` block
    // legitimately leaves is accepted, while structural corruption (nested or
    // mismatched tags, a phantom heading inside a closed block) is still
    // refused. A `gate`-terminated file has no open last section, so the
    // in-flight parser validates it identically to the strict parser.
    parse_vet_in_flight(&new_content, journal_path).map_err(|source| {
        JournalError::ProducedVetUnparseable {
            path: journal_path.to_path_buf(),
            source,
        }
    })?;

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
