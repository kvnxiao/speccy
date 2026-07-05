//! M2 done-when: a run cannot reach verified without recorded evidence
//! (covered in e2e), pasted command output is refused, a second agent gets
//! lease_held, and the provenance scan flags a seeded leak in a product file.

#![expect(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::needless_pass_by_value,
    clippy::doc_markdown,
    reason = "integration-test helpers assert on known-shape CLI/JSON output; indexing and expect are the idiomatic way a test fails and never reach shipped code"
)]

mod common;

use common::Harness;
use serde_json::Value;
use serde_json::json;
use std::thread::sleep;
use std::time::Duration;

/// Minimal single-task spec; `cmd_evidence` swaps R1's proof to a shell
/// command.
fn draft(cmd_evidence: Option<&str>) -> Value {
    let evidence = match cmd_evidence {
        Some(cmd) => json!([{ "id": "E1", "kind": "command", "command": cmd }]),
        None => json!([{ "id": "E1", "kind": "review", "note": "diff review" }]),
    };
    json!({
        "goal": "do the thing", "scope": { "in": ["x"] }, "risk": "standard",
        "requirements": [{ "id": "R1", "statement": "it works", "evidence": evidence }],
        "tasks": [{ "id": "T1", "title": "the task", "requirements": ["R1"] }]
    })
}

fn approve(h: &Harness, body: Value) -> (String, String) {
    let start = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "trust test", "title": "Trust test" }),
    );
    let spec_ref = start["spec_ref"]
        .as_str()
        .expect("spec_ref present")
        .to_string();
    let patched = h.ctl_in(
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
        &body,
    );
    assert_eq!(patched["lint"]["clean"], json!(true), "{patched}");
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
        .expect("approved_revision present")
        .to_string();
    (spec_ref, revision)
}

fn start_run(h: &Harness, body: Value) -> String {
    let (spec_ref, revision) = approve(h, body);
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
    started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string()
}

#[test]
fn approval_requires_human_prose() {
    let h = Harness::new();
    let start = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "trust test", "title": "Trust test" }),
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
        &draft(None),
    );
    let refused = h.ctl_in_raw(
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
        &json!({ "type": "approve", "revision": "spec_rev_001-draft" }),
    );
    assert_eq!(refused["error"]["code"], json!("validation_failed"));
    assert!(
        refused["error"]["message"]
            .as_str()
            .expect("error message present")
            .contains("approved_in_prose")
    );
}

#[test]
fn spec_status_risk_tracks_the_active_revision() {
    let h = Harness::new();
    let (spec_ref, _revision) = approve(&h, draft(None));
    h.ctl_in(
        &[
            "ctl",
            "spec",
            "patch-draft",
            "--spec",
            &spec_ref,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "set": { "risk": "critical" } }),
    );

    let status = h.ctl(&["ctl", "spec", "status", "--spec", &spec_ref, "--json"]);
    assert_eq!(status["active_revision"], json!("spec_rev_001"));
    assert_eq!(status["risk"], json!("standard"));
}

#[test]
fn second_agent_gets_lease_held() {
    let h = Harness::new();
    let run = start_run(&h, draft(None));
    // Agent A takes the lease.
    let a = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "claude:A", "--json",
    ]);
    assert_eq!(a["lease"]["agent"], json!("claude:A"));
    // Agent B, within the (default 600s) TTL, is refused.
    let b = h.ctl_raw(&[
        "ctl", "run", "next", "--run", &run, "--agent", "claude:B", "--json",
    ]);
    assert_eq!(b["ok"], json!(false));
    assert_eq!(b["error"]["code"], json!("lease_held"));
    assert!(
        b["error"]["message"]
            .as_str()
            .expect("error message present")
            .contains("claude:A")
    );
}

#[test]
fn evidence_record_must_match_the_declared_ledger() {
    let h = Harness::new();
    let run = start_run(&h, draft(None));

    let unknown = h.ctl_in_raw(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R9", "kind": "review", "collected_by": "v" }),
    );
    assert_eq!(unknown["error"]["code"], json!("not_found"));

    let wrong_kind = h.ctl_in_raw(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "request": "E1", "kind": "browser", "collected_by": "v" }),
    );
    assert_eq!(wrong_kind["error"]["code"], json!("validation_failed"));
    assert!(
        wrong_kind["error"]["message"]
            .as_str()
            .expect("error message present")
            .contains("not Browser")
    );
}

#[test]
fn set_status_requires_recorded_evidence_for_that_requirement() {
    let h = Harness::new();
    let run = start_run(&h, draft(None));
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();

    let refused = h.ctl_in_raw(
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
        &json!({ "updates": [{ "requirement": "R1", "status": "passed", "evidence": ["ev_missing"] }] }),
    );
    assert_eq!(refused["error"]["code"], json!("not_found"));
    assert!(
        refused["error"]["message"]
            .as_str()
            .expect("error message present")
            .contains("ev_missing")
    );
}

#[test]
fn high_risk_browser_evidence_requires_a_readable_artifact() {
    let h = Harness::new();
    let run = start_run(
        &h,
        json!({
            "goal": "do the thing", "scope": { "in": ["x"] }, "risk": "high",
            "requirements": [{ "id": "R1", "statement": "it works",
                "evidence": [{ "id": "E1", "kind": "browser", "note": "screenshot" }] }],
            "tasks": [{ "id": "T1", "title": "the task", "requirements": ["R1"] }]
        }),
    );

    let prose_only = h.ctl_in_raw(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "request": "E1", "kind": "browser", "collected_by": "v" }),
    );
    assert_eq!(prose_only["error"]["code"], json!("validation_failed"));
    assert!(
        prose_only["error"]["message"]
            .as_str()
            .expect("error message present")
            .contains("requires an artifact")
    );

    let missing_file = h.ctl_in_raw(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "request": "E1", "kind": "browser",
                 "collected_by": "v", "artifact": "evidence/missing.png" }),
    );
    assert_eq!(missing_file["error"]["code"], json!("validation_failed"));
    assert!(
        missing_file["error"]["message"]
            .as_str()
            .expect("error message present")
            .contains("not readable")
    );

    let outside_tree = h.ctl_in_raw(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "request": "E1", "kind": "browser",
                 "collected_by": "v", "artifact": "events.jsonl" }),
    );
    assert_eq!(outside_tree["error"]["code"], json!("validation_failed"));
    assert!(
        outside_tree["error"]["message"]
            .as_str()
            .expect("error message present")
            .contains("evidence/")
    );
}

#[test]
fn task_claim_records_the_caller_chosen_agent() {
    let h = Harness::new();
    let run = start_run(&h, draft(None));
    // Claim T1 with a distinctive, non-default agent ID.
    let d = h.ctl(&[
        "ctl",
        "run",
        "next",
        "--run",
        &run,
        "--agent",
        "codex:sess_probe",
        "--json",
    ]);
    let task = d["subject"]["task"]
        .as_str()
        .expect("task present")
        .to_string();
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    h.ctl(&[
        "ctl",
        "task",
        "claim",
        "--run",
        &run,
        "--task",
        &task,
        "--agent",
        "codex:sess_probe",
        "--lease",
        &lease,
        "--json",
    ]);
    // The claim is recorded against the caller's agent, not a hardcoded value.
    let log = h
        .read_home_file_containing(&format!("{run}/events.jsonl"))
        .expect("run events log stored");
    let claimed = log
        .lines()
        .find(|l| l.contains("\"type\":\"task_claimed\""))
        .expect("a task_claimed event");
    assert!(
        claimed.contains("\"agent\":\"codex:sess_probe\""),
        "claim must record the caller-chosen agent: {claimed}"
    );
}

#[test]
fn mutating_op_requires_the_live_lease() {
    let h = Harness::new();
    let run = start_run(&h, draft(None));
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let task = d["subject"]["task"].as_str().expect("task present");
    // A bogus lease token is refused.
    let bad = h.ctl_raw(&[
        "ctl",
        "task",
        "claim",
        "--run",
        &run,
        "--task",
        task,
        "--agent",
        "a",
        "--lease",
        "not-the-token",
        "--json",
    ]);
    assert_eq!(bad["error"]["code"], json!("lease_held"));
}

#[test]
fn expired_lease_is_cleared_with_resume_attribution() {
    let mut h = Harness::new();
    h.set_env("SPECCY_LEASE_TTL_SECONDS", "1");
    let run = start_run(&h, draft(None));

    // Agent A claims T1 and the worker edits files, then "crashes".
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "claude:A", "--json",
    ]);
    let task = d["subject"]["task"]
        .as_str()
        .expect("task present")
        .to_string();
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", &run, "--task", &task, "--agent", "claude:A", "--lease",
        &lease, "--json",
    ]);
    h.write_file("src/partial.rs", "fn work() {}\n");

    // Wait past the TTL, then a fresh session (agent B) resumes.
    sleep(Duration::from_millis(1200));
    let resumed = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "claude:B", "--json",
    ]);
    assert_eq!(resumed["lease"]["agent"], json!("claude:B"));
    assert_eq!(resumed["resume"]["cleared_lease"], json!("claude:A"));
    assert_eq!(
        resumed["resume"]["dirty_diff"]["attributed_to"],
        json!("T1")
    );
    assert!(
        resumed["resume"]["dirty_diff"]["files"]
            .as_u64()
            .expect("dirty diff files count present")
            >= 1
    );
}

#[test]
fn command_evidence_is_collected_by_the_controller() {
    let h = Harness::new();
    let run = start_run(&h, draft(Some("echo speccy-ok")));

    // evidence collect executes the declared command itself.
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
    let ev = &collected["evidence"][0];
    assert_eq!(ev["kind"], json!("command"));
    assert_eq!(ev["exit_code"], json!(0));
    assert_eq!(ev["collected_by"], json!("controller"));
    assert!(
        ev["stdout_hash"]
            .as_str()
            .expect("stdout_hash present")
            .starts_with("sha256:")
    );
    let artifact_hash = ev["artifact_hash"].as_str().expect("artifact_hash present");
    assert!(artifact_hash.starts_with("sha256:"));
    let log = h
        .read_home_file_containing(&format!("{run}/events.jsonl"))
        .expect("run events log stored");
    let evidence_event: Value = log
        .lines()
        .find(|l| l.contains("\"type\":\"evidence_recorded\""))
        .and_then(|l| serde_json::from_str(l).ok())
        .expect("an evidence_recorded event");
    assert_eq!(
        evidence_event["evidence"]["artifact_hash"],
        json!(artifact_hash)
    );

    // Pasting command output through evidence record is refused.
    let refused = h.ctl_in_raw(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "kind": "command", "collected_by": "agent" }),
    );
    assert_eq!(refused["error"]["code"], json!("validation_failed"));
    assert!(
        refused["error"]["message"]
            .as_str()
            .expect("error message present")
            .contains("evidence collect")
    );
}

#[test]
fn failing_command_records_nonzero_exit() {
    let h = Harness::new();
    let run = start_run(&h, draft(Some("exit 3")));
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
    assert_eq!(collected["evidence"][0]["exit_code"], json!(3));
}

#[test]
fn known_secret_env_value_is_scrubbed_from_stored_output() {
    let mut h = Harness::new();
    // A secret-named env var; the command echoes its value verbatim.
    h.set_env("MY_TEST_TOKEN", "ZZsecretvalueZZ");
    let run = start_run(&h, draft(Some("echo ZZsecretvalueZZ")));
    h.ctl(&[
        "ctl",
        "evidence",
        "collect",
        "--run",
        &run,
        "--requirements",
        "R1",
        "--json",
    ]);
    // The stored command output must carry the placeholder, not the raw secret.
    // (Scan the stdout section; the declared command string itself is not
    // "output" and is shown verbatim on the spec card.)
    let artifact = h
        .read_home_file_containing("/evidence/")
        .expect("evidence artifact stored");
    let stdout = artifact
        .split("--- stdout ---")
        .nth(1)
        .expect("artifact has a stdout section");
    assert!(
        stdout.contains("[REDACTED:MY_TEST_TOKEN]"),
        "expected scrubbed output, got: {stdout}"
    );
    assert!(
        !stdout.contains("ZZsecretvalueZZ"),
        "raw secret leaked into stored output: {stdout}"
    );
}

#[test]
fn output_over_the_byte_cap_is_noted_as_truncated() {
    let h = Harness::new();
    h.write_file(
        ".speccy/project.yaml",
        "evidence:\n  command_output_max_bytes: 4\n",
    );
    h.git(&["add", "-A"]);
    h.git(&["commit", "-m", "policy"]);
    let run = start_run(&h, draft(Some("echo aaaaaaaaaaaaaaaa")));
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
    let note = collected["evidence"][0]["note"].as_str().unwrap_or("");
    assert!(
        note.contains("truncated"),
        "expected truncation note, got {note:?}"
    );
}

#[test]
fn provenance_leak_in_product_file_blocks_integration() {
    let h = Harness::new();
    let run = start_run(&h, draft(None));

    // Claim + build T1; the worker leaks a Speccy identifier into a product file.
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
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("dispatch_worker"));
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    h.write_file(
        "src/leak.rs",
        "// this satisfies the speccy requirement\nfn f() {}\n",
    );
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
        &json!({ "task": "T1", "round": 1, "summary": "did it", "requirements_claimed": ["R1"] }),
    );

    // Verifier records evidence and passes R1 — but the diff leaks "speccy".
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("dispatch_verifier"));
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    let packet = h.ctl(&[
        "ctl",
        "packet",
        "verification",
        "--run",
        &run,
        "--requirements",
        "R1",
        "--json",
    ]);
    // The controller already scanned the task diff and recorded the leak.
    assert!(
        packet["provenance_scan"]["hits"]
            .as_u64()
            .expect("provenance hits count present")
            >= 1,
        "{packet}"
    );
    let ev = h.ctl_in(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "kind": "review", "collected_by": "v" }),
    );
    h.ctl_in(
        &["ctl", "requirement", "set-status", "--run", &run, "--lease", &lease, "--input", "-", "--json"],
        &json!({ "updates": [{ "requirement": "R1", "status": "passed", "evidence": [ev["id"]] }] }),
    );

    // Despite R1 passing, the blocking provenance finding forces a repair round,
    // not integration.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(
        d["action"],
        json!("dispatch_worker"),
        "expected repair, got {d}"
    );
    assert_eq!(d["round"]["current"], json!(2));
    let integrated = d["applied_transitions"]
        .as_array()
        .expect("applied_transitions present")
        .iter()
        .any(|t| t["to"] == json!("integrated"));
    assert!(
        !integrated,
        "task must not integrate with an unresolved leak"
    );
}
