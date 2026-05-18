//! CLI entry for the private SPEC-0029 migration tool.
//!
//! Usage: `migrate-tasks-schema <PATH>...`
//!
//! Each `<PATH>` is migrated in place. The tool prints one line per
//! file describing the outcome (`migrated` or `no change`). Exit code
//! is 0 on success and 1 on any error.

use camino::Utf8PathBuf;
use migrate_tasks_schema::Outcome;
use migrate_tasks_schema::migrate_file;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: migrate-tasks-schema <TASKS.md path>...");
        eprintln!();
        eprintln!(
            "Converts each TASKS.md from the legacy markdown-bullet conventions to the new XML schema (SPEC-0029)."
        );
        return ExitCode::from(2);
    }

    let mut had_error = false;
    for raw in args {
        let path = Utf8PathBuf::from(raw);
        match migrate_file(&path) {
            Ok(Outcome::Unchanged) => println!("{path}: no change"),
            Ok(Outcome::Migrated) => println!("{path}: migrated"),
            Err(err) => {
                eprintln!("{path}: {err}");
                had_error = true;
            }
        }
    }

    if had_error {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
