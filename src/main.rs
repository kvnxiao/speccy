//! `speccy` binary entry point.
//!
//! Parses the CLI, dispatches, and renders the response. Controller (`ctl`)
//! operations always emit the JSON envelope to stdout; a failed operation
//! still prints its envelope and exits nonzero so shells can branch on it.

use std::process::ExitCode;

use speccy::cli::{Cli, Command};
use speccy::error::{envelope, Result};
use speccy::ops;

use clap::Parser;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Ctl(command) => emit_envelope(ops::dispatch(command)),
        Command::Doctor => doctor(),
        // Human-facing commands arrive in later milestones (M1/M3).
        _ => {
            eprintln!("speccy: this command is not implemented yet");
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
