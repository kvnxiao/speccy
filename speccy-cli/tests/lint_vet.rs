#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for the `VET-*` lint family (SPEC-0055 REQ-007).
//!
//! `speccy verify` lints `journal/VET.md` only when the file exists. The
//! file's frontmatter status drives demotion: VET-* fires at `Level::Error`,
//! so the fixture spec is `status: implemented` everywhere a gating exit is
//! asserted (an in-progress spec would demote the error to info, matching
//! the RPT-* posture). Scenario coverage maps to T-006's `<task-scenarios>`.

mod common;

use camino::Utf8Path;
use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::write_spec;
use serde_json::Value;
use speccy_cli::verify::VerifyArgs;
use speccy_cli::verify::run;

static JSON_NULL: Value = Value::Null;

fn field<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&JSON_NULL)
}

fn at<'a>(v: &'a Value, keys: &[&str]) -> &'a Value {
    keys.iter().fold(v, |acc, k| field(acc, k))
}

fn invoke(root: &Utf8Path) -> TestResult<(i32, String)> {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        VerifyArgs {
            include_archive: false,
            json: true,
        },
        root,
        &mut out,
        &mut err,
    )?;
    Ok((code, String::from_utf8(out)?))
}

const FRONTMATTER: &str = "---\nspec: SPEC-0001\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n";

/// Write `body` to `<spec-dir>/journal/VET.md`.
fn write_vet(spec_dir: &Utf8Path, body: &str) -> TestResult {
    let journal = spec_dir.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    fs_err::write(journal.join("VET.md").as_std_path(), body)?;
    Ok(())
}

fn error_codes(json: &Value) -> Vec<String> {
    at(json, &["lint", "errors"])
        .as_array()
        .map(|errs| {
            errs.iter()
                .filter_map(|d| field(d, "code").as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// CHK-010: a `verdict="maybe"` drift-review fails the frozen grammar, so
/// `speccy verify --json` lists a VET-001 error naming the file and exits 1.
#[test]
fn out_of_domain_verdict_fires_vet_001_and_gates() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-vet001",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    let body = format!(
        "{FRONTMATTER}## Invocation 1 — 2026-05-21T18:00:00Z\n\n\
         <drift-review verdict=\"maybe\" round=\"1\" date=\"2026-05-21T18:00:00Z\" model=\"m\">\n\
         bogus verdict\n\
         </drift-review>\n\n\
         <gate verdict=\"passed\" tasks_hash=\"abc123\" date=\"2026-05-21T18:10:00Z\">\n\
         gate\n\
         </gate>\n",
    );
    write_vet(&spec_dir, &body)?;

    let (code, out) = invoke(&ws.root)?;
    assert_eq!(
        code, 1,
        "VET-001 on an implemented spec must gate; out:\n{out}"
    );

    let json: Value = serde_json::from_str(&out)?;
    let vet_001 = at(&json, &["lint", "errors"])
        .as_array()
        .expect("lint.errors array")
        .iter()
        .find(|d| field(d, "code").as_str() == Some("VET-001"))
        .expect("VET-001 must appear in lint.errors");
    let file = field(vet_001, "file").as_str().unwrap_or("");
    assert!(
        file.ends_with("VET.md"),
        "VET-001 must name the VET.md file; got: {file}",
    );
    Ok(())
}

/// A malformed-frontmatter VET.md (missing `spec`) is a grammar failure,
/// not a gate-structure one, so it routes to VET-001.
#[test]
fn missing_frontmatter_fires_vet_001() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-vet001-fm",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    let body = "## Invocation 1 — 2026-05-21T18:00:00Z\n\n\
         <gate verdict=\"passed\" tasks_hash=\"abc123\" date=\"2026-05-21T18:10:00Z\">\n\
         gate\n\
         </gate>\n";
    write_vet(&spec_dir, body)?;

    let (code, out) = invoke(&ws.root)?;
    assert_eq!(code, 1, "out:\n{out}");
    let json: Value = serde_json::from_str(&out)?;
    assert!(
        error_codes(&json).iter().any(|c| c == "VET-001"),
        "missing frontmatter must fire VET-001; got: {out}",
    );
    Ok(())
}

/// A non-final invocation section lacking a terminal `<gate>` while a later
/// section exists fires VET-002 and gates verify.
#[test]
fn non_final_section_without_gate_fires_vet_002_and_gates() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-vet002",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    let body = format!(
        "{FRONTMATTER}## Invocation 1 — 2026-05-21T18:00:00Z\n\n\
         <drift-review verdict=\"pass\" round=\"1\" date=\"2026-05-21T18:00:00Z\" model=\"m\">\n\
         first section never gated\n\
         </drift-review>\n\n\
         ## Invocation 2 — 2026-05-21T19:00:00Z\n\n\
         <drift-review verdict=\"pass\" round=\"1\" date=\"2026-05-21T19:00:00Z\" model=\"m\">\n\
         second section\n\
         </drift-review>\n\n\
         <gate verdict=\"passed\" tasks_hash=\"def456\" date=\"2026-05-21T19:10:00Z\">\n\
         gate\n\
         </gate>\n",
    );
    write_vet(&spec_dir, &body)?;

    let (code, out) = invoke(&ws.root)?;
    assert_eq!(code, 1, "VET-002 must gate; out:\n{out}");
    let json: Value = serde_json::from_str(&out)?;
    let codes = error_codes(&json);
    assert!(
        codes.iter().any(|c| c == "VET-002"),
        "missing gate in a non-final section must fire VET-002; got: {out}",
    );
    assert!(
        !codes.iter().any(|c| c == "VET-001"),
        "a gate-structure violation must route to VET-002, not VET-001; got: {out}",
    );
    Ok(())
}

/// A block following a section's terminal `<gate>` fires VET-002.
#[test]
fn block_after_gate_fires_vet_002() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-vet002-after",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    let body = format!(
        "{FRONTMATTER}## Invocation 1 — 2026-05-21T18:00:00Z\n\n\
         <gate verdict=\"passed\" tasks_hash=\"abc123\" date=\"2026-05-21T18:10:00Z\">\n\
         gate\n\
         </gate>\n\n\
         <simplifier-scan verdict=\"clean\">\n\
         after the gate\n\
         </simplifier-scan>\n",
    );
    write_vet(&spec_dir, &body)?;

    let (code, out) = invoke(&ws.root)?;
    assert_eq!(code, 1, "out:\n{out}");
    let json: Value = serde_json::from_str(&out)?;
    assert!(
        error_codes(&json).iter().any(|c| c == "VET-002"),
        "a block after the gate must fire VET-002; got: {out}",
    );
    Ok(())
}

/// A well-formed VET.md (the shape `journal append` produces) fires no
/// VET-* diagnostic.
#[test]
fn well_formed_vet_md_fires_no_vet_diagnostic() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-vet-clean",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;
    let body = format!(
        "{FRONTMATTER}## Invocation 1 — 2026-05-21T18:00:00Z\n\n\
         <drift-review verdict=\"pass\" round=\"1\" date=\"2026-05-21T18:00:00Z\" model=\"m\">\n\
         clean\n\
         </drift-review>\n\n\
         <gate verdict=\"passed\" tasks_hash=\"abc123\" date=\"2026-05-21T18:10:00Z\">\n\
         gate\n\
         </gate>\n",
    );
    write_vet(&spec_dir, &body)?;

    let (_code, out) = invoke(&ws.root)?;
    let json: Value = serde_json::from_str(&out)?;
    let any_vet = ["errors", "warnings", "info"].iter().any(|bucket| {
        at(&json, &["lint", bucket]).as_array().is_some_and(|arr| {
            arr.iter().any(|d| {
                field(d, "code")
                    .as_str()
                    .is_some_and(|c| c.starts_with("VET-"))
            })
        })
    });
    assert!(
        !any_vet,
        "a well-formed VET.md must emit no VET-* diagnostic; got: {out}",
    );
    Ok(())
}

/// A spec directory without a VET.md emits no VET-* diagnostic in any bucket.
#[test]
fn absent_vet_md_emits_no_vet_diagnostic() -> TestResult {
    let ws = Workspace::new()?;
    write_spec(
        &ws.root,
        "0001-no-vet",
        &spec_md_template("SPEC-0001", "implemented"),
        None,
    )?;

    let (_code, out) = invoke(&ws.root)?;
    let json: Value = serde_json::from_str(&out)?;
    let any_vet = ["errors", "warnings", "info"].iter().any(|bucket| {
        at(&json, &["lint", bucket]).as_array().is_some_and(|arr| {
            arr.iter().any(|d| {
                field(d, "code")
                    .as_str()
                    .is_some_and(|c| c.starts_with("VET-"))
            })
        })
    });
    assert!(
        !any_vet,
        "absence of VET.md must emit no VET-* diagnostic; got: {out}",
    );
    Ok(())
}
