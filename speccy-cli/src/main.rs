//! Speccy CLI entry point.
//!
//! Thin dispatcher over command modules in `speccy_cli`. `clap` derives
//! parse the argv; each match arm resolves cwd, calls the library
//! function, and maps library errors to documented process exit codes.

use camino::Utf8PathBuf;
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
    /// Render the Phase 1 planning prompt (greenfield or amendment).
    Plan {
        /// Optional `SPEC-NNNN` to amend; omit for greenfield.
        spec_id: Option<String>,
    },
    /// Render the Phase 2 tasks prompt or commit the spec hash.
    Tasks {
        /// `SPEC-NNNN` to operate on.
        spec_id: Option<String>,
        /// Rewrite TASKS.md frontmatter with the current SPEC.md sha256.
        #[arg(long)]
        commit: bool,
    },
    /// Render the Phase 3 implementer prompt for a task.
    Implement {
        /// `T-NNN` (searches all specs) or `SPEC-NNNN/T-NNN` (qualified).
        task_ref: Option<String>,
    },
    /// Render the Phase 4 reviewer prompt for one persona.
    Review {
        /// `T-NNN` (searches all specs) or `SPEC-NNNN/T-NNN` (qualified).
        task_ref: Option<String>,
        /// Reviewer persona. One of: business, tests, security, style,
        /// architecture, docs
        #[arg(long, value_name = "NAME")]
        persona: Option<String>,
    },
    /// Render the Phase 5 report prompt for a completed spec.
    Report {
        /// `SPEC-NNNN` to report on.
        spec_id: Option<String>,
    },
    /// Print workspace overview (text by default; `--json` for envelope).
    Status {
        /// Emit JSON envelope (`schema_version = 1`).
        #[arg(long)]
        json: bool,
    },
    /// Pick the next actionable task across the workspace.
    Next {
        /// Restrict to one kind. Omit for default priority.
        #[arg(long, value_name = "KIND")]
        kind: Option<String>,
        /// Emit JSON envelope (`schema_version = 1`).
        #[arg(long)]
        json: bool,
    },
    /// Run command-form proofs from spec.toml.
    Check {
        /// Polymorphic selector; omit to run every check across every spec.
        /// Accepted shapes: `SPEC-NNNN` (all checks in spec),
        /// `SPEC-NNNN/CHK-NNN` (one spec-qualified check),
        /// `SPEC-NNNN/T-NNN` (checks proving the task's covered
        /// requirements), `CHK-NNN` (every spec's `CHK-NNN`), or
        /// `T-NNN` (unqualified task).
        #[arg(value_name = "SELECTOR")]
        selector: Option<String>,
    },
    /// CI gate: lint + check execution with a binary exit code.
    Verify {
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
        Command::Plan { spec_id } => run_plan(spec_id),
        Command::Tasks { spec_id, commit } => run_tasks(spec_id, commit),
        Command::Implement { task_ref } => run_implement(task_ref),
        Command::Review { task_ref, persona } => run_review(task_ref, persona),
        Command::Report { spec_id } => run_report(spec_id),
        Command::Status { json } => run_status(json),
        Command::Next { kind, json } => run_next(kind.as_deref(), json),
        Command::Check { selector } => run_check(selector),
        Command::Verify { json } => run_verify(json),
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
            e @ (speccy_cli::init::InitError::WorkspaceExists { .. }
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

fn run_plan(spec_id: Option<String>) -> u8 {
    let cwd = match speccy_cli::plan::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy plan: {e}");
            return 1;
        }
    };
    invoke_plan(&cwd, spec_id)
}

fn invoke_plan(cwd: &Utf8PathBuf, spec_id: Option<String>) -> u8 {
    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::plan::run(speccy_cli::plan::PlanArgs { spec_id }, cwd, &mut stdout);
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    match result {
        Ok(()) => 0,
        Err(e @ speccy_cli::plan::PlanError::InvalidSpecIdFormat { .. }) => {
            eprintln!("speccy plan: {e}");
            2
        }
        Err(e) => {
            eprintln!("speccy plan: {e}");
            1
        }
    }
}

fn run_tasks(spec_id: Option<String>, commit: bool) -> u8 {
    let Some(id) = spec_id else {
        eprintln!("speccy tasks: missing required SPEC-ID argument");
        eprintln!("usage: speccy tasks SPEC-ID [--commit]");
        return 2;
    };

    let cwd = match speccy_cli::tasks::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy tasks: {e}");
            return 1;
        }
    };
    invoke_tasks(&cwd, id, commit)
}

fn invoke_tasks(cwd: &Utf8PathBuf, spec_id: String, commit: bool) -> u8 {
    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::tasks::run(
        speccy_cli::tasks::TasksArgs { spec_id, commit },
        cwd,
        &mut stdout,
    );
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    match result {
        Ok(()) => 0,
        Err(e @ speccy_cli::tasks::TasksError::InvalidSpecIdFormat { .. }) => {
            eprintln!("speccy tasks: {e}");
            2
        }
        Err(speccy_cli::tasks::TasksError::Commit(inner)) => {
            eprintln!("speccy tasks: --commit failed: {inner}");
            1
        }
        Err(speccy_cli::tasks::TasksError::Parse {
            artifact,
            id,
            source,
        }) => {
            eprintln!("speccy tasks: failed to parse {artifact} for {id}: {source}");
            1
        }
        Err(e) => {
            eprintln!("speccy tasks: {e}");
            1
        }
    }
}

fn run_implement(task_ref: Option<String>) -> u8 {
    let Some(arg) = task_ref else {
        eprintln!("speccy implement: missing required TASK-ID argument");
        eprintln!("usage: speccy implement TASK-ID");
        eprintln!("       TASK-ID is T-NNN (searches all specs) or SPEC-NNNN/T-NNN");
        return 2;
    };

    let cwd = match speccy_cli::implement::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy implement: {e}");
            return 1;
        }
    };
    invoke_implement(&cwd, arg)
}

fn invoke_implement(cwd: &Utf8PathBuf, task_ref: String) -> u8 {
    use speccy_cli::implement::ImplementError;
    use speccy_core::task_lookup::LookupError;

    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::implement::run(
        &speccy_cli::implement::ImplementArgs { task_ref },
        cwd,
        &mut stdout,
    );
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    match result {
        Ok(()) => 0,
        Err(ImplementError::Lookup(LookupError::InvalidFormat { arg })) => {
            eprintln!("speccy implement: invalid task reference `{arg}`");
            eprintln!("  expected `T-NNN` (unqualified) or `SPEC-NNNN/T-NNN` (qualified)");
            1
        }
        Err(ImplementError::Lookup(LookupError::NotFound { task_ref })) => {
            eprintln!("speccy implement: task `{task_ref}` not found in any spec");
            eprintln!("  run `speccy status` to list specs and their tasks");
            1
        }
        Err(ImplementError::Lookup(LookupError::Ambiguous {
            task_id,
            candidate_specs,
        })) => {
            eprintln!(
                "speccy implement: {task_id} is ambiguous; matches in {count} specs.",
                count = candidate_specs.len(),
            );
            eprintln!("Disambiguate with one of:");
            for spec_id in &candidate_specs {
                eprintln!("  speccy implement {spec_id}/{task_id}");
            }
            1
        }
        Err(ImplementError::Prompt(e)) => {
            eprintln!("speccy implement: prompt template error: {e}");
            2
        }
        Err(e) => {
            eprintln!("speccy implement: {e}");
            1
        }
    }
}

fn run_review(task_ref: Option<String>, persona: Option<String>) -> u8 {
    let Some(arg) = task_ref else {
        eprintln!("speccy review: missing required TASK-ID argument");
        eprintln!("usage: speccy review TASK-ID --persona <name>");
        return 2;
    };

    let Some(p) = persona else {
        eprintln!("speccy review: missing required --persona <name>");
        eprintln!("usage: speccy review TASK-ID --persona <name>");
        eprintln!(
            "  valid personas: {}",
            speccy_core::personas::ALL.join(", "),
        );
        return 1;
    };

    let cwd = match speccy_cli::review::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy review: {e}");
            return 1;
        }
    };
    invoke_review(&cwd, arg, p)
}

fn invoke_review(cwd: &Utf8PathBuf, task_ref: String, persona: String) -> u8 {
    use speccy_cli::review::ReviewError;
    use speccy_core::personas::PersonaError;
    use speccy_core::task_lookup::LookupError;

    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::review::run(
        &speccy_cli::review::ReviewArgs { task_ref, persona },
        cwd,
        &mut stdout,
    );
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    match result {
        Ok(()) => 0,
        Err(ReviewError::Lookup(LookupError::InvalidFormat { arg })) => {
            eprintln!("speccy review: invalid task reference `{arg}`");
            eprintln!("  expected `T-NNN` (unqualified) or `SPEC-NNNN/T-NNN` (qualified)");
            1
        }
        Err(ReviewError::Lookup(LookupError::NotFound { task_ref })) => {
            eprintln!("speccy review: task `{task_ref}` not found in any spec");
            eprintln!("  run `speccy status` to list specs and their tasks");
            1
        }
        Err(ReviewError::Lookup(LookupError::Ambiguous {
            task_id,
            candidate_specs,
        })) => {
            eprintln!(
                "speccy review: {task_id} is ambiguous; matches in {count} specs.",
                count = candidate_specs.len(),
            );
            eprintln!("Disambiguate with one of:");
            for spec_id in &candidate_specs {
                eprintln!("  speccy review {spec_id}/{task_id} --persona <name>");
            }
            1
        }
        Err(ReviewError::Persona(PersonaError::UnknownName { name, valid })) => {
            eprintln!("speccy review: unknown persona `{name}`");
            eprintln!("  valid personas: {}", valid.join(", "));
            1
        }
        Err(ReviewError::Prompt(e)) => {
            eprintln!("speccy review: prompt template error: {e}");
            2
        }
        Err(e) => {
            eprintln!("speccy review: {e}");
            1
        }
    }
}

fn run_report(spec_id: Option<String>) -> u8 {
    let Some(id) = spec_id else {
        eprintln!("speccy report: missing required SPEC-ID argument");
        eprintln!("usage: speccy report SPEC-ID");
        return 2;
    };

    let cwd = match speccy_cli::report::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy report: {e}");
            return 1;
        }
    };
    invoke_report(&cwd, id)
}

fn invoke_report(cwd: &Utf8PathBuf, spec_id: String) -> u8 {
    use speccy_cli::report::ReportError;

    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::report::run(
        &speccy_cli::report::ReportArgs { spec_id },
        cwd,
        &mut stdout,
    );
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    match result {
        Ok(()) => 0,
        Err(ReportError::Incomplete { id, offending }) => {
            eprintln!(
                "speccy report: {id} has incomplete tasks; all tasks must be [x] before report",
            );
            for task in &offending {
                eprintln!(
                    "  {id}: {state}",
                    id = task.id,
                    state = task.state.as_glyph()
                );
            }
            1
        }
        Err(ReportError::Parse {
            artifact,
            id,
            source,
        }) => {
            eprintln!("speccy report: failed to parse {artifact} for {id}: {source}");
            1
        }
        Err(ReportError::Prompt(e)) => {
            eprintln!("speccy report: prompt template error: {e}");
            2
        }
        Err(e) => {
            eprintln!("speccy report: {e}");
            1
        }
    }
}

fn run_status(json: bool) -> u8 {
    let cwd = match speccy_cli::status::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy status: {e}");
            return 1;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let result =
        speccy_cli::status::run(speccy_cli::status::StatusArgs { json }, &cwd, &mut stdout);
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

fn run_next(kind: Option<&str>, json: bool) -> u8 {
    let kind = match kind {
        None => None,
        Some("implement") => Some(speccy_core::next::KindFilter::Implement),
        Some("review") => Some(speccy_core::next::KindFilter::Review),
        Some(other) => {
            eprintln!("speccy next: invalid --kind `{other}` (expected `implement` or `review`)");
            eprintln!("usage: speccy next [--kind implement|review] [--json]");
            return 2;
        }
    };

    let cwd = match speccy_cli::next::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy next: {e}");
            return 1;
        }
    };
    let mut stdout = std::io::stdout().lock();
    let result =
        speccy_cli::next::run(speccy_cli::next::NextArgs { kind, json }, &cwd, &mut stdout);
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
        Err(CheckError::ChildSpawn { check_id, source }) => {
            eprintln!("speccy check: failed to spawn shell for {check_id}: {source}");
            2
        }
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
