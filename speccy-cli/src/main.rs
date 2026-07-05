//! `speccy` binary entry point.
//!
//! Parses the CLI, dispatches, and renders the response. Controller (`ctl`)
//! operations always emit the JSON envelope to stdout; a failed operation
//! still prints its envelope and exits nonzero so shells can branch on it.

use crate::cli::Cli;
use crate::cli::Command;
use crate::cli::ExportCommand;
use clap::Parser;
use speccy_core::error::Result;
use speccy_core::error::envelope;
use speccy_core::store::Store;
use std::process::ExitCode;

mod cli;
mod humancli;
mod ops;
mod style;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    match cli.command {
        Command::Ctl(command) => emit_envelope(&ops::dispatch(command)),
        Command::Doctor => doctor(),
        Command::Status => emit_text(humancli::status),
        Command::Review(args) => emit_text(|store| {
            humancli::review(store, args.selector.as_deref(), args.evidence, json)
        }),
        Command::List(args) => emit_text(|store| {
            humancli::list(
                store,
                args.query.as_deref(),
                args.all,
                args.accepted,
                args.archived,
                args.status.as_deref(),
                json,
            )
        }),
        Command::Accept(args) => emit_text(|store| {
            humancli::accept(
                store,
                args.selector.as_deref(),
                args.pr.as_deref(),
                args.note.as_deref(),
            )
        }),
        Command::Archive(args) => {
            emit_text(|store| humancli::archive(store, args.selector.as_deref()))
        }
        Command::Cancel(args) => {
            emit_text(|store| humancli::cancel(store, args.selector.as_deref()))
        }
        Command::New(args) => {
            emit_text(|store| humancli::new_spec(store, &args.request, args.title.as_deref()))
        }
        Command::Export(ExportCommand::Review(args)) => emit_text(|store| {
            humancli::export_review(store, args.selector.as_deref(), args.dest.as_deref())
        }),
        Command::Export(_) => {
            anstream::eprintln!(
                "{} export spec / run-bundle arrive in a later milestone",
                style::paint(style::ERR, "speccy:")
            );
            ExitCode::FAILURE
        }
        Command::Install(args) => install(&args),
    }
}

/// `speccy install` resolves the repo root via git and renders/manages packs.
fn install(args: &crate::cli::InstallArgs) -> ExitCode {
    use speccy_core::install::InstallOptions;
    use speccy_core::install::run;
    let opts = InstallOptions {
        target: args.target.clone(),
        update: args.update,
        dry_run: args.dry_run,
        yes: args.yes,
        check: args.check,
        force: args.force,
    };
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            anstream::eprintln!("{} {e}", style::paint(style::ERR, "speccy:"));
            return ExitCode::FAILURE;
        }
    };
    let cwd = match camino::Utf8PathBuf::from_path_buf(cwd) {
        Ok(d) => d,
        Err(p) => {
            anstream::eprintln!(
                "{} current directory {} is not UTF-8",
                style::paint(style::ERR, "speccy:"),
                p.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let repo_root = match speccy_core::gitx::toplevel(&cwd) {
        Ok(root) => root,
        Err(e) => {
            anstream::eprintln!("{} {}", style::paint(style::ERR, "speccy:"), e.message);
            return ExitCode::FAILURE;
        }
    };
    match run(&repo_root, &opts) {
        Ok(report) => {
            anstream::println!("{report}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            anstream::eprintln!("{} {}", style::paint(style::ERR, "speccy:"), e.message);
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
            anstream::println!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            anstream::eprintln!("{} {}", style::paint(style::ERR, "speccy:"), e.message);
            ExitCode::FAILURE
        }
    }
}

/// Print a controller result as the JSON envelope and map ok/err to exit code.
fn emit_envelope<T: serde::Serialize>(result: &Result<T>) -> ExitCode {
    let env = envelope(result);
    // The envelope is small, always-serializable JSON; `to_string` cannot fail
    // here.
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

/// `speccy doctor` — check git, the store, and pack freshness.
fn doctor() -> ExitCode {
    use speccy_core::install::InstallOptions;
    use speccy_core::install::run;

    let mut ok = true;

    if let Some(v) = speccy_core::gitx::version() {
        anstream::println!("git    {}  ({v})", style::paint(style::OK, "OK"));
    } else {
        anstream::println!(
            "git    {} — install git",
            style::paint(style::ERR, "MISSING")
        );
        ok = false;
    }

    match Store::open() {
        Ok(store) => anstream::println!(
            "store  {}  ({} writable; workspace {})",
            style::paint(style::OK, "OK"),
            store.home,
            store.workspace_id
        ),
        Err(e) => anstream::println!(
            "store  {}  ({})",
            style::paint(style::WARN, "WARN"),
            e.message
        ),
    }

    let cwd = std::env::current_dir()
        .ok()
        .and_then(|d| camino::Utf8PathBuf::from_path_buf(d).ok())
        .unwrap_or_default();
    match speccy_core::gitx::toplevel(&cwd) {
        Ok(root) if root.join(".speccy/pack-lock.yaml").exists() => {
            let opts = InstallOptions {
                target: "auto".into(),
                update: false,
                dry_run: false,
                yes: false,
                check: true,
                force: false,
            };
            match run(&root, &opts) {
                Ok(msg) => anstream::println!("packs  {}  ({msg})", style::paint(style::OK, "OK")),
                Err(e) => {
                    anstream::println!(
                        "packs  {}  ({})",
                        style::paint(style::ERR, "DRIFT"),
                        e.message.lines().next().unwrap_or("")
                    );
                    ok = false;
                }
            }
        }
        Ok(_) => anstream::println!(
            "packs  {} (run `speccy install`)",
            style::paint(style::DIM, "none")
        ),
        Err(_) => anstream::println!(
            "packs  {}  (not in a git repository)",
            style::paint(style::DIM, "n/a")
        ),
    }

    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
