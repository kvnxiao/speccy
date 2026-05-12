//! Speccy CLI entry point.
//!
//! Thin dispatcher over command modules in `speccy_cli`. Each `run_*`
//! helper resolves cwd, parses arguments, invokes the library function,
//! and translates errors into the documented process exit codes.

use camino::Utf8PathBuf;
use std::io::Write as _;
use std::process::ExitCode;

const USAGE: &str = "speccy <status|check|verify> [args]";

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
