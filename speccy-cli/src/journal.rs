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

use crate::check_selector::bare_spec_regex;
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
use speccy_core::workspace::scan;
use std::time::Duration;
use std::time::Instant;
use thiserror::Error;

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
    /// The would-be new per-task journal content (existing bytes + the freshly
    /// rendered block) does not parse under [`parse_journal_xml`], so the
    /// append is refused before any write (DEC-001, mirroring the vet path's
    /// [`JournalError::ProducedVetUnparseable`]). The parser is the single
    /// authority over what lands on disk: any body that would produce an
    /// unparseable file — e.g. one whose own line is a journal tag the scanner
    /// reads as a nested block — is rejected at write time, leaving the journal
    /// byte-identical (or still absent). Distinct from
    /// [`JournalError::ExistingJournalUnparseable`], which surfaces a corrupt
    /// file that was *already* on disk.
    #[error(
        "the appended block would make the journal at {path} unparseable; \
         refusing to write (journal left unchanged)"
    )]
    ProducedJournalUnparseable {
        /// Path of the journal that would have become unparseable.
        path: Utf8PathBuf,
        /// Underlying parse error from the round-trip.
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

    let project_root = crate::cwd::resolve_root(cwd, JournalError::ProjectRootNotFound)?;

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
    if bare_spec_regex().is_match(selector) {
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
    if !bare_spec_regex().is_match(selector) {
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

    // --- critical section: derive → validate → append → write ---
    // The append result is bound, then the guard is dropped by closing its
    // lexical scope, so the lock is provably released before the terminal-gate
    // reap below observes the sidecar (SPEC-0058 REQ-002).
    let appended = {
        let _guard = LockGuard::acquire(&lock_path)?;
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
    };

    // SPEC-0058 REQ-002 / DEC-003: the `<gate>` is the terminal vet write on
    // every exit path, so after the gate append lands reap `VET.md.lock`. Only
    // the gate reaps — every non-gate vet block (`drift-review`,
    // `holistic-fix`, `simplifier-scan`, `simplifier-apply`) leaves the
    // sidecar in place for the next sequential appender. The append's own
    // `_guard` released above, so the reap's `try_lock` (REQ-003) observes the
    // lock as free; terminal-boundary quiescence is the real safety contract.
    // The reap is infallible by design (it runs only after the load-bearing
    // append succeeded) — it runs solely on the `Ok` path and an absent
    // sidecar is a safe no-op.
    if appended.is_ok() && matches!(kind, VetBlockKind::Gate) {
        reap_lock_sidecar(&lock_path);
    }
    appended
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

    // Round-trip the COMPLETE would-be new file through the journal parser
    // before writing a byte (DEC-001), mirroring the vet path's write-time
    // round-trip. This is the single authority over what lands on disk: any
    // body that would produce an unparseable file — e.g. one whose own line is
    // a journal tag the scanner reads as a nested block — is rejected here, so
    // no separate body-markup pre-scan is needed. A body that merely mentions
    // an element name inline as prose stays inert and parses cleanly.
    parse_journal_xml(&new_content, journal_path).map_err(|source| {
        JournalError::ProducedJournalUnparseable {
            path: journal_path.to_path_buf(),
            source,
        }
    })?;

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

/// Delete a journal advisory-lock sidecar at a terminal lifecycle boundary,
/// guarded by a non-blocking `try_lock` so a mistimed or repeated reap is a
/// safe no-op rather than a mutual-exclusion break (DEC-001, DEC-004,
/// REQ-003).
///
/// Unlike [`LockGuard::acquire`], this opens the sidecar **without**
/// `create(true)`: an absent sidecar (`NotFound`) is the idempotent no-op and
/// is never re-created. A single non-blocking [`FileExt::try_lock`] verifies
/// the lock is currently free; a held lock (`WouldBlock`) is left untouched.
/// On a successful lock the file handle is dropped **before** `remove_file`,
/// releasing both the advisory lock and the OS handle so the unlink succeeds
/// identically on Windows (where deleting a path with a live open handle
/// fails with a sharing violation) and POSIX.
///
/// Infallible by design (DEC-004): the reap runs only after the owning
/// command's load-bearing mutation has already landed, so a reap failure must
/// never fail the command. The expected no-ops (`NotFound` open, `WouldBlock`
/// lock) return silently; any error short of a clean reap — an open error
/// other than `NotFound`, a `try_lock` `Error`, or a `remove_file` error —
/// emits exactly one `WARN` naming the sidecar path (REQ-005, DEC-005) and
/// still returns.
pub(crate) fn reap_lock_sidecar(lock_path: &Utf8Path) {
    let file = match fs_err::OpenOptions::new()
        .read(true)
        .write(true)
        .open(lock_path.as_std_path())
    {
        Ok(file) => file,
        // Absent sidecar: the idempotent no-op. Never create one here.
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return,
        Err(source) => {
            tracing::warn!(
                sidecar = %lock_path,
                error = %source,
                "failed to open journal lock sidecar for reaping",
            );
            return;
        }
    };
    // fs4's lock methods extend std::fs::File; unwrap the fs-err wrapper.
    let (file, _path) = file.into_parts();

    match FileExt::try_lock(&file) {
        Ok(()) => {}
        // Held by an in-flight appender: an expected no-op, never unlinked.
        Err(fs4::TryLockError::WouldBlock) => return,
        Err(fs4::TryLockError::Error(source)) => {
            tracing::warn!(
                sidecar = %lock_path,
                error = %source,
                "failed to probe journal lock sidecar for reaping",
            );
            return;
        }
    }

    // Drop the handle (releasing the advisory lock and the OS handle) before
    // unlinking, so the unlink behaves identically on Windows and POSIX
    // (DEC-004).
    drop(file);

    if let Err(source) = fs_err::remove_file(lock_path.as_std_path()) {
        tracing::warn!(
            sidecar = %lock_path,
            error = %source,
            "failed to unlink journal lock sidecar after reaping",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tracing::Level;

    /// A `tracing` collector that records each event's level and the rendered
    /// value of its `sidecar` field, so a test can assert how many `WARN`
    /// events fired and that they name the sidecar path.
    #[derive(Clone, Default)]
    struct CapturingCollector {
        events: Arc<Mutex<Vec<CapturedEvent>>>,
    }

    #[derive(Clone)]
    struct CapturedEvent {
        level: Level,
        // Only the induced-failure assertion reads this, and that test is
        // gated to Unix (see below), so the field is unused on other hosts.
        #[cfg_attr(
            not(unix),
            expect(dead_code, reason = "read only by the Unix-gated failure test")
        )]
        sidecar: Option<String>,
    }

    /// Extracts the `sidecar` field's `Display` value from an event.
    #[derive(Default)]
    struct SidecarVisitor {
        sidecar: Option<String>,
    }

    impl tracing::field::Visit for SidecarVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "sidecar" {
                self.sidecar = Some(format!("{value:?}"));
            }
        }

        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            if field.name() == "sidecar" {
                self.sidecar = Some(value.to_owned());
            }
        }
    }

    impl tracing::Subscriber for CapturingCollector {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            let mut visitor = SidecarVisitor::default();
            event.record(&mut visitor);
            if let Ok(mut events) = self.events.lock() {
                events.push(CapturedEvent {
                    level: *event.metadata().level(),
                    sidecar: visitor.sidecar,
                });
            }
        }

        fn enter(&self, _span: &tracing::span::Id) {}

        fn exit(&self, _span: &tracing::span::Id) {}
    }

    /// Returns the captured `WARN`-level events.
    fn warn_events(collector: &CapturingCollector) -> Vec<CapturedEvent> {
        let events = collector
            .events
            .lock()
            .expect("collector mutex should not be poisoned");
        events
            .iter()
            .filter(|event| event.level == Level::WARN)
            .cloned()
            .collect()
    }

    /// Creates a sidecar file at `<dir>/foo.md.lock` and returns its path.
    fn make_sidecar(dir: &Utf8Path) -> Utf8PathBuf {
        let path = dir.join("foo.md.lock");
        fs_err::write(path.as_std_path(), b"").expect("sidecar write should succeed");
        path
    }

    #[test]
    fn free_sidecar_is_unlinked_without_warning() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let dir = Utf8Path::from_path(dir.path()).expect("tempdir path should be UTF-8");
        let sidecar = make_sidecar(dir);

        let collector = CapturingCollector::default();
        tracing::subscriber::with_default(collector.clone(), || {
            reap_lock_sidecar(&sidecar);
        });

        assert!(
            !sidecar.as_std_path().exists(),
            "a free sidecar should be unlinked by the reap"
        );
        assert!(
            warn_events(&collector).is_empty(),
            "a clean reap must emit no WARN event"
        );
    }

    #[test]
    fn held_sidecar_survives_reap_without_warning() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let dir = Utf8Path::from_path(dir.path()).expect("tempdir path should be UTF-8");
        let sidecar = make_sidecar(dir);

        // Hold the exclusive advisory lock from a separate handle, mirroring an
        // in-flight appender.
        let held = fs_err::OpenOptions::new()
            .read(true)
            .write(true)
            .open(sidecar.as_std_path())
            .expect("opening the sidecar should succeed");
        let (held, _path) = held.into_parts();
        FileExt::lock(&held).expect("acquiring the test-held lock should succeed");

        let collector = CapturingCollector::default();
        tracing::subscriber::with_default(collector.clone(), || {
            reap_lock_sidecar(&sidecar);
        });

        assert!(
            sidecar.as_std_path().exists(),
            "a held sidecar must be left intact (the try_lock guard skips it)"
        );
        assert!(
            warn_events(&collector).is_empty(),
            "the held-lock skip is an expected no-op and must emit no WARN"
        );

        // Release for teardown; dropping the handle releases the OS lock.
        drop(held);
    }

    #[test]
    fn absent_sidecar_is_a_silent_noop() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let dir = Utf8Path::from_path(dir.path()).expect("tempdir path should be UTF-8");
        let sidecar = dir.join("foo.md.lock");

        let collector = CapturingCollector::default();
        tracing::subscriber::with_default(collector.clone(), || {
            reap_lock_sidecar(&sidecar);
        });

        assert!(
            !sidecar.as_std_path().exists(),
            "an absent sidecar must not be created by the reap"
        );
        assert!(
            warn_events(&collector).is_empty(),
            "an absent-sidecar reap is an expected no-op and must emit no WARN"
        );
    }

    // Inducing a `remove_file` failure after a successful `try_lock` relies on a
    // read-only parent directory, which only blocks deletion on POSIX; Windows
    // ignores the directory's read-only attribute for child deletion. Gate the
    // induction to Unix per the task's allowance (CHK-009, REQ-005).
    #[cfg(unix)]
    #[test]
    fn induced_unlink_failure_emits_one_warn_naming_the_sidecar() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let dir = Utf8Path::from_path(dir.path()).expect("tempdir path should be UTF-8");
        let sidecar = make_sidecar(dir);

        // Make the parent directory non-writable so `remove_file` fails after
        // the helper's `try_lock` succeeds.
        let mut perms = fs_err::metadata(dir.as_std_path())
            .expect("reading dir metadata should succeed")
            .permissions();
        perms.set_mode(0o555);
        fs_err::set_permissions(dir.as_std_path(), perms.clone())
            .expect("tightening dir permissions should succeed");

        let collector = CapturingCollector::default();
        tracing::subscriber::with_default(collector.clone(), || {
            reap_lock_sidecar(&sidecar);
        });

        // Restore writability so the tempdir can be torn down.
        perms.set_mode(0o755);
        fs_err::set_permissions(dir.as_std_path(), perms)
            .expect("restoring dir permissions should succeed");

        let warns = warn_events(&collector);
        assert_eq!(
            warns.len(),
            1,
            "an induced unlink failure must emit exactly one WARN"
        );
        let event = warns.first().expect("one WARN event was just asserted");
        assert_eq!(
            event.sidecar.as_deref(),
            Some(sidecar.as_str()),
            "the WARN must name the sidecar path as a structured field"
        );
    }
}
