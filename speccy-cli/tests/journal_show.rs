#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy journal show`.
//!
//! Drives the built `speccy` binary against scratch workspaces. The
//! load-bearing scenarios: `--round latest --verdict blocking`
//! on a two-round journal with five round-2 reviews returns exactly the one
//! blocking block with `schema_version` 1; the `--block review --round N`
//! completeness call site (lists the personas that reviewed round N), the
//! VET.md spec-selector path (invocation sections and blocks appear in the
//! JSON), and the missing-file non-zero exit.

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use common::TestResult;
use common::VetTestBlock;
use common::Workspace;
use common::render_vet_block;
use common::render_vet_md;
use common::sha256_hex;
use common::spec_md_template;
use common::task_xml;
use common::tasks_md_xml;
use common::write_spec;
use serde_json::Value;

/// Build a workspace with one spec carrying a single task, returning the
/// workspace and the spec dir.
fn workspace_with_task(state: &str) -> TestResult<(Workspace, Utf8PathBuf)> {
    let ws = Workspace::new()?;
    let spec_id = "SPEC-0042";
    let tasks_md = tasks_md_xml(spec_id, &task_xml("T-001", state));
    let dir = write_spec(
        &ws.root,
        "0042-example-slug",
        &spec_md_template(spec_id, "in-progress"),
        Some(&tasks_md),
    )?;
    Ok((ws, dir))
}

/// Write a per-task journal with two rounds: round 1 has one implementer +
/// one passing review; round 2 has one implementer plus five reviews, one
/// of which (`security`) is `blocking`. Returns the journal path.
fn write_two_round_journal(spec_dir: &Utf8Path) -> TestResult<Utf8PathBuf> {
    let journal = spec_dir.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    let path = journal.join("T-001.md");
    let body = concat!(
        "---\n",
        "spec: SPEC-0042\n",
        "task: T-001\n",
        "generated_at: 2026-05-21T18:00:00Z\n",
        "---\n\n",
        "<implementer date=\"2026-05-21T18:00:00Z\" model=\"m\" round=\"1\">\n",
        "round 1 impl\n",
        "</implementer>\n\n",
        "<review date=\"2026-05-21T18:10:00Z\" model=\"m\" persona=\"tests\" verdict=\"pass\" round=\"1\">\n",
        "r1 tests\n",
        "</review>\n\n",
        "<implementer date=\"2026-05-21T19:00:00Z\" model=\"m\" round=\"2\">\n",
        "round 2 impl\n",
        "</implementer>\n\n",
        "<review date=\"2026-05-21T19:10:00Z\" model=\"m\" persona=\"business\" verdict=\"pass\" round=\"2\">\n",
        "r2 business\n",
        "</review>\n\n",
        "<review date=\"2026-05-21T19:11:00Z\" model=\"m\" persona=\"tests\" verdict=\"pass\" round=\"2\">\n",
        "r2 tests\n",
        "</review>\n\n",
        "<review date=\"2026-05-21T19:12:00Z\" model=\"m\" persona=\"security\" verdict=\"blocking\" round=\"2\">\n",
        "r2 security blocks\n",
        "</review>\n\n",
        "<review date=\"2026-05-21T19:13:00Z\" model=\"m\" persona=\"style\" verdict=\"pass\" round=\"2\">\n",
        "r2 style\n",
        "</review>\n\n",
        "<review date=\"2026-05-21T19:14:00Z\" model=\"m\" persona=\"docs\" verdict=\"pass\" round=\"2\">\n",
        "r2 docs\n",
        "</review>\n",
    );
    fs_err::write(path.as_std_path(), body)?;
    Ok(path)
}

/// `--round latest --verdict blocking` on the two-round fixture
/// returns exactly the one blocking round-2 block, with the persona/verdict
/// matching the fixture and `schema_version` 1.
#[test]
fn round_latest_verdict_blocking_returns_single_block() -> TestResult {
    let (ws, spec_dir) = workspace_with_task("in-review")?;
    write_two_round_journal(&spec_dir)?;

    let out = Command::cargo_bin("speccy")?
        .args([
            "journal",
            "show",
            "SPEC-0042/T-001",
            "--json",
            "--round",
            "latest",
            "--verdict",
            "blocking",
        ])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out)?;
    assert_eq!(
        json.get("schema_version").and_then(Value::as_u64),
        Some(1),
        "schema_version must be 1",
    );
    assert_eq!(
        json.get("latest_round").and_then(Value::as_u64),
        Some(2),
        "latest round must be surfaced as 2",
    );
    let blocks = json
        .get("blocks")
        .and_then(Value::as_array)
        .expect("blocks array");
    assert_eq!(
        blocks.len(),
        1,
        "exactly one blocking block in latest round"
    );
    let block = blocks.first().expect("one block");
    assert_eq!(
        block.get("persona").and_then(Value::as_str),
        Some("security")
    );
    assert_eq!(
        block.get("verdict").and_then(Value::as_str),
        Some("blocking")
    );
    assert_eq!(block.get("round").and_then(Value::as_u64), Some(2));
    Ok(())
}

/// The completeness call site: `--block review --round N` lists the personas
/// that reviewed round N. Round 2 has five reviewers; round 1 has one.
#[test]
fn block_review_round_n_lists_round_personas() -> TestResult {
    let (ws, spec_dir) = workspace_with_task("in-review")?;
    write_two_round_journal(&spec_dir)?;

    let out = Command::cargo_bin("speccy")?
        .args([
            "journal",
            "show",
            "SPEC-0042/T-001",
            "--json",
            "--block",
            "review",
            "--round",
            "2",
        ])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out)?;
    let blocks = json
        .get("blocks")
        .and_then(Value::as_array)
        .expect("blocks array");
    let mut personas: Vec<&str> = blocks
        .iter()
        .filter_map(|b| b.get("persona").and_then(Value::as_str))
        .collect();
    personas.sort_unstable();
    assert_eq!(
        personas,
        vec!["business", "docs", "security", "style", "tests"],
        "all five round-2 reviewer personas appear; round-1 review is excluded",
    );
    // No implementer block leaks through the `--block review` filter.
    assert!(
        blocks
            .iter()
            .all(|b| b.get("block").and_then(Value::as_str) == Some("review")),
        "only review blocks survive the --block review filter",
    );
    Ok(())
}

/// A bare spec selector resolves VET.md; its invocation sections and blocks
/// appear in the JSON envelope.
#[test]
fn spec_selector_shows_vet_invocations_and_blocks() -> TestResult {
    let (ws, spec_dir) = workspace_with_task("completed")?;
    let tasks_md = tasks_md_xml("SPEC-0042", &task_xml("T-001", "completed"));
    let hash = sha256_hex(tasks_md.as_bytes());
    let journal = spec_dir.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    let drift = render_vet_block(&VetTestBlock::DriftReview {
        verdict: "pass",
        round: 1,
    })?;
    let vet = render_vet_md("SPEC-0042", "passed", &hash, None, &[drift])?;
    fs_err::write(journal.join("VET.md").as_std_path(), vet)?;

    let out = Command::cargo_bin("speccy")?
        .args(["journal", "show", "SPEC-0042", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out)?;
    assert_eq!(json.get("schema_version").and_then(Value::as_u64), Some(1));
    let invocations = json
        .get("invocations")
        .and_then(Value::as_array)
        .expect("invocations array");
    assert_eq!(invocations.len(), 1, "one invocation section");
    let inv = invocations.first().expect("one invocation");
    assert_eq!(inv.get("number").and_then(Value::as_u64), Some(1));
    let blocks = inv.get("blocks").and_then(Value::as_array).expect("blocks");
    let kinds: Vec<&str> = blocks
        .iter()
        .filter_map(|b| b.get("block").and_then(Value::as_str))
        .collect();
    assert_eq!(
        kinds,
        vec!["drift-review", "gate"],
        "both vet blocks appear"
    );
    // The gate's tasks_hash is surfaced; round-less blocks omit `round`.
    let gate = blocks.get(1).expect("gate block");
    assert_eq!(
        gate.get("tasks_hash").and_then(Value::as_str),
        Some(hash.as_str())
    );
    assert!(gate.get("round").is_none(), "gate carries no round field");
    Ok(())
}

/// A mid-vet-run VET.md whose last invocation section is still open — a
/// `drift-review` has landed but its terminal `<gate>` has not — must read
/// cleanly. The vet flow's call sites (Phase 0 invocation read-back, the
/// drift-implementer's `<drift-review>` read, the simplifier-apply's
/// `<simplifier-scan>` read) all run before the gate lands, so `journal
/// show` parses VET.md in-flight; the strict parser would reject this shape.
#[test]
fn spec_selector_shows_open_invocation_section() -> TestResult {
    let (ws, spec_dir) = workspace_with_task("completed")?;
    let journal = spec_dir.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    // No `<gate>` block: the section is open, exactly as it is mid-run after
    // a `drift-review` is appended and before the polish/gate phases.
    let vet = concat!(
        "---\n",
        "spec: SPEC-0042\n",
        "generated_at: 2026-05-22T00:00:00Z\n",
        "---\n\n",
        "## Invocation 1 — 2026-05-22T00:00:00Z\n\n",
        "<drift-review verdict=\"blocking\" round=\"1\" date=\"2026-05-22T00:01:00Z\" model=\"m\">\n",
        "drift found, no gate yet\n",
        "</drift-review>\n",
    );
    fs_err::write(journal.join("VET.md").as_std_path(), vet)?;

    let out = Command::cargo_bin("speccy")?
        .args(["journal", "show", "SPEC-0042", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out)?;
    let invocations = json
        .get("invocations")
        .and_then(Value::as_array)
        .expect("invocations array");
    assert_eq!(invocations.len(), 1, "the open invocation section is shown");
    let inv = invocations.first().expect("one invocation");
    let blocks = inv.get("blocks").and_then(Value::as_array).expect("blocks");
    let kinds: Vec<&str> = blocks
        .iter()
        .filter_map(|b| b.get("block").and_then(Value::as_str))
        .collect();
    assert_eq!(
        kinds,
        vec!["drift-review"],
        "the un-gated section's blocks read back without a terminal gate",
    );
    Ok(())
}

/// A missing journal file exits non-zero with a diagnostic.
#[test]
fn missing_journal_exits_nonzero() -> TestResult {
    let (ws, _spec_dir) = workspace_with_task("in-review")?;

    Command::cargo_bin("speccy")?
        .args(["journal", "show", "SPEC-0042/T-001", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("journal not found"));
    Ok(())
}

/// `--json` toggles representation, never content: the text form lists the
/// same filtered blocks. Pins scenario-2's text-mode counterpart so the
/// content/representation invariant is exercised in both directions.
#[test]
fn text_mode_renders_same_filtered_blocks() -> TestResult {
    let (ws, spec_dir) = workspace_with_task("in-review")?;
    write_two_round_journal(&spec_dir)?;

    let out = Command::cargo_bin("speccy")?
        .args([
            "journal",
            "show",
            "SPEC-0042/T-001",
            "--block",
            "review",
            "--round",
            "2",
        ])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out)?;
    // Five review lines for round 2, no implementer lines (filtered out).
    let review_lines = text.lines().filter(|l| l.contains("review")).count();
    assert_eq!(review_lines, 5, "five round-2 review lines in text mode");
    assert!(
        !text.contains("implementer"),
        "the --block review filter drops implementer lines in text mode too",
    );
    Ok(())
}
