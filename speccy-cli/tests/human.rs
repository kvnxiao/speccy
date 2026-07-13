//! M3 done-when: review renders the right state-aware packet and infers the
//! current spec, rework returns the run to implementing with an appended
//! RT<n>, and accept is idempotent after landing.

#![expect(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::too_many_lines,
    reason = "integration-test helpers assert on known-shape CLI/JSON output; indexing and expect are the idiomatic way a test fails and never reach shipped code"
)]

mod common;

use common::Harness;
use common::approve_minimal;
use common::drive_to_gate;
use serde_json::Value;
use serde_json::json;

fn verified_run(h: &Harness) -> (String, String) {
    let (spec_ref, revision) = approve_minimal(h, "Human test");
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &revision,
        "--json",
    ]);
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();
    let gate = drive_to_gate(h, &run);
    assert_eq!(gate["run_state"], json!("verified"), "{gate}");
    (spec_ref, run)
}

#[test]
fn ship_then_accept_is_idempotent() {
    let h = Harness::new();
    let (spec_ref, run) = verified_run(&h);

    // Take the ship lease, then record the ship.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["subject"]["gate"], json!("ship_decision"));
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    let shipped = h.ctl_in(
        &[
            "ctl",
            "run",
            "record-ship",
            "--run",
            &run,
            "--lease",
            &lease,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "kind": "pull_request", "url": "https://example/pr/1",
                 "branch": "speccy/x", "head_sha": "abc", "base": "main" }),
    );
    assert_eq!(shipped["run_state"], json!("submitted"));

    // A later run next just halts awaiting the external merge.
    let halt = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(halt["action"], json!("halt"));

    // Accept records the landing; the spec becomes accepted and leaves default
    // list.
    let out = h.human(&["accept", &spec_ref]);
    assert!(out.contains("submitted -> landed"), "{out}");
    assert!(out.contains("accepted"), "{out}");
    assert!(h.human(&["list"]).contains("No active specs"));
    assert!(h.human(&["list", "--accepted"]).contains(&spec_ref));

    // Accept again is idempotent.
    let again = h.human(&["accept", &spec_ref]);
    assert!(again.contains("already recorded"), "{again}");
}

#[test]
fn rework_returns_to_implementing_with_rt_task() {
    let h = Harness::new();
    let (_spec_ref, run) = verified_run(&h);

    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    let reworked = h.ctl_in(
        &[
            "ctl",
            "run",
            "record-decision",
            "--run",
            &run,
            "--lease",
            &lease,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "type": "rework", "reason": "reuse the standard error layout" }),
    );
    assert_eq!(reworked["run_state"], json!("implementing"));
    assert_eq!(reworked["task_appended"], json!("RT1"));

    // The loop resumes at the appended RT1 task.
    let next = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(next["run_state"], json!("implementing"));
    assert_eq!(next["action"], json!("claim_task"));
    assert_eq!(next["subject"]["task"], json!("RT1"));

    // The rework task carries the feedback into its packet.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", &run, "--task", "RT1", "--agent", "a", "--lease", &lease,
        "--json",
    ]);
    let packet = h.ctl(&[
        "ctl", "packet", "task", "--run", &run, "--task", "RT1", "--json",
    ]);
    assert!(
        packet["seed_feedback"]
            .as_str()
            .expect("seed_feedback present")
            .contains("standard error layout")
    );
}

#[test]
fn review_is_state_aware_and_infers_current_spec() {
    let h = Harness::new();
    let (spec_ref, _revision) = approve_minimal(&h, "Passwordless");

    // Approved, no run yet: review shows the spec card (inferred, no selector).
    let card = h.human(&["review"]);
    assert!(card.contains(&spec_ref), "{card}");
    assert!(card.contains("Acceptance"), "{card}");

    // Drive to verified, then review shows the review packet.
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        "spec_rev_001",
        "--json",
    ]);
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();
    drive_to_gate(&h, &run);
    let review = h.human(&["review"]);
    assert!(
        review.contains("verified") || review.contains("Ready to ship"),
        "{review}"
    );
    // --evidence drills into the ledger.
    let evidence = h.human(&["review", "--evidence"]);
    assert!(evidence.contains("Ledger"), "{evidence}");
}

#[test]
fn status_card_ready_to_ship() {
    let h = Harness::new();
    let (_spec_ref, _run) = verified_run(&h);
    let status = h.human(&["status"]);
    assert!(status.contains("Ready to ship"), "{status}");
    assert!(status.contains("Next: /speccy-ship"), "{status}");
    // No controller machinery leaks onto the card.
    assert!(!status.contains("lease"), "{status}");
    assert!(!status.contains("run_"), "{status}");
}

#[test]
fn status_card_uses_task_titles_in_activity() {
    let h = Harness::new();
    let (spec_ref, revision) = approve_minimal(&h, "Activity titles");
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &revision,
        "--json",
    ]);
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", &run, "--task", "T1", "--agent", "a", "--lease", &lease,
        "--json",
    ]);

    let status = h.human(&["status"]);
    assert!(status.contains("the task"), "{status}");
    assert!(!status.contains("T1"), "{status}");
}

#[test]
fn status_card_shows_interrupted_after_lease_expiry() {
    let mut h = Harness::new();
    h.set_env("SPECCY_LEASE_TTL_SECONDS", "1");
    let (spec_ref, revision) = approve_minimal(&h, "Interrupted");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &revision,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Claim T1 and let the worker edit, then let the lease expire (session dies).
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "claude:A", "--json",
    ]);
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", &run, "--task", "T1", "--agent", "claude:A", "--lease",
        &lease, "--json",
    ]);
    h.write_file("src/partial.rs", "fn work() {}\n");
    std::thread::sleep(std::time::Duration::from_millis(1200));

    let status = h.human(&["status"]);
    assert!(status.contains("Interrupted"), "{status}");
    assert!(status.contains("Uncommitted diff"), "{status}");
    assert!(status.contains("Next: /speccy-implement"), "{status}");
}

#[test]
fn review_json_is_state_aware() {
    let h = Harness::new();
    let (spec_ref, _revision) = approve_minimal(&h, "Json review");
    // Approved, no run yet → the spec-card surface, structurally.
    let out = h.human(&["review", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["surface"], json!("spec_card"), "{v}");
    assert_eq!(v["spec_ref"], json!(spec_ref));
}

#[test]
fn spec_card_lists_command_evidence_before_approval() {
    let h = Harness::new();
    let start = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "run command evidence", "title": "Command card" }),
    );
    let spec_ref = start["spec_ref"]
        .as_str()
        .expect("spec_ref present")
        .to_string();
    h.ctl_in(
        &[
            "ctl",
            "spec",
            "record-draft",
            "--spec",
            &spec_ref,
            "--input",
            "-",
            "--json",
        ],
        &json!({
            "goal": "g", "scope": { "in": ["x"] }, "risk": "standard",
            "requirements": [
                { "id": "R1", "statement": "it works",
                  "evidence": [
                    { "id": "E1", "kind": "command", "command": "cargo test --test trust" },
                    { "id": "E2", "kind": "command", "command": "cargo test --test trust" }
                  ] }
            ],
            "tasks": [{ "id": "T1", "title": "the task", "requirements": ["R1"] }]
        }),
    );

    let card = h.human(&["review"]);
    assert!(card.contains("Commands"), "{card}");
    assert!(card.contains("cargo test --test trust"), "{card}");
    assert_eq!(card.matches("cargo test --test trust").count(), 1, "{card}");
}

#[test]
fn archive_refuses_an_active_spec() {
    let h = Harness::new();
    let (spec_ref, _revision) = approve_minimal(&h, "Still active");
    let (_out, ok) = h.output(&["archive", &spec_ref]);
    assert!(!ok, "archive must refuse an approved (active) spec");
}

#[test]
fn cancel_and_new_and_list_query() {
    let h = Harness::new();
    let created = h.human(&["new", "Rate-limit magic links", "--title", "Rate limit"]);
    assert!(created.contains("Created draft spec"), "{created}");

    let listed = h.human(&["list", "--query", "rate"]);
    assert!(listed.contains("Rate limit"), "{listed}");

    // Cancel by selector.
    let spec_ref = listed
        .split_whitespace()
        .find(|w| w.starts_with("SPEC-"))
        .expect("SPEC- selector present")
        .to_string();
    let cancelled = h.human(&["cancel", &spec_ref]);
    assert!(cancelled.contains("Cancelled"), "{cancelled}");
    // Cancelled specs leave the active list.
    assert!(h.human(&["list"]).contains("No active specs"));
}

#[test]
fn escalation_packet_scopes_to_failing_requirement() {
    let h = Harness::new();
    let (spec_ref, revision) = approve_minimal(&h, "Escalate me");
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &revision,
        "--json",
    ]);
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Fail R1 through all three task repair rounds → escalation.
    let mut escalated = None;
    for _ in 0..40 {
        let d = h.ctl(&[
            "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
        ]);
        let lease = d["lease"]["token"]
            .as_str()
            .expect("lease token present")
            .to_string();
        match d["action"].as_str().expect("action present") {
            "claim_task" => {
                h.ctl(&[
                    "ctl", "task", "claim", "--run", &run, "--task", "T1", "--agent", "a",
                    "--lease", &lease, "--json",
                ]);
            }
            "dispatch_worker" => {
                let round = d["round"]["current"]
                    .as_u64()
                    .expect("round current present");
                h.write_file(&format!("src/attempt_{round}.txt"), "try\n");
                h.ctl_in(
                    &[
                        "ctl",
                        "task",
                        "record-handoff",
                        "--run",
                        &run,
                        "--lease",
                        &lease,
                        "--input",
                        "-",
                        "--json",
                    ],
                    &json!({ "task": "T1", "round": round, "summary": format!("attempt {round}"),
                             "requirements_claimed": ["R1"] }),
                );
            }
            "dispatch_verifier" => {
                let f = h.ctl_in(
                    &[
                        "ctl", "finding", "record", "--run", &run, "--input", "-", "--json",
                    ],
                    &json!({ "requirement": "R1", "severity": "blocking",
                             "note": "does not actually work", "recorded_by": "v" }),
                );
                h.ctl_in(
                    &[
                        "ctl",
                        "requirement",
                        "set-status",
                        "--run",
                        &run,
                        "--lease",
                        &lease,
                        "--input",
                        "-",
                        "--json",
                    ],
                    &json!({ "updates": [{ "requirement": "R1", "status": "failed",
                                           "findings": [f["id"]] }] }),
                );
            }
            "await_human_gate" => {
                escalated = Some(d);
                break;
            }
            other => panic!("unexpected {other}"),
        }
    }
    let d = escalated.expect("run should escalate");
    assert_eq!(d["subject"]["gate"], json!("escalation"));
    assert_eq!(d["run_state"], json!("escalated"));

    // The escalation packet is scoped to R1 with the tried approaches.
    let packet = h.ctl(&["ctl", "packet", "escalation", "--run", &run, "--json"]);
    assert_eq!(packet["failing"][0]["id"], json!("R1"));
    let md = packet["markdown"].as_str().expect("markdown present");
    assert!(md.contains("R1"), "{md}");
    assert!(md.contains("Recommended: amend the spec"), "{md}");

    // `speccy review` on an escalated run renders that escalation packet.
    let review = h.human(&["review", &spec_ref]);
    assert!(
        review.contains("Recommended: amend the spec"),
        "escalated review should show the escalation packet: {review}"
    );

    // Waiving R1 resolves it and resumes the run.
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    let waived = h.ctl_in(
        &[
            "ctl",
            "run",
            "record-decision",
            "--run",
            &run,
            "--lease",
            &lease,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "type": "waive", "requirement": "R1", "reason": "accepted",
                 "residual_risk": "not proven locally" }),
    );
    assert_eq!(waived["requirement_status"], json!("waived"));
    // The run leaves escalated and eventually reaches verified.
    let gate = drive_to_gate(&h, &run);
    assert_eq!(gate["run_state"], json!("verified"), "{gate}");
}

/// A2 cross-log convergence: a crash between accept's run `landed` transition
/// and the spec's `accepted` status is repaired by re-running `speccy accept`,
/// with no second run event.
#[test]
fn accept_retry_completes_the_spec_transition() {
    let h = Harness::new();
    let (spec_ref, run) = verified_run(&h);
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    h.ctl_in(
        &[
            "ctl",
            "run",
            "record-ship",
            "--run",
            &run,
            "--lease",
            &lease,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "kind": "none" }),
    );
    let out = h.human(&["accept", &spec_ref]);
    assert!(out.contains("submitted -> landed"), "{out}");

    // Simulate the crash: the run landed but the spec never became accepted.
    common::drop_last_event(&common::spec_log_path(&h));

    let retry = h.human(&["accept", &spec_ref]);
    assert!(
        retry.contains("completed the spec's accepted status"),
        "{retry}"
    );
    assert!(h.human(&["list", "--accepted"]).contains(&spec_ref));
    let run_log = h
        .read_home_file_containing(&format!("{run}/events.jsonl"))
        .expect("run event log exists");
    assert_eq!(
        run_log.matches("\"to\":\"landed\"").count(),
        1,
        "duplicate landed transition:\n{run_log}"
    );
    let spec_log = fs_err::read_to_string(common::spec_log_path(&h)).expect("read spec log");
    assert_eq!(
        spec_log.matches("\"to\":\"accepted\"").count(),
        1,
        "duplicate accepted status:\n{spec_log}"
    );

    // A retry after full success stays a no-op.
    let again = h.human(&["accept", &spec_ref]);
    assert!(again.contains("already recorded"), "{again}");
}

/// A2 cross-log convergence: `speccy cancel` records one cancellation
/// decision (the durable intent, first), and an exact retry converges the
/// runs a crash left behind without a second decision.
#[test]
fn cancel_retry_converges_runs_without_a_second_decision() {
    let h = Harness::new();
    let (spec_ref, revision) = approve_minimal(&h, "Cancel converge");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &revision,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    let out = h.human(&["cancel", &spec_ref]);
    assert!(out.contains("Cancelled"), "{out}");

    // Simulate the crash: the decision landed but the run was never cancelled.
    let run_log = h
        .home_path_containing(&format!("{run}/events.jsonl"))
        .expect("run event log path");
    common::drop_last_event(&run_log);
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    assert_eq!(status["run_state"], json!("implementing"), "{status}");

    let retry = h.human(&["cancel", &spec_ref]);
    assert!(retry.contains("Cancelled"), "{retry}");
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    assert_eq!(status["run_state"], json!("cancelled"), "{status}");
    let spec_log = fs_err::read_to_string(common::spec_log_path(&h)).expect("read spec log");
    assert_eq!(
        spec_log.matches("\"type\":\"cancel\"").count(),
        1,
        "duplicate cancellation decision:\n{spec_log}"
    );

    // A cancel of an already-cancelled spec with no active runs is a no-op.
    let again = h.human(&["cancel", &spec_ref]);
    assert!(again.contains("already cancelled"), "{again}");
    let spec_log = fs_err::read_to_string(common::spec_log_path(&h)).expect("read spec log");
    assert_eq!(spec_log.matches("\"type\":\"cancel\"").count(), 1);
}

// E: the run-bundle receipt is deterministic, verifiable, and safe by
// construction — no raw command output, secrets scrubbed from included notes.
#[test]
fn run_bundle_receipt_is_deterministic_and_safe() {
    let mut h = Harness::new();
    h.set_env("RECEIPT_TEST_TOKEN", "tok-sekrit-value-123");

    // Spec whose command evidence prints a sentinel we must NOT find in the
    // receipt.
    let start = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "receipt test", "title": "Receipt test" }),
    );
    let spec_ref = start["spec_ref"]
        .as_str()
        .expect("spec_ref present")
        .to_string();
    h.ctl_in(
        &[
            "ctl",
            "spec",
            "record-draft",
            "--spec",
            &spec_ref,
            "--input",
            "-",
            "--json",
        ],
        &json!({
            "goal": "g", "scope": { "in": ["x"] }, "risk": "standard",
            "requirements": [{ "id": "R1", "statement": "works",
                "evidence": [{ "id": "E1", "kind": "command",
                               "command": "echo raw-stdout-sentinel-xyz" }] }],
            "tasks": [{ "id": "T1", "title": "t", "requirements": ["R1"] }]
        }),
    );
    let approved = h.ctl_in(
        &[
            "ctl",
            "spec",
            "record-decision",
            "--spec",
            &spec_ref,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "type": "approve", "revision": "spec_rev_001-draft", "approved_in_prose": "go" }),
    );
    let revision = approved["approved_revision"]
        .as_str()
        .expect("approved revision")
        .to_string();
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &revision,
        "--json",
    ]);
    let run = started["run_id"].as_str().expect("run id").to_string();

    // Collected command evidence stores the raw sentinel stdout in the log
    // and artifact; a residual-risk note carries a known-secret value.
    let collected = h.ctl(&[
        "ctl",
        "evidence",
        "collect",
        "--run",
        &run,
        "--requirements",
        "R1",
        "--json",
    ]);
    let ev_id = collected["evidence"][0]["id"]
        .as_str()
        .expect("evidence id")
        .to_string();
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token")
        .to_string();
    h.ctl_in(
        &[
            "ctl",
            "requirement",
            "set-status",
            "--run",
            &run,
            "--lease",
            &lease,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "updates": [{ "requirement": "R1", "status": "review_passed",
                 "evidence": [ev_id],
                 "residual_risk": "auth uses tok-sekrit-value-123 in dev" }] }),
    );

    // Export twice, plus once with --redact: byte-identical output.
    let (out1, ok1) = h.output(&["export", "run-bundle", "--dest", "d1"]);
    assert!(ok1, "{out1}");
    let (_, ok2) = h.output(&["export", "run-bundle", "--dest", "d2"]);
    assert!(ok2);
    let (_, ok3) = h.output(&["export", "run-bundle", "--redact", "--dest", "d3"]);
    assert!(ok3);
    let json1 = h.read("d1/run-bundle.json");
    assert_eq!(
        json1,
        h.read("d2/run-bundle.json"),
        "receipt must be deterministic"
    );
    assert_eq!(
        json1,
        h.read("d3/run-bundle.json"),
        "--redact output is identical"
    );
    let md = h.read("d1/run-bundle.md");

    // The manifest hash verifies over the receipt minus the hash field.
    let mut receipt: Value = serde_json::from_str(&json1).expect("receipt parses");
    assert_eq!(receipt["receipt_schema"], json!(1));
    assert_eq!(receipt["spec"]["ref"], json!(spec_ref));
    assert_eq!(receipt["run"]["id"], json!(run));
    let manifest = receipt["manifest_hash"]
        .as_str()
        .expect("manifest hash present")
        .to_string();
    receipt
        .as_object_mut()
        .expect("receipt is an object")
        .remove("manifest_hash");
    assert_eq!(
        speccy_core::hash::sha256_prefixed(receipt.to_string().as_bytes()),
        manifest
    );
    assert!(md.contains(&format!("Manifest {manifest}")));

    // Allowlist only: hashes are present, raw stdout bodies and command
    // strings are not, and the included note is secret-scrubbed.
    let ev = &receipt["evidence"][0];
    assert_eq!(ev["kind"], json!("command"));
    assert_eq!(ev["exit_code"], json!(0));
    assert!(
        ev["stdout_hash"]
            .as_str()
            .expect("stdout hash kept")
            .starts_with("sha256:")
    );
    for text in [&json1, &md] {
        assert!(
            !text.contains("raw-stdout-sentinel-xyz"),
            "raw command output leaked into the receipt"
        );
        assert!(
            !text.contains("tok-sekrit-value-123"),
            "secret value leaked into the receipt"
        );
    }
    assert!(
        json1.contains("[REDACTED:RECEIPT_TEST_TOKEN]"),
        "residual-risk note should be scrubbed, not dropped: {json1}"
    );
}
