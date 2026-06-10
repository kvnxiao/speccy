#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy journal append <SPEC-NNNN>` vet block
//! routing (SPEC-0055 REQ-004).
//!
//! Drives the built `speccy` binary against scratch workspaces. The
//! load-bearing scenarios are CHK-006 (a `drift-review` → `holistic-fix` →
//! `gate --verdict passed` sequence produces a VET.md that parses, holds one
//! invocation section ending in the gate, and lets `speccy next` resolve the
//! spec past the vet step) and CHK-007 (a following `simplifier-scan` opens a
//! fresh `## Invocation 2` rather than landing in the gate-terminated first
//! section). Two further tests pin the `tasks_hash` freshness contract and
//! the DEC-004 selector/block-type pairing.

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use common::TestResult;
use common::Workspace;
use common::sha256_hex;
use common::spec_md_template;
use common::task_xml;
use common::tasks_md_xml;
use common::write_spec;
use predicates::str::contains;
use speccy_core::parse::VetBlock;
use speccy_core::parse::parse_vet_xml;

/// Build a workspace with one in-progress spec whose single task is
/// `completed`, returning the workspace and the spec dir.
fn workspace_with_completed_task() -> TestResult<(Workspace, Utf8PathBuf)> {
    let ws = Workspace::new()?;
    let spec_id = "SPEC-0042";
    let tasks_md = tasks_md_xml(spec_id, &task_xml("T-001", "completed"));
    let dir = write_spec(
        &ws.root,
        "0042-example-slug",
        &spec_md_template(spec_id, "in-progress"),
        Some(&tasks_md),
    )?;
    Ok((ws, dir))
}

fn vet_path(spec_dir: &Utf8Path) -> Utf8PathBuf {
    spec_dir.join("journal").join("VET.md")
}

/// Append one vet block; returns the command for `.assert()` chaining.
fn append(ws: &Workspace, args: &[&str], body: &str) -> Command {
    let mut full = vec!["journal", "append"];
    full.extend_from_slice(args);
    let mut cmd = Command::cargo_bin("speccy").expect("cargo bin");
    cmd.args(&full)
        .current_dir(ws.root.as_std_path())
        .write_stdin(body.to_owned());
    cmd
}

/// CHK-006: a `drift-review` → `holistic-fix` → `gate --verdict passed`
/// sequence produces a VET.md that parses under the new parser, holds one
/// invocation section ending in the gate, and lets `speccy next` resolve the
/// spec past the vet step (the gate's `tasks_hash` is fresh).
#[test]
fn drift_review_holistic_fix_gate_pass_resolves_past_vet() -> TestResult {
    let (ws, spec_dir) = workspace_with_completed_task()?;
    let vpath = vet_path(&spec_dir);
    assert!(!vpath.as_std_path().exists(), "VET.md must start absent");

    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "drift-review",
            "--verdict",
            "blocking",
            "--model",
            "test-model",
        ],
        "drift found",
    )
    .assert()
    .success();

    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "holistic-fix",
            "--verdict",
            "addressed",
            "--model",
            "test-model",
        ],
        "fixed the drift",
    )
    .assert()
    .success();

    append(
        &ws,
        &["SPEC-0042", "--block", "gate", "--verdict", "passed"],
        "shipping",
    )
    .assert()
    .success();

    // VET.md parses under the strict parser, one section, gate last.
    let src = fs_err::read_to_string(vpath.as_std_path())?;
    let doc = parse_vet_xml(&src, &vpath)?;
    assert_eq!(doc.invocations.len(), 1, "exactly one invocation section");
    let inv = doc.invocations.first().expect("one invocation");
    assert_eq!(inv.number, 1);
    assert_eq!(inv.blocks.len(), 3, "drift-review, holistic-fix, gate");
    assert!(
        matches!(inv.blocks.last(), Some(VetBlock::Gate { .. })),
        "the gate must be the last block",
    );

    // The drift-review opened round 1; the holistic-fix attached to it.
    match inv.blocks.first().expect("drift-review") {
        VetBlock::DriftReview { round, .. } => assert_eq!(*round, 1),
        other => return Err(format!("expected drift-review, got {other:?}").into()),
    }
    match inv.blocks.get(1).expect("holistic-fix") {
        VetBlock::HolisticFix { round, .. } => assert_eq!(*round, 1),
        other => return Err(format!("expected holistic-fix, got {other:?}").into()),
    }

    // `speccy next` resolves past the vet step: all tasks completed +
    // fresh passing gate + REPORT.md absent → ship.
    Command::cargo_bin("speccy")?
        .args(["next", "SPEC-0042", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .stdout(contains("\"kind\":\"ship\""));
    Ok(())
}

/// CHK-007: a `simplifier-scan` append after a gate-terminated section lands
/// under a freshly opened `## Invocation 2`, not the closed first section.
#[test]
fn simplifier_scan_after_gate_opens_invocation_two() -> TestResult {
    let (ws, spec_dir) = workspace_with_completed_task()?;
    let vpath = vet_path(&spec_dir);

    // Build a gate-terminated first section.
    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "drift-review",
            "--verdict",
            "pass",
            "--model",
            "test-model",
        ],
        "no drift",
    )
    .assert()
    .success();
    append(
        &ws,
        &["SPEC-0042", "--block", "gate", "--verdict", "passed"],
        "gate one",
    )
    .assert()
    .success();

    // Now a simplifier-scan must open Invocation 2. The second section is
    // open (no gate yet) after this append, a shape the strict parser
    // rejects — assert the fresh heading on the raw text first.
    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "simplifier-scan",
            "--verdict",
            "candidates",
        ],
        "candidate simplifications",
    )
    .assert()
    .success();

    let raw = fs_err::read_to_string(vpath.as_std_path())?;
    assert!(
        raw.contains("## Invocation 2"),
        "the scan must open a second invocation heading, got:\n{raw}",
    );

    // Close the second section with a gate so the whole file round-trips
    // through the strict parser, then assert the scan opened section 2.
    append(
        &ws,
        &["SPEC-0042", "--block", "gate", "--verdict", "passed"],
        "gate two",
    )
    .assert()
    .success();

    let src = fs_err::read_to_string(vpath.as_std_path())?;
    let doc = parse_vet_xml(&src, &vpath)?;
    assert_eq!(doc.invocations.len(), 2, "two invocation sections");
    let second = doc.invocations.get(1).expect("invocation 2");
    assert_eq!(second.number, 2);
    assert!(
        matches!(second.blocks.first(), Some(VetBlock::SimplifierScan { .. })),
        "the scan opens the second section",
    );
    Ok(())
}

/// REQ-004 behavior: two gate appends straddling a TASKS.md edit carry
/// different `tasks_hash` values, each equal to the file hash at its own
/// append time. (Each gate is in its own invocation section.)
#[test]
fn two_gates_across_a_tasks_edit_carry_distinct_fresh_hashes() -> TestResult {
    let (ws, spec_dir) = workspace_with_completed_task()?;
    let vpath = vet_path(&spec_dir);
    let tasks_path = spec_dir.join("TASKS.md");

    // First section: drift-review + gate. Capture the hash at this point.
    let tasks_v1 = fs_err::read(tasks_path.as_std_path())?;
    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "drift-review",
            "--verdict",
            "pass",
            "--model",
            "m",
        ],
        "clean",
    )
    .assert()
    .success();
    append(
        &ws,
        &["SPEC-0042", "--block", "gate", "--verdict", "passed"],
        "gate one",
    )
    .assert()
    .success();

    // Edit TASKS.md between the two gates so the bytes (and hash) change.
    let edited = format!(
        "{}\n<!-- edit between gates -->\n",
        String::from_utf8(tasks_v1.clone())?
    );
    fs_err::write(tasks_path.as_std_path(), &edited)?;

    // Second section: drift-review + gate (the new section opens automatically).
    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "drift-review",
            "--verdict",
            "pass",
            "--model",
            "m",
        ],
        "still clean",
    )
    .assert()
    .success();
    append(
        &ws,
        &["SPEC-0042", "--block", "gate", "--verdict", "passed"],
        "gate two",
    )
    .assert()
    .success();

    let src = fs_err::read_to_string(vpath.as_std_path())?;
    let doc = parse_vet_xml(&src, &vpath)?;
    let hashes: Vec<String> = doc
        .invocations
        .iter()
        .filter_map(|inv| {
            inv.blocks.iter().find_map(|b| match b {
                VetBlock::Gate { tasks_hash, .. } => Some(tasks_hash.clone()),
                _ => None,
            })
        })
        .collect();
    assert_eq!(hashes.len(), 2, "two gate hashes");
    let first = hashes.first().expect("first gate hash");
    let second = hashes.get(1).expect("second gate hash");
    assert_ne!(first, second, "the TASKS.md edit must change the hash");
    assert_eq!(
        *first,
        sha256_hex(&tasks_v1),
        "first gate hashes the pre-edit TASKS.md",
    );
    assert_eq!(
        *second,
        sha256_hex(edited.as_bytes()),
        "second gate hashes the post-edit TASKS.md",
    );
    Ok(())
}

/// DEC-004: a vet block type with a task selector is an argument error, and
/// a task block type with a bare spec selector is too — VET.md untouched.
#[test]
fn mismatched_selector_block_pairing_is_an_argument_error() -> TestResult {
    let (ws, spec_dir) = workspace_with_completed_task()?;
    let vpath = vet_path(&spec_dir);

    // Vet block + task selector → error.
    append(
        &ws,
        &[
            "SPEC-0042/T-001",
            "--block",
            "drift-review",
            "--verdict",
            "pass",
            "--model",
            "m",
        ],
        "body",
    )
    .assert()
    .failure()
    .stderr(contains("block type but the selector"));
    assert!(
        !vpath.as_std_path().exists(),
        "a mismatched vet append must not create VET.md",
    );

    // Task block + bare spec selector → error.
    let tjournal = spec_dir.join("journal").join("T-001.md");
    append(
        &ws,
        &["SPEC-0042", "--block", "implementer", "--model", "m"],
        "body",
    )
    .assert()
    .failure()
    .stderr(contains("block type but the selector"));
    assert!(
        !tjournal.as_std_path().exists(),
        "a mismatched task append must not create the task journal",
    );
    Ok(())
}

/// A `## Invocation N — <date>` line inside a drift-review body (plausible
/// when a persona quotes a prior VET.md excerpt) must not be mistaken for a
/// real section heading. Append derivation runs off the typed in-flight parse
/// (DEC-008), and the parser already excludes heading-shaped lines inside a
/// block body from its section count. Driven through the real append: a
/// following holistic-fix must still attach (no spurious `NoRoundToAttach`),
/// and a following drift-review must number the next invocation from the one
/// real heading rather than the phantom in-body number.
#[test]
fn invocation_heading_in_body_does_not_skew_append_derivation() -> TestResult {
    let (ws, spec_dir) = workspace_with_completed_task()?;
    let vpath = vet_path(&spec_dir);

    // A drift-review whose body quotes a `## Invocation 9 — <date>` line.
    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "drift-review",
            "--verdict",
            "blocking",
            "--model",
            "test-model",
        ],
        "quoting a prior run:\n## Invocation 9 — 2026-05-21T18:00:00Z\nthat run drifted",
    )
    .assert()
    .success();

    // The heading-shaped line reached disk (the body guard does not reject it).
    let raw = fs_err::read_to_string(vpath.as_std_path())?;
    assert!(
        raw.contains("## Invocation 9 — 2026-05-21T18:00:00Z"),
        "the in-body heading line must reach disk, got:\n{raw}",
    );

    // A following holistic-fix attaches to the open round — the in-body
    // `## Invocation 9` must NOT have opened a phantom section that loses the
    // open round (which produced the round-1 `NoRoundToAttach` symptom).
    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "holistic-fix",
            "--verdict",
            "addressed",
            "--model",
            "test-model",
        ],
        "addressed the drift",
    )
    .assert()
    .success();

    // Close the section with a gate so the whole file round-trips, then assert
    // the strict parser sees exactly ONE invocation (the in-body heading
    // excluded by its body-range discipline).
    append(
        &ws,
        &["SPEC-0042", "--block", "gate", "--verdict", "passed"],
        "shipping",
    )
    .assert()
    .success();

    let src = fs_err::read_to_string(vpath.as_std_path())?;
    let doc = parse_vet_xml(&src, &vpath)?;
    assert_eq!(
        doc.invocations.len(),
        1,
        "the in-body `## Invocation 9` must not count as a real section heading",
    );
    let inv = doc.invocations.first().expect("one invocation");
    assert_eq!(inv.number, 1, "the one real heading is Invocation 1");
    assert_eq!(
        inv.blocks.len(),
        3,
        "drift-review + holistic-fix attached + gate",
    );

    // A following drift-review opens Invocation 2 — numbered from the single
    // real heading, not the phantom in-body 9.
    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "drift-review",
            "--verdict",
            "pass",
            "--model",
            "test-model",
        ],
        "second pass, clean",
    )
    .assert()
    .success();

    let raw2 = fs_err::read_to_string(vpath.as_std_path())?;
    assert!(
        raw2.contains("## Invocation 2"),
        "the next invocation numbers from the real heading (1 → 2), got:\n{raw2}",
    );
    assert!(
        !raw2.contains("## Invocation 10"),
        "the phantom in-body 9 must not seed a `## Invocation 10`, got:\n{raw2}",
    );
    Ok(())
}

/// A `drift-review` body containing a line-isolated close tag `</drift-review>`
/// followed by a `## Invocation N` line must be rejected at write time, not
/// silently written. The shared scanner reads the in-body `</drift-review>` as
/// a structural close, so the would-be-new file fails the write-time
/// `parse_vet_in_flight` round-trip (close tag without matching open) and the
/// append is refused with VET.md absent (DEC-008) — no separate body guard
/// needed.
#[test]
fn close_tag_in_body_is_rejected_at_write_time() -> TestResult {
    let (ws, spec_dir) = workspace_with_completed_task()?;
    let vpath = vet_path(&spec_dir);

    // The append must fail; VET.md must stay absent (this is the first append
    // against a fresh workspace, so nothing else could have created it).
    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "drift-review",
            "--verdict",
            "blocking",
            "--model",
            "test-model",
        ],
        "intro\n</drift-review>\n## Invocation 9 — 2026-05-21T18:00:00Z\ntail",
    )
    .assert()
    .failure();
    assert!(
        !vpath.as_std_path().exists(),
        "a body smuggling a line-isolated close tag must be rejected before any write",
    );
    Ok(())
}

/// A `drift-review` body line whose tag name is separated from its attributes
/// by a Unicode-whitespace character (here U+000C form-feed) must be rejected
/// at write time, not silently written. The shared scanner's `\s` class reads
/// `<gate\u{0c}verdict="passed">` as a line-isolated `<gate>` open tag, so the
/// would-be-new file fails the `parse_vet_in_flight` round-trip (a nested
/// block) and the append is refused with VET.md absent. Because the same
/// parser is the authority for both reading and writing (DEC-008), there is no
/// whitespace-class gap between a body guard and the scanner to slip through.
#[test]
fn form_feed_separated_open_tag_in_body_is_rejected_at_write_time() -> TestResult {
    for body in [
        "intro\n<gate\u{0c}verdict=\"passed\">\ntail",
        "intro\n<drift-review\u{0c}round=\"9\">\ntail",
    ] {
        let (ws, spec_dir) = workspace_with_completed_task()?;
        let vpath = vet_path(&spec_dir);

        // First append against a fresh workspace, so nothing else could create
        // VET.md: a rejection must leave it absent.
        append(
            &ws,
            &[
                "SPEC-0042",
                "--block",
                "drift-review",
                "--verdict",
                "blocking",
                "--model",
                "test-model",
            ],
            body,
        )
        .assert()
        .failure();
        assert!(
            !vpath.as_std_path().exists(),
            "a body with a form-feed-separated line-isolated open tag must be \
             rejected before any write (body {body:?})",
        );
    }
    Ok(())
}

/// A `drift-review` body whose own line is a whitespace-padded vet close tag
/// (`</drift-review >`, `</gate >`, a tab variant, a CRLF variant) must be
/// rejected at write time with VET.md absent. The scanner's close-tag
/// predicate (`^</([a-z][a-z-]*)\s*>$`) reads these as structural close tags,
/// so the would-be-new file fails the `parse_vet_in_flight` round-trip and the
/// append is refused (DEC-008). Driven through the real binary across space,
/// tab, and CRLF variants.
#[test]
fn whitespace_padded_close_tag_in_body_is_rejected_at_write_time() -> TestResult {
    for body in [
        "intro\n</drift-review >\ntail",
        "intro\n</gate >\ntail",
        "intro\n</drift-review\t>\ntail",
        "intro\r\n</drift-review >\r\ntail\r\n",
    ] {
        let (ws, spec_dir) = workspace_with_completed_task()?;
        let vpath = vet_path(&spec_dir);

        // First append against a fresh workspace, so nothing else could create
        // VET.md: a rejection must leave it absent.
        append(
            &ws,
            &[
                "SPEC-0042",
                "--block",
                "drift-review",
                "--verdict",
                "blocking",
                "--model",
                "test-model",
            ],
            body,
        )
        .assert()
        .failure();
        assert!(
            !vpath.as_std_path().exists(),
            "a body with a whitespace-padded line-isolated close tag must be \
             rejected before any write (body {body:?})",
        );
    }
    Ok(())
}

/// Write-time round-trip invariant, driven through the real binary: any body
/// the append accepts must produce a VET.md that strict-parses once its
/// section is gate-terminated. The append re-parses the would-be-new file
/// through the VET parser before writing (DEC-008), so this property holds by
/// construction — the parser is the single authority for what lands on disk.
///
/// For each accepted body we append a `drift-review` carrying it, then close
/// the section with a `gate`, then assert the produced file `parse_vet_xml`-es
/// cleanly as exactly one invocation. Legitimate prose (inline mid-sentence
/// vet-tag mentions, a quoted `## Invocation` line, plain multi-paragraph
/// text) is accepted and the file parses.
#[test]
fn any_accepted_body_produces_a_strict_parseable_vet() -> TestResult {
    let accepted_bodies = [
        "the gateway at <gateway> and 3 < 4 are fine prose",
        "drift summary mentioning <gate verdict=\"passed\"> inline",
        "the reviewer wrote </drift-review> inline in a sentence",
        "quoting a prior log:\n## Invocation 9 — 2026-05-21T18:00:00Z\nend",
        "plain multi-paragraph\n\ndrift body\nwith no markup",
    ];
    for body in accepted_bodies {
        let (ws, spec_dir) = workspace_with_completed_task()?;
        let vpath = vet_path(&spec_dir);

        append(
            &ws,
            &[
                "SPEC-0042",
                "--block",
                "drift-review",
                "--verdict",
                "blocking",
                "--model",
                "test-model",
            ],
            body,
        )
        .assert()
        .success();
        append(
            &ws,
            &["SPEC-0042", "--block", "gate", "--verdict", "passed"],
            "shipping is clear",
        )
        .assert()
        .success();

        let raw = fs_err::read_to_string(vpath.as_std_path())?;
        let doc = parse_vet_xml(&raw, &vpath)
            .map_err(|e| format!("accepted body {body:?} must strict-parse, got {e}"))?;
        assert_eq!(
            doc.invocations.len(),
            1,
            "accepted body {body:?} must produce exactly one invocation",
        );
    }
    Ok(())
}

/// Write-time round-trip non-over-rejection, driven through the real binary: a
/// `drift-review` body carrying an inert (non-line-isolated) `<gate ...>`
/// mention in prose is legitimate — the scanner does not read a mid-sentence
/// `<gate ...>` as a structural tag — so the section closed by a real `gate`
/// parses, the gate append succeeds, and `speccy next` resolves past the vet
/// step. Confirms the `parse_vet_in_flight` round-trip (DEC-008) does not
/// over-reject a file whose body merely mentions a tag.
#[test]
fn gate_round_trip_accepts_legitimate_inline_tag_prose() -> TestResult {
    let (ws, spec_dir) = workspace_with_completed_task()?;
    let vpath = vet_path(&spec_dir);

    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "drift-review",
            "--verdict",
            "blocking",
            "--model",
            "test-model",
        ],
        "noted an inline <gate verdict=\"passed\"> mention in prose",
    )
    .assert()
    .success();
    append(
        &ws,
        &["SPEC-0042", "--block", "gate", "--verdict", "passed"],
        "all clear",
    )
    .assert()
    .success();

    let raw = fs_err::read_to_string(vpath.as_std_path())?;
    let doc = parse_vet_xml(&raw, &vpath)?;
    assert_eq!(doc.invocations.len(), 1);
    let inv = doc.invocations.first().expect("one invocation");
    assert!(
        matches!(inv.blocks.last(), Some(VetBlock::Gate { .. })),
        "section must end in the gate",
    );
    Ok(())
}

/// REQ-004 done-when: a `holistic-fix` with no preceding `drift-review` in
/// the open section exits non-zero with VET.md still absent.
#[test]
fn holistic_fix_with_no_drift_review_exits_nonzero() -> TestResult {
    let (ws, spec_dir) = workspace_with_completed_task()?;
    let vpath = vet_path(&spec_dir);

    append(
        &ws,
        &[
            "SPEC-0042",
            "--block",
            "holistic-fix",
            "--verdict",
            "addressed",
            "--model",
            "m",
        ],
        "premature fix",
    )
    .assert()
    .failure();
    assert!(
        !vpath.as_std_path().exists(),
        "no drift-review to attach to; VET.md must stay absent",
    );
    Ok(())
}
