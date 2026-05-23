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
        /// them in. Independent of `--all`. See SPEC-0042 REQ-007.
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
    },
    /// CI gate: proof-shape validation with a binary exit code.
    Verify {
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    ExitCode::from(dispatch(cli.command))
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
        Command::Next { spec_id, json } => run_next(spec_id, json),
        Command::Check { selector } => run_check(selector),
        Command::Verify { json } => run_verify(json),
        Command::Lock { spec_id } => run_lock(spec_id),
        Command::Vacancy { json } => run_vacancy(json),
        Command::Archive {
            spec_id,
            reason,
            force,
            json,
        } => run_archive(spec_id, reason, force, json),
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
            json,
        },
        &cwd,
    );
    match result {
        Ok(outcome) => {
            // Emit one warning line per orphan candidate to stderr, in
            // both text and JSON modes. SPEC-0042 REQ-008 / CHK-020.
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
            return 1;
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

fn run_next(spec_id: Option<String>, json: bool) -> u8 {
    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy next: {e}");
            return 1;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let result = speccy_cli::next::run(
        &speccy_cli::next::NextArgs { spec_id, json },
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

fn run_check(selector: Option<String>) -> u8 {
    use speccy_cli::check::CheckError;
    use speccy_core::task_lookup::LookupError;

    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy check: {e}");
            return 1;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let result = speccy_cli::check::run(
        speccy_cli::check::CheckArgs { selector },
        &cwd,
        &mut stdout,
        &mut stderr,
    );
    flush_best_effort(&mut stdout);
    match result {
        Ok(code) => clamp_exit(code),
        Err(CheckError::TaskLookup(LookupError::InvalidFormat { arg })) => {
            eprintln!("speccy check: invalid task reference `{arg}`");
            eprintln!("  expected `T-NNN` (unqualified) or `SPEC-NNNN/T-NNN` (qualified)");
            1
        }
        Err(CheckError::TaskLookup(LookupError::NotFound { task_ref })) => {
            eprintln!("speccy check: task `{task_ref}` not found in any spec");
            eprintln!("  run `speccy status` to list specs and their tasks");
            1
        }
        Err(CheckError::TaskLookup(LookupError::Ambiguous {
            task_id,
            candidate_specs,
        })) => {
            eprintln!(
                "speccy check: {task_id} is ambiguous; matches in {count} specs.",
                count = candidate_specs.len(),
            );
            eprintln!("Disambiguate with one of:");
            for spec_id in &candidate_specs {
                eprintln!("  speccy check {spec_id}/{task_id}");
            }
            1
        }
        Err(e) => {
            eprintln!("speccy check: {e}");
            1
        }
    }
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

fn run_verify(json: bool) -> u8 {
    let cwd = match speccy_cli::cwd::resolve() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy verify: {e}");
            return 1;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let result = speccy_cli::verify::run(
        speccy_cli::verify::VerifyArgs { json },
        &cwd,
        &mut stdout,
        &mut stderr,
    );
    flush_best_effort(&mut stdout);
    flush_best_effort(&mut stderr);
    match result {
        Ok(code) => clamp_exit(code),
        Err(e) => {
            eprintln!("speccy verify: {e}");
            1
        }
    }
}
