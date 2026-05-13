//! Speccy CLI entry point.
//!
//! Thin dispatcher over command modules in `speccy_cli`. Each `run_*`
//! helper resolves cwd, parses arguments, invokes the library function,
//! and translates errors into the documented process exit codes.

use camino::Utf8PathBuf;
use std::io::Write as _;
use std::process::ExitCode;

const USAGE: &str = "speccy <init|plan|tasks|status|check|verify> [args]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Ok(code) | Err(code)) = dispatch(&args);
    ExitCode::from(code)
}

fn dispatch(args: &[String]) -> Result<u8, u8> {
    let mut iter = args.iter();
    let Some(command) = iter.next() else {
        eprintln!("speccy: no command given");
        eprintln!("usage: {USAGE}");
        return Err(2);
    };

    match command.as_str() {
        "init" => run_init(iter.as_slice()).map(|()| 0_u8),
        "plan" => run_plan(iter.as_slice()).map(|()| 0_u8),
        "tasks" => run_tasks(iter.as_slice()).map(|()| 0_u8),
        "status" => run_status(iter.as_slice()).map(|()| 0_u8),
        "check" => run_check(iter.as_slice()),
        "verify" => run_verify(iter.as_slice()),
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            Ok(0)
        }
        other => {
            eprintln!("speccy: unknown command `{other}`");
            eprintln!("usage: {USAGE}");
            Err(2)
        }
    }
}

fn run_init(args: &[String]) -> Result<(), u8> {
    let mut host: Option<String> = None;
    let mut force = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("usage: speccy init [--host <name>] [--force]");
                return Ok(());
            }
            "--force" => force = true,
            "--host" => {
                let Some(value) = iter.next() else {
                    eprintln!("speccy init: --host requires a value");
                    eprintln!("usage: speccy init [--host <name>] [--force]");
                    return Err(2);
                };
                host = Some(value.clone());
            }
            other if other.starts_with("--host=") => {
                let value = other.strip_prefix("--host=").unwrap_or("");
                host = Some(value.to_owned());
            }
            other => {
                eprintln!("speccy init: unexpected argument `{other}`");
                eprintln!("usage: speccy init [--host <name>] [--force]");
                return Err(2);
            }
        }
    }

    let cwd = match speccy_cli::init::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy init: {e}");
            return Err(2);
        }
    };
    invoke_init(&cwd, host, force)
}

fn invoke_init(cwd: &Utf8PathBuf, host: Option<String>, force: bool) -> Result<(), u8> {
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let result = speccy_cli::init::run(
        speccy_cli::init::InitArgs { host, force },
        cwd,
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
        Ok(()) => Ok(()),
        Err(
            e @ (speccy_cli::init::InitError::WorkspaceExists { .. }
            | speccy_cli::init::InitError::UnknownHost { .. }
            | speccy_cli::init::InitError::CursorDetected),
        ) => {
            eprintln!("speccy init: {e}");
            Err(1)
        }
        Err(e) => {
            eprintln!("speccy init: {e}");
            Err(2)
        }
    }
}

fn run_plan(args: &[String]) -> Result<(), u8> {
    let mut spec_id: Option<String> = None;
    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("usage: speccy plan [SPEC-ID]");
                return Ok(());
            }
            other if other.starts_with("--") => {
                eprintln!("speccy plan: unknown flag `{other}`");
                eprintln!("usage: speccy plan [SPEC-ID]");
                return Err(2);
            }
            positional if spec_id.is_none() => spec_id = Some(positional.to_owned()),
            extra => {
                eprintln!("speccy plan: unexpected extra argument `{extra}`");
                eprintln!("usage: speccy plan [SPEC-ID]");
                return Err(2);
            }
        }
    }

    let cwd = match speccy_cli::plan::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy plan: {e}");
            return Err(1);
        }
    };
    invoke_plan(&cwd, spec_id)
}

fn invoke_plan(cwd: &Utf8PathBuf, spec_id: Option<String>) -> Result<(), u8> {
    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::plan::run(speccy_cli::plan::PlanArgs { spec_id }, cwd, &mut stdout);
    if stdout.flush().is_err() {
        // stdout closed; nothing more to do.
    }
    match result {
        Ok(()) => Ok(()),
        Err(e @ speccy_cli::plan::PlanError::InvalidSpecIdFormat { .. }) => {
            eprintln!("speccy plan: {e}");
            Err(2)
        }
        Err(e) => {
            eprintln!("speccy plan: {e}");
            Err(1)
        }
    }
}

fn run_tasks(args: &[String]) -> Result<(), u8> {
    let mut spec_id: Option<String> = None;
    let mut commit = false;
    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("usage: speccy tasks SPEC-ID [--commit]");
                return Ok(());
            }
            "--commit" => commit = true,
            other if other.starts_with("--") => {
                eprintln!("speccy tasks: unknown flag `{other}`");
                eprintln!("usage: speccy tasks SPEC-ID [--commit]");
                return Err(2);
            }
            positional if spec_id.is_none() => spec_id = Some(positional.to_owned()),
            extra => {
                eprintln!("speccy tasks: unexpected extra argument `{extra}`");
                eprintln!("usage: speccy tasks SPEC-ID [--commit]");
                return Err(2);
            }
        }
    }

    let Some(id) = spec_id else {
        eprintln!("speccy tasks: missing required SPEC-ID argument");
        eprintln!("usage: speccy tasks SPEC-ID [--commit]");
        return Err(2);
    };

    let cwd = match speccy_cli::tasks::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy tasks: {e}");
            return Err(1);
        }
    };
    invoke_tasks(&cwd, id, commit)
}

fn invoke_tasks(cwd: &Utf8PathBuf, spec_id: String, commit: bool) -> Result<(), u8> {
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
        Ok(()) => Ok(()),
        Err(e @ speccy_cli::tasks::TasksError::InvalidSpecIdFormat { .. }) => {
            eprintln!("speccy tasks: {e}");
            Err(2)
        }
        Err(speccy_cli::tasks::TasksError::Commit(inner)) => {
            eprintln!("speccy tasks: --commit failed: {inner}");
            Err(1)
        }
        Err(speccy_cli::tasks::TasksError::Parse {
            artifact,
            id,
            source,
        }) => {
            eprintln!("speccy tasks: failed to parse {artifact} for {id}: {source}");
            Err(1)
        }
        Err(e) => {
            eprintln!("speccy tasks: {e}");
            Err(1)
        }
    }
}

fn run_status(args: &[String]) -> Result<(), u8> {
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            other => {
                eprintln!("speccy status: unknown argument `{other}`");
                eprintln!("usage: speccy status [--json]");
                return Err(2);
            }
        }
    }

    let cwd = match speccy_cli::status::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy status: {e}");
            return Err(1);
        }
    };
    invoke_status(&cwd, json)
}

fn invoke_status(cwd: &Utf8PathBuf, json: bool) -> Result<(), u8> {
    let mut stdout = std::io::stdout().lock();
    let result = speccy_cli::status::run(speccy_cli::status::StatusArgs { json }, cwd, &mut stdout);
    if stdout.flush().is_err() {
        // stdout is closed; nothing useful to do.
    }
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("speccy status: {e}");
            Err(1)
        }
    }
}

fn run_check(args: &[String]) -> Result<u8, u8> {
    let mut id: Option<String> = None;
    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("usage: speccy check [CHK-ID]");
                return Ok(0);
            }
            other if other.starts_with("--") => {
                eprintln!("speccy check: unknown flag `{other}`");
                eprintln!("usage: speccy check [CHK-ID]");
                return Err(2);
            }
            positional if id.is_none() => id = Some(positional.to_owned()),
            extra => {
                eprintln!("speccy check: unexpected extra argument `{extra}`");
                eprintln!("usage: speccy check [CHK-ID]");
                return Err(2);
            }
        }
    }

    let cwd = match speccy_cli::check::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy check: {e}");
            return Err(1);
        }
    };
    invoke_check(&cwd, id)
}

fn invoke_check(cwd: &Utf8PathBuf, id: Option<String>) -> Result<u8, u8> {
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let result = speccy_cli::check::run(
        speccy_cli::check::CheckArgs { id },
        cwd,
        &mut stdout,
        &mut stderr,
    );
    if stdout.flush().is_err() {
        // stdout is closed; nothing useful to do.
    }
    match result {
        Ok(code) => Ok(clamp_exit(code)),
        Err(speccy_cli::check::CheckError::ChildSpawn { check_id, source }) => {
            eprintln!("speccy check: failed to spawn shell for {check_id}: {source}");
            Err(2)
        }
        Err(e) => {
            eprintln!("speccy check: {e}");
            Err(1)
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

fn run_verify(args: &[String]) -> Result<u8, u8> {
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "--help" | "-h" => {
                println!("usage: speccy verify [--json]");
                return Ok(0);
            }
            other => {
                eprintln!("speccy verify: unknown argument `{other}`");
                eprintln!("usage: speccy verify [--json]");
                return Err(2);
            }
        }
    }

    let cwd = match speccy_cli::verify::resolve_cwd() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("speccy verify: {e}");
            return Err(1);
        }
    };
    invoke_verify(&cwd, json)
}

fn invoke_verify(cwd: &Utf8PathBuf, json: bool) -> Result<u8, u8> {
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let result = speccy_cli::verify::run(
        speccy_cli::verify::VerifyArgs { json },
        cwd,
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
        Ok(code) => Ok(clamp_exit(code)),
        Err(e) => {
            eprintln!("speccy verify: {e}");
            Err(1)
        }
    }
}
