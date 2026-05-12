//! Speccy CLI entry point.
//!
//! The CLI surface lands in SPEC-0002 onward. Until then, this binary
//! exits with a clear marker so users invoking it accidentally know it is
//! a work-in-progress.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("speccy CLI; no commands implemented yet");
    ExitCode::from(2)
}
