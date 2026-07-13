//! Current-truth consistency: the workspace's Cargo metadata agrees with
//! itself and the README, and every command advertised in `--help` has a
//! working implementation (unimplemented stubs stay hidden).

#![expect(
    clippy::expect_used,
    reason = "integration-test helpers assert on known-shape CLI/JSON output; expect is the idiomatic way a test fails and never reaches shipped code"
)]

mod common;

use common::Harness;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// `cargo metadata --no-deps` for this workspace, parsed.
fn workspace_metadata() -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root exists");
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .expect("cargo metadata runs");
    assert!(
        out.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("cargo metadata emits JSON")
}

#[test]
fn workspace_members_edition_and_msrv_agree() {
    let meta = workspace_metadata();
    let packages = meta["packages"].as_array().expect("packages array");

    let names: BTreeSet<&str> = packages
        .iter()
        .map(|p| p["name"].as_str().expect("package name"))
        .collect();
    assert_eq!(
        names,
        BTreeSet::from(["speccy-cli", "speccy-core"]),
        "workspace members changed; update this test and the docs"
    );

    let editions: BTreeSet<&str> = packages
        .iter()
        .map(|p| p["edition"].as_str().expect("edition"))
        .collect();
    assert_eq!(editions.len(), 1, "member crates disagree on edition");

    let msrvs: BTreeSet<&str> = packages
        .iter()
        .map(|p| p["rust_version"].as_str().expect("rust-version declared"))
        .collect();
    assert_eq!(msrvs.len(), 1, "member crates disagree on rust-version");
}

#[test]
fn readme_states_the_declared_edition() {
    let meta = workspace_metadata();
    let edition = meta["packages"][0]["edition"]
        .as_str()
        .expect("edition")
        .to_string();
    let readme = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root exists")
        .join("README.md");
    let text = fs_err::read_to_string(readme).expect("read README");
    assert!(
        text.contains(&format!("Rust {edition}")),
        "README does not state the declared edition (Rust {edition})"
    );
}

/// Subcommand names listed in a clap help screen (the `Commands:` block),
/// excluding the auto-generated `help`.
fn advertised(help: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut in_commands = false;
    for line in help.lines() {
        if line.trim_end() == "Commands:" {
            in_commands = true;
            continue;
        }
        if in_commands {
            if line.trim().is_empty() {
                break;
            }
            // Command rows are indented two spaces; wrapped descriptions more.
            if let Some(rest) = line.strip_prefix("  ")
                && !rest.starts_with(' ')
                && let Some(name) = rest.split_whitespace().next()
                && name != "help"
            {
                names.insert(name.to_string());
            }
        }
    }
    names
}

#[test]
fn every_advertised_command_is_implemented() {
    let h = Harness::new();
    let (help, _, ok) = h.output_full(&["--help"]);
    assert!(ok, "--help works");

    // One invocation per advertised command that reaches its implementation.
    // Failing is fine (empty workspace); claiming "not implemented" is not.
    let invocations: &[(&str, &[&str])] = &[
        ("accept", &["accept"]),
        ("archive", &["archive"]),
        ("cancel", &["cancel"]),
        (
            "ctl",
            &[
                "ctl", "run", "next", "--run", "nope", "--agent", "a", "--json",
            ],
        ),
        ("doctor", &["doctor"]),
        ("export", &["export", "review"]),
        ("install", &["install", "--dry-run"]),
        ("list", &["list"]),
        ("new", &["new", "test request"]),
        ("review", &["review"]),
        ("status", &["status"]),
    ];

    let covered: BTreeSet<String> = invocations.iter().map(|(n, _)| (*n).to_string()).collect();
    assert_eq!(
        advertised(&help),
        covered,
        "advertised commands changed; prove each new one works by adding an invocation here"
    );

    for (name, args) in invocations {
        let (stdout, stderr, _) = h.output_full(args);
        for needle in ["not implemented", "not_implemented"] {
            assert!(
                !stdout.contains(needle) && !stderr.contains(needle),
                "advertised command `{name}` is a stub: {stdout}{stderr}"
            );
        }
    }
}

#[test]
fn unimplemented_export_stubs_are_hidden_but_invocable() {
    let h = Harness::new();

    let (help, _, ok) = h.output_full(&["export", "--help"]);
    assert!(ok, "export --help works");
    assert_eq!(
        advertised(&help),
        BTreeSet::from(["review".to_string(), "run-bundle".to_string()]),
        "export must advertise only implemented subcommands"
    );

    let stub = ["export", "spec"];
    let (_, stderr, ok) = h.output_full(&stub);
    assert!(!ok, "stub {stub:?} must exit nonzero");
    assert!(
        stderr.contains("not implemented"),
        "stub {stub:?} must say it is unimplemented: {stderr}"
    );
}
