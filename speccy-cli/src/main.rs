//! Speccy CLI entry point.
//!
//! Thin dispatcher over `speccy_cli::status::run`. Future commands land
//! alongside; until then, only `speccy status [--json]` is wired.

use camino::Utf8PathBuf;
use std::io::Write as _;
use std::process::ExitCode;

const USAGE: &str = "speccy status [--json]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match dispatch(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(code),
    }
}

fn dispatch(args: &[String]) -> Result<(), u8> {
    let mut iter = args.iter();
    let Some(command) = iter.next() else {
        eprintln!("speccy: no command given");
        eprintln!("usage: {USAGE}");
        return Err(2);
    };

    match command.as_str() {
        "status" => run_status(iter.as_slice()),
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            Ok(())
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
                eprintln!("usage: {USAGE}");
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
