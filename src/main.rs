//! `speccy` binary entry point.
//!
//! Parses the CLI, dispatches, and renders the response. Controller (`ctl`)
//! operations always emit the JSON envelope to stdout; a failed operation
//! still prints its envelope and exits nonzero so shells can branch on it.

use std::process::ExitCode;

use speccy::cli::{Cli, Command};
use speccy::error::{envelope, Result};
use speccy::store::Store;
use speccy::{humancli, ops};

use clap::Parser;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Ctl(command) => emit_envelope(ops::dispatch(command)),
        Command::Doctor => doctor(),
        Command::Status(_) => emit_text(humancli::status),
        Command::Review(args) => {
            emit_text(|store| humancli::review(store, args.selector.as_deref(), args.evidence))
        }
        // Remaining human-facing commands arrive at M3.
        _ => {
            eprintln!("speccy: this command is not implemented yet");
            ExitCode::FAILURE
        }
    }
}

/// Open the store, run a human command that renders text, and print it.
fn emit_text<F>(render: F) -> ExitCode
where
    F: FnOnce(&Store) -> Result<String>,
{
    let result = Store::open().and_then(|store| render(&store));
    match result {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("speccy: {}", e.message);
            ExitCode::FAILURE
        }
    }
}

/// Print a controller result as the JSON envelope and map ok/err to exit code.
fn emit_envelope<T: serde::Serialize>(result: Result<T>) -> ExitCode {
    let env = envelope(&result);
    // The envelope is small, always-serializable JSON; `to_string` cannot fail here.
    println!(
        "{}",
        serde_json::to_string(&env).unwrap_or_else(|_| "{\"ok\":false}".into())
    );
    if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// M0 stub: report basic health as plain text.
fn doctor() -> ExitCode {
    println!("speccy doctor: M0 stub — checks arrive with the store (M1) and packs (M4)");
    ExitCode::SUCCESS
}
