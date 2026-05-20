//! Speccy CLI entry point.
//!
//! Thin dispatcher over command modules in `speccy_cli`. `clap` derives
//! parse the argv; each match arm resolves cwd, calls the library
//! function, and maps library errors to documented process exit codes.

use clap::Parser;
use clap::Subcommand;
use std::io::Write as _;
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
    Status {
        /// `SPEC-NNNN` to render exactly one spec, unfiltered. Cannot
        /// be combined with `--all`. Omit to use the default
        /// attention-list view (or pass `--all`).
        #[arg(value_name = "SELECTOR", conflicts_with = "all")]
        selector: Option<String>,
        /// Render every spec in workspace order, unfiltered. Cannot
        /// be combined with a positional `SPEC-NNNN` selector.
        #[arg(long)]
        all: bool,
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
            json,
        } => run_status(selector, all, json),
        Command::Next { spec_id, json } => run_next(spec_id, json),
        Command::Check { selector } => run_check(selector),
        Command::Verify { json } => run_verify(json),
        Command::Lock { spec_id } => run_lock(spec_id),
        Command::Vacancy { json } => run_vacancy(json),
    }
}

fn run_init(host: Option<String>, force: bool) -> u8 {
    let cwd = match speccy_cli::init::resolve_cwd() {
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
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    if stderr.flush().is_err() {
        // stderr closed; nothing more to do.
    }
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

fn run_status(selector: Option<String>, all: bool, json: bool) -> u8 {
    let cwd = match speccy_cli::status::resolve_cwd() {
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
            json,
        },
        &cwd,
        &mut stdout,
    );
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("speccy status: {e}");
            1
        }
    }
}

fn run_next(spec_id: Option<String>, json: bool) -> u8 {
    let cwd = match speccy_cli::next::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy next: {e}");
            return 1;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::next::run(
        &speccy_cli::next::NextArgs { spec_id, json },
        &cwd,
        &mut stdout,
    );
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("speccy next: {e}");
            1
        }
    }
}

fn run_check(selector: Option<String>) -> u8 {
    use speccy_cli::check::CheckError;
    use speccy_core::task_lookup::LookupError;

    let cwd = match speccy_cli::check::resolve_cwd() {
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
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
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

fn run_lock(spec_id: String) -> u8 {
    use speccy_cli::lock::LockError;

    let cwd = match speccy_cli::lock::resolve_cwd() {
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
    let cwd = match speccy_cli::vacancy::resolve_cwd() {
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
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("speccy vacancy: {e}");
            1
        }
    }
}

fn run_verify(json: bool) -> u8 {
    let cwd = match speccy_cli::verify::resolve_cwd() {
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
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    if stderr.flush().is_err() {
        // stderr closed; nothing more to do.
    }
    match result {
        Ok(code) => clamp_exit(code),
        Err(e) => {
            eprintln!("speccy verify: {e}");
            1
        }
    }
}
