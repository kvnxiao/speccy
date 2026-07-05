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
            eprintln!("speccy: export spec / run-bundle arrive in a later milestone");
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
            eprintln!("speccy: {e}");
            return ExitCode::FAILURE;
        }
    };
    let cwd = match camino::Utf8PathBuf::from_path_buf(cwd) {
        Ok(d) => d,
        Err(p) => {
            eprintln!("speccy: current directory {} is not UTF-8", p.display());
            return ExitCode::FAILURE;
        }
    };
    let repo_root = match speccy_core::gitx::toplevel(&cwd) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("speccy: {}", e.message);
            return ExitCode::FAILURE;
        }
    };
    match run(&repo_root, &opts) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("speccy: {}", e.message);
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
        println!("git    OK  ({v})");
    } else {
        println!("git    MISSING — install git");
        ok = false;
    }

    match Store::open() {
        Ok(store) => println!(
            "store  OK  ({} writable; workspace {})",
            store.home, store.workspace_id
        ),
        Err(e) => println!("store  WARN  ({})", e.message),
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
                Ok(msg) => println!("packs  OK  ({msg})"),
                Err(e) => {
                    println!("packs  DRIFT  ({})", e.message.lines().next().unwrap_or(""));
                    ok = false;
                }
            }
        }
        Ok(_) => println!("packs  none (run `speccy install`)"),
        Err(_) => println!("packs  n/a  (not in a git repository)"),
    }

    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
