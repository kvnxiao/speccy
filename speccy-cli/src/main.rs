//! Speccy CLI entry point.
//!
//! Thin dispatcher over command modules in `speccy_cli`. `clap` derives
//! parse the argv; each match arm resolves cwd, calls the library
//! function, and maps library errors to documented process exit codes.

use clap::Parser;
use clap::Subcommand;
use std::process::ExitCode;

/// Speccy CLI.
#[derive(Parser)]
#[command(
    name = "speccy",
    version,
    about = "Deterministic feedback engine for spec-driven development.",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a `.speccy/` workspace and copy the host skill pack.
    Init {
        /// Force overwrite of shipped files in an existing workspace.
        #[arg(long)]
        force: bool,
        /// Host pack to install (`claude-code` or `codex`).
        #[arg(long, value_name = "NAME")]
        host: Option<String>,
    },
    /// Print workspace overview (text by default; `--json` for envelope).
    ///
    /// `--all` broadens the attention filter on non-archived specs;
    /// `--include-archive` adds archived specs under `.speccy/archive/`
    /// to the scan. The two flags are independent and may be combined.
    Status {
        /// `SPEC-NNNN` to render exactly one spec, unfiltered. Cannot
        /// be combined with `--all`. Omit to use the default
        /// attention-list view (or pass `--all`).
        #[arg(value_name = "SELECTOR", conflicts_with = "all")]
        selector: Option<String>,
        /// Render every active spec in workspace order, unfiltered.
        /// Broadens the attention filter on non-archived specs only;
        /// pair with `--include-archive` to also surface archived
        /// specs. Cannot be combined with a positional `SPEC-NNNN`
        /// selector.
        #[arg(long)]
        all: bool,
        /// Also include specs under `.speccy/archive/` in the scan.
        /// Archived specs never participate in the attention-list
        /// filter; they are surfaced solely because this flag opts
        /// them in. Independent of `--all`.
        #[arg(long)]
        include_archive: bool,
        /// Emit JSON envelope (`schema_version = 1`).
        #[arg(long)]
        json: bool,
    },
    /// Pick the next actionable task across the workspace.
    Next {
        /// Optional `SPEC-NNNN` selector; omit for workspace-wide listing.
        #[arg(value_name = "SPEC-ID")]
        spec_id: Option<String>,
        /// Also include specs under `.speccy/archive/` in the scan.
        /// Archived specs are terminal — they only ever resolve to a
        /// `reason` (`completed` / `dropped` / `superseded`) in the
        /// per-spec form. Mirrors `status --include-archive`.
        #[arg(long)]
        include_archive: bool,
        /// Emit JSON envelope (`schema_version = 1`).
        #[arg(long)]
        json: bool,
    },
    /// Render `<scenario>` Given/When/Then prose from SPEC.md (no execution).
    Check {
        /// Polymorphic selector; omit to render every scenario across every
        /// spec. Accepted shapes: `SPEC-NNNN` (all scenarios in spec),
        /// `SPEC-NNNN/CHK-NNN` (one spec-qualified scenario),
        /// `SPEC-NNNN/T-NNN` (scenarios covering the task's requirements),
        /// `CHK-NNN` (every spec's `CHK-NNN`), or
        /// `T-NNN` (unqualified task).
        #[arg(value_name = "SELECTOR")]
        selector: Option<String>,
        /// Also include specs under `.speccy/archive/` in the scan, so
        /// scenarios from archived SPECs render alongside active ones.
        /// Mirrors `status --include-archive`.
        #[arg(long)]
        include_archive: bool,
    },
    /// Emit a task- or spec-scoped context bundle for loop subagents.
    ///
    /// A bare `SPEC-NNNN` prints a whole-SPEC bundle for the vet loop.
    /// Task selectors (`T-NNN` or `SPEC-NNNN/T-NNN`) retain the task-scoped
    /// bundle and same lookup diagnostics as `speccy check`. `--json`
    /// toggles representation, never content; agents always pass `--json`.
    /// The command performs no writes anywhere. Failures exit non-zero with
    /// no partial stdout.
    Context {
        /// Selector: `SPEC-NNNN`, `T-NNN`, or `SPEC-NNNN/T-NNN`.
        #[arg(value_name = "SELECTOR")]
        selector: String,
        /// Emit JSON envelope (`schema_version = 1`).
        #[arg(long)]
        json: bool,
    },
    /// CI gate: proof-shape validation with a binary exit code.
    Verify {
        /// Also include specs under `.speccy/archive/` in the gate, so
        /// proof-shape errors on archived SPECs continue to fail CI
        /// after archiving. Mirrors `status --include-archive`.
        #[arg(long)]
        include_archive: bool,
        /// Emit JSON envelope (`schema_version = 1`).
        #[arg(long)]
        json: bool,
    },
    /// Record SPEC.md content hash and UTC timestamp into TASKS.md frontmatter.
    Lock {
        /// `SPEC-NNNN` identifier of the spec to lock.
        #[arg(value_name = "SPEC-ID")]
        spec_id: String,
    },
    /// Report the next free SPEC-NNNN identifier.
    Vacancy {
        /// Emit JSON envelope (`schema_version = 1`).
        #[arg(long)]
        json: bool,
    },
    /// Task lifecycle commands (state transitions).
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    /// Per-task journal commands (validated block appends).
    Journal {
        #[command(subcommand)]
        command: JournalCommand,
    },
    /// Relocate a shipped/dropped/superseded spec into `.speccy/archive/`.
    Archive {
        /// `SPEC-NNNN` identifier of the spec to archive.
        #[arg(value_name = "SPEC-ID")]
        spec_id: String,
        /// Free-form reason recorded into SPEC.md frontmatter as
        /// `archived_reason`. Newlines are rejected.
        #[arg(long, value_name = "STRING", value_parser = speccy_cli::archive::parse_reason)]
        reason: Option<String>,
        /// Bypass the status gate that refuses `in-progress` specs.
        #[arg(long)]
        force: bool,
        /// Emit JSON envelope (`schema_version = 1`).
        #[arg(long)]
        json: bool,
    },
}

/// `speccy task` subcommands.
#[derive(Subcommand)]
enum TaskCommand {
    /// Rewrite one task's `state` attribute over the legal state graph.
    ///
    /// Resolves the selector with the same grammar `speccy check` uses,
    /// enforces the closed six-edge legal graph (same-state targets are
    /// idempotent no-ops), and splices the new state into TASKS.md
    /// byte-surgically. An illegal edge or unresolved selector exits
    /// non-zero with the file untouched.
    Transition {
        /// Task selector: `T-NNN` (unqualified) or `SPEC-NNNN/T-NNN`.
        #[arg(value_name = "SELECTOR")]
        selector: String,
        /// Target state. Only the four legal task states are accepted; an
        /// unknown value is rejected at argument-parse time.
        #[arg(long, value_name = "STATE", value_parser = parse_task_state)]
        to: speccy_core::parse::TaskState,
    },
}

/// clap value parser for `--to`: accepts only the four on-disk task
/// states, rejecting any other value at argument-parse time.
fn parse_task_state(raw: &str) -> Result<speccy_core::parse::TaskState, String> {
    speccy_core::parse::TaskState::parse(raw).ok_or_else(|| {
        format!(
            "invalid state `{raw}`; expected one of: pending, in-progress, in-review, completed",
        )
    })
}

/// `speccy journal` subcommands.
#[derive(Subcommand)]
enum JournalCommand {
    /// Append one validated block to a journal.
    ///
    /// Reads the block body from stdin and appends exactly one block to the
    /// journal the block type implies: a task block type
    /// (`implementer` / `review` / `blockers`) routes a task selector to
    /// `<spec-dir>/journal/<task-id>.md`; a vet block type (`drift-review` /
    /// `holistic-fix` / `simplifier-scan` / `simplifier-apply` / `gate`)
    /// routes a bare `SPEC-NNNN` selector to `<spec-dir>/journal/VET.md`. The
    /// file is created with frontmatter on first append. The CLI stamps
    /// `date`, derives `round`, manages vet invocation sections, and computes
    /// a `gate` block's `tasks_hash`; there is no flag to override any of
    /// these. Validation runs before any write, so a malformed block leaves
    /// the journal untouched.
    Append {
        /// Selector: `T-NNN` / `SPEC-NNNN/T-NNN` for task blocks, or a bare
        /// `SPEC-NNNN` for vet blocks.
        #[arg(value_name = "SELECTOR")]
        selector: String,
        /// Block type. One of `implementer`, `review`, `blockers`,
        /// `drift-review`, `holistic-fix`, `simplifier-scan`,
        /// `simplifier-apply`, `gate`; an unknown value is rejected at
        /// argument-parse time.
        #[arg(long, value_name = "TYPE", value_parser = parse_journal_block)]
        block: speccy_cli::journal::JournalBlock,
        /// Model identity (required for `implementer`/`review` and the
        /// round-bearing vet blocks `drift-review`/`holistic-fix`).
        #[arg(long, value_name = "STRING")]
        model: Option<String>,
        /// Reviewer persona (required for `review`).
        #[arg(long, value_name = "NAME")]
        persona: Option<String>,
        /// Verdict (required for `review` and every vet block).
        #[arg(long, value_name = "VALUE")]
        verdict: Option<String>,
    },
    /// Show a journal's frontmatter and blocks, filtered.
    ///
    /// Parses the resolved journal (a task selector resolves
    /// `<spec-dir>/journal/<task-id>.md`; a bare `SPEC-NNNN` resolves
    /// `<spec-dir>/journal/VET.md`) and emits the blocks that survive the
    /// three conjunctive filters. `--json` toggles representation, never
    /// content. A missing journal exits non-zero.
    Show {
        /// Selector: `T-NNN` / `SPEC-NNNN/T-NNN` for a task journal, or a
        /// bare `SPEC-NNNN` for VET.md.
        #[arg(value_name = "SELECTOR")]
        selector: String,
        /// Emit JSON envelope (`schema_version = 1`).
        #[arg(long)]
        json: bool,
        /// Keep only the highest round (`latest`) or a specific round (`N`).
        /// For VET.md the round dimension is scoped to the last invocation
        /// section.
        #[arg(long, value_name = "latest|N", value_parser = parse_round_filter)]
        round: Option<speccy_cli::journal_show::RoundFilter>,
        /// Keep only blocks whose verdict equals this value.
        #[arg(long, value_name = "VALUE")]
        verdict: Option<String>,
        /// Keep only blocks of this element type (e.g. `review`, `gate`).
        #[arg(long, value_name = "TYPE")]
        block: Option<String>,
    },
}

/// clap value parser for `--round`: accepts the literal `latest` or a
/// positive integer, rejecting any other value at argument-parse time.
fn parse_round_filter(raw: &str) -> Result<speccy_cli::journal_show::RoundFilter, String> {
    use speccy_cli::journal_show::RoundFilter;
    if raw == "latest" {
        return Ok(RoundFilter::Latest);
    }
    match raw.parse::<u32>() {
        Ok(n) if n >= 1 => Ok(RoundFilter::Exact(n)),
        _ => Err(format!(
            "invalid round `{raw}`; expected `latest` or a positive integer"
        )),
    }
}

/// clap value parser for `--block`: accepts the three task-journal block
/// types and the five vet-journal block types, rejecting any other value at
/// argument-parse time. The returned [`JournalBlock`] carries which journal
/// the block targets.
fn parse_journal_block(raw: &str) -> Result<speccy_cli::journal::JournalBlock, String> {
    use speccy_cli::journal::JournalBlock;
    use speccy_core::parse::TaskBlockKind;
    use speccy_core::parse::VetBlockKind;
    match raw {
        "implementer" => Ok(JournalBlock::Task(TaskBlockKind::Implementer)),
        "review" => Ok(JournalBlock::Task(TaskBlockKind::Review)),
        "blockers" => Ok(JournalBlock::Task(TaskBlockKind::Blockers)),
        "drift-review" => Ok(JournalBlock::Vet(VetBlockKind::DriftReview)),
        "holistic-fix" => Ok(JournalBlock::Vet(VetBlockKind::HolisticFix)),
        "simplifier-scan" => Ok(JournalBlock::Vet(VetBlockKind::SimplifierScan)),
        "simplifier-apply" => Ok(JournalBlock::Vet(VetBlockKind::SimplifierApply)),
        "gate" => Ok(JournalBlock::Vet(VetBlockKind::Gate)),
        other => Err(format!(
            "invalid block type `{other}`; expected one of: implementer, review, blockers, \
             drift-review, holistic-fix, simplifier-scan, simplifier-apply, gate"
        )),
    }
}

fn main() -> ExitCode {
    init_tracing();
    let cli = Cli::parse();
    ExitCode::from(dispatch(cli.command))
}

/// Initialize the process-wide `tracing` subscriber exactly once.
///
/// Diagnostics are written to **stderr** so the stable stdout / JSON
/// command-output contract is unaffected. The level is governed solely by
/// the standard `tracing-subscriber` environment filter (`RUST_LOG`),
/// defaulting to `warn` when unset — no Speccy-specific configuration knob
/// is introduced. `tracing` observes behavior; it never drives control flow.
/// A failed install is tolerated: a missing diagnostic
/// channel must never abort the command, so `try_init`'s "already
/// initialized" error is intentionally discarded.
fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(tracing::Level::WARN.to_string()));
    if tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .try_init()
        .is_err()
    {
        // A subscriber is already installed (e.g. a re-entrant call); the
        // diagnostic channel is best-effort and never fatal.
        return;
    }
    // First real diagnostic on the channel: a startup TRACE naming the
    // binary version. It is below the default `warn` level, so a normal
    // invocation never sees it; under `RUST_LOG=trace` it is emitted to
    // stderr, where it proves the stdout / JSON contract holds even with a
    // diagnostic in flight — and would corrupt stdout
    // were the subscriber ever miswired to it.
    tracing::trace!(version = env!("CARGO_PKG_VERSION"), "speccy starting");
}

fn dispatch(command: Command) -> u8 {
    match command {
        Command::Init { force, host } => run_init(host, force),
        Command::Status {
            selector,
            all,
            include_archive,
            json,
        } => run_status(selector, all, include_archive, json),
        Command::Next {
            spec_id,
            include_archive,
            json,
        } => run_next(spec_id, include_archive, json),
        Command::Check {
            selector,
            include_archive,
        } => run_check(selector, include_archive),
        Command::Context { selector, json } => run_context(selector, json),
        Command::Verify {
            include_archive,
            json,
        } => run_verify(include_archive, json),
        Command::Lock { spec_id } => run_lock(spec_id),
        Command::Vacancy { json } => run_vacancy(json),
        Command::Task { command } => match command {
            TaskCommand::Transition { selector, to } => run_transition(selector, to),
        },
        Command::Journal { command } => match command {
            JournalCommand::Append {
                selector,
                block,
                model,
                persona,
                verdict,
            } => run_journal_append(selector, block, model, persona, verdict),
            JournalCommand::Show {
                selector,
                json,
                round,
                verdict,
                block,
            } => run_journal_show(selector, json, round, verdict, block),
        },
        Command::Archive {
            spec_id,
            reason,
            force,
            json,
        } => run_archive(spec_id, reason, force, json),
    }
}

fn run_transition(selector: String, to: speccy_core::parse::TaskState) -> u8 {
    use speccy_cli::transition::TransitionError;

    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy task transition: {e}");
            return 2;
        }
    };
    let result = speccy_cli::transition::run(
        speccy_cli::transition::TransitionArgs { selector, to },
        &cwd,
    );
    match result {
        Ok(()) => 0,
        Err(TransitionError::TaskLookup(e)) => {
            report_lookup_error("task transition", " --to <state>", &e)
        }
        Err(e) => {
            eprintln!("speccy task transition: {e}");
            1
        }
    }
}

fn run_journal_append(
    selector: String,
    block: speccy_cli::journal::JournalBlock,
    model: Option<String>,
    persona: Option<String>,
    verdict: Option<String>,
) -> u8 {
    use speccy_cli::journal::JournalError;

    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy journal append: {e}");
            return 2;
        }
    };
    let mut stdin = std::io::stdin().lock();
    let result = speccy_cli::journal::run(
        speccy_cli::journal::AppendArgs {
            selector,
            block,
            model,
            persona,
            verdict,
        },
        &cwd,
        &mut stdin,
    );
    match result {
        Ok(()) => 0,
        Err(JournalError::TaskLookup(e)) => {
            report_lookup_error("journal append", " --block <type>", &e)
        }
        Err(e) => {
            eprintln!("speccy journal append: {e}");
            1
        }
    }
}

fn run_journal_show(
    selector: String,
    json: bool,
    round: Option<speccy_cli::journal_show::RoundFilter>,
    verdict: Option<String>,
    block: Option<String>,
) -> u8 {
    use speccy_cli::journal_show::ShowError;

    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy journal show: {e}");
            return 2;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::journal_show::run(
        speccy_cli::journal_show::ShowArgs {
            selector,
            json,
            round,
            verdict,
            block,
        },
        &cwd,
        &mut stdout,
    );
    flush_best_effort(&mut stdout);
    match result {
        Ok(()) => 0,
        Err(ShowError::TaskLookup(e)) => report_lookup_error("journal show", "", &e),
        Err(e) => {
            eprintln!("speccy journal show: {e}");
            1
        }
    }
}

fn run_archive(spec_id: String, reason: Option<String>, force: bool, json: bool) -> u8 {
    use speccy_cli::archive::ArchiveError;

    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy archive: {e}");
            return 2;
        }
    };
    let result = speccy_cli::archive::run(
        speccy_cli::archive::ArchiveArgs {
            spec_id,
            reason,
            force,
        },
        &cwd,
    );
    match result {
        Ok(outcome) => {
            // Emit one warning line per orphan candidate to stderr, in
            // both text and JSON modes.
            for orphan in &outcome.orphan_warnings {
                eprintln!(
                    "warning: archiving {archiving} will orphan {orphan} ({orphan} has status: superseded and no other active spec declares supersedes: [{orphan}]; SPC-006 will fire on {orphan} after the move).",
                    archiving = outcome.spec_id,
                    orphan = orphan,
                );
            }
            if json {
                let receipt = speccy_cli::archive::ArchiveReceipt::from_outcome(&outcome);
                match serde_json::to_string(&receipt) {
                    Ok(s) => println!("{s}"),
                    Err(e) => {
                        eprintln!("speccy archive: failed to serialize receipt: {e}");
                        return 1;
                    }
                }
            } else {
                println!(
                    "archived {id}: {from} -> {to} (archived_at: {date})",
                    id = outcome.spec_id,
                    from = outcome.from,
                    to = outcome.to,
                    date = outcome.archived_at,
                );
            }
            0
        }
        Err(e @ ArchiveError::InvalidSpecIdFormat { .. }) => {
            eprintln!("speccy archive: {e}");
            2
        }
        Err(e) => {
            eprintln!("speccy archive: {e}");
            1
        }
    }
}

fn run_init(host: Option<String>, force: bool) -> u8 {
    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy init: {e}");
            return 2;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let result = speccy_cli::init::run(
        speccy_cli::init::InitArgs { host, force },
        &cwd,
        &mut stdout,
        &mut stderr,
    );
    flush_best_effort(&mut stdout);
    flush_best_effort(&mut stderr);
    match result {
        Ok(()) => 0,
        Err(
            e @ (speccy_cli::init::InitError::FilesConflict { .. }
            | speccy_cli::init::InitError::UnknownHost { .. }
            | speccy_cli::init::InitError::CursorDetected),
        ) => {
            eprintln!("speccy init: {e}");
            1
        }
        Err(e) => {
            eprintln!("speccy init: {e}");
            2
        }
    }
}

fn run_status(selector: Option<String>, all: bool, include_archive: bool, json: bool) -> u8 {
    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy status: {e}");
            return 2;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::status::run(
        &speccy_cli::status::StatusArgs {
            selector,
            all,
            include_archive,
            json,
        },
        &cwd,
        &mut stdout,
    );
    flush_best_effort(&mut stdout);
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("speccy status: {e}");
            1
        }
    }
}

fn run_next(spec_id: Option<String>, include_archive: bool, json: bool) -> u8 {
    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy next: {e}");
            return 2;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let result = speccy_cli::next::run(
        &speccy_cli::next::NextArgs {
            spec_id,
            include_archive,
            json,
        },
        &cwd,
        &mut stdout,
        &mut stderr,
    );
    flush_best_effort(&mut stdout);
    flush_best_effort(&mut stderr);
    match result {
        Ok(code) => u8::try_from(code).unwrap_or(1),
        Err(e) => {
            eprintln!("speccy next: {e}");
            1
        }
    }
}

fn run_check(selector: Option<String>, include_archive: bool) -> u8 {
    use speccy_cli::check::CheckError;

    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy check: {e}");
            return 2;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let result = speccy_cli::check::run(
        speccy_cli::check::CheckArgs {
            selector,
            include_archive,
        },
        &cwd,
        &mut stdout,
        &mut stderr,
    );
    flush_best_effort(&mut stdout);
    match result {
        Ok(code) => clamp_exit(code),
        Err(CheckError::TaskLookup(e)) => report_lookup_error("check", "", &e),
        Err(e) => {
            eprintln!("speccy check: {e}");
            1
        }
    }
}

fn run_context(selector: String, json: bool) -> u8 {
    use speccy_cli::context::ContextError;

    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy context: {e}");
            return 2;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::context::run(
        speccy_cli::context::ContextArgs { selector, json },
        &cwd,
        &mut stdout,
    );
    flush_best_effort(&mut stdout);
    match result {
        Ok(()) => 0,
        // Route selector failures through the shared helper so the
        // diagnostic class matches `speccy check`.
        Err(ContextError::TaskLookup(e)) => report_lookup_error("context", "", &e),
        Err(e) => {
            eprintln!("speccy context: {e}");
            1
        }
    }
}

/// Render a task-selector `LookupError` to stderr for one of the
/// selector-taking commands and return exit code 1.
///
/// `cmd` is the command name used as the message prefix and in the
/// disambiguation examples (e.g. `"task transition"`). `disambig_suffix`
/// is appended after the `SPEC-NNNN/T-NNN` selector in each
/// disambiguation example line (e.g. `" --to <state>"`; `""` for
/// commands that take no trailing argument).
fn report_lookup_error(
    cmd: &str,
    disambig_suffix: &str,
    err: &speccy_core::task_lookup::LookupError,
) -> u8 {
    use speccy_core::task_lookup::LookupError;
    match err {
        LookupError::InvalidFormat { arg } => {
            eprintln!("speccy {cmd}: invalid task reference `{arg}`");
            eprintln!("  expected `T-NNN` (unqualified) or `SPEC-NNNN/T-NNN` (qualified)");
        }
        LookupError::NotFound { task_ref } => {
            eprintln!("speccy {cmd}: task `{task_ref}` not found in any spec");
            eprintln!("  run `speccy status` to list specs and their tasks");
        }
        LookupError::Ambiguous {
            task_id,
            candidate_specs,
        } => {
            eprintln!(
                "speccy {cmd}: {task_id} is ambiguous; matches in {count} specs.",
                count = candidate_specs.len(),
            );
            eprintln!("Disambiguate with one of:");
            for spec_id in candidate_specs {
                eprintln!("  speccy {cmd} {spec_id}/{task_id}{disambig_suffix}");
            }
        }
        other => eprintln!("speccy {cmd}: {other}"),
    }
    1
}

fn clamp_exit(code: i32) -> u8 {
    if code == 0 {
        0
    } else {
        u8::try_from(code).unwrap_or(1)
    }
}

/// Flush a stdio stream, discarding errors from a closed pipe so
/// `speccy <cmd> | head` does not crash the program.
fn flush_best_effort<W: std::io::Write>(stream: &mut W) {
    if stream.flush().is_err() {
        // stream closed; nothing more to do.
    }
}

fn run_lock(spec_id: String) -> u8 {
    use speccy_cli::lock::LockError;

    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy lock: {e}");
            return 2;
        }
    };
    match speccy_cli::lock::run(speccy_cli::lock::LockArgs { spec_id }, &cwd) {
        Ok(()) => 0,
        Err(e @ LockError::InvalidSpecIdFormat { .. }) => {
            eprintln!("speccy lock: {e}");
            2
        }
        Err(e) => {
            eprintln!("speccy lock: {e}");
            1
        }
    }
}

fn run_vacancy(json: bool) -> u8 {
    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy vacancy: {e}");
            return 2;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::vacancy::run(
        &speccy_cli::vacancy::VacancyArgs { json },
        &cwd,
        &mut stdout,
    );
    flush_best_effort(&mut stdout);
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("speccy vacancy: {e}");
            1
        }
    }
}

fn run_verify(include_archive: bool, json: bool) -> u8 {
    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy verify: {e}");
            return 2;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::verify::run(
        speccy_cli::verify::VerifyArgs {
            include_archive,
            json,
        },
        &cwd,
        &mut stdout,
    );
    flush_best_effort(&mut stdout);
    match result {
        Ok(code) => clamp_exit(code),
        Err(e) => {
            eprintln!("speccy verify: {e}");
            1
        }
    }
}
