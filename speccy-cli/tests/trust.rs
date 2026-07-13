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

/// B: a command that backgrounds a delayed sentinel write leaves no live
/// descendant — the write must never land after evidence is recorded.
/// (Unix shell syntax; the Windows twin is below.)
#[cfg(unix)]
#[test]
fn backgrounded_descendant_cannot_mutate_after_collection() {
    let h = Harness::new();
    let run = start_run(
        &h,
        draft(Some("( sleep 2 && touch escaped_normal.txt ) & echo done")),
    );
    let out = h.ctl(&[
        "ctl",
        "evidence",
        "collect",
        "--run",
        &run,
        "--requirements",
        "R1",
        "--json",
    ]);
    assert_eq!(out["evidence"][0]["exit_code"], json!(0), "{out}");
    assert_eq!(out["evidence"][0]["contained"], json!(true), "{out}");
    // Past the sentinel delay: the descendant must be dead, not just slow.
    sleep(Duration::from_secs(3));
    assert!(
        !h.exists("escaped_normal.txt"),
        "descendant survived teardown and mutated the workspace"
    );
}

/// B: the same containment holds when the command itself times out.
#[cfg(unix)]
#[test]
fn timed_out_descendant_cannot_mutate_after_collection() {
    let h = Harness::new();
    h.write_file(
        ".speccy/project.yaml",
        "evidence:\n  command_timeout_seconds: 1\n",
    );
    h.git(&["add", "-A"]);
    h.git(&["commit", "-m", "config"]);
    let run = start_run(
        &h,
        draft(Some("( sleep 3 && touch escaped_timeout.txt ) & sleep 30")),
    );
    let out = h.ctl(&[
        "ctl",
        "evidence",
        "collect",
        "--run",
        &run,
        "--requirements",
        "R1",
        "--json",
    ]);
    assert_eq!(out["evidence"][0]["exit_code"], json!(-1), "{out}");
    let note = out["evidence"][0]["note"].as_str().expect("note present");
    assert!(note.contains("timed out"), "{out}");
    assert_eq!(out["evidence"][0]["contained"], json!(true), "{out}");
    sleep(Duration::from_secs(3));
    assert!(
        !h.exists("escaped_timeout.txt"),
        "descendant survived the timeout teardown"
    );
}

/// B (Windows twin): a detached descendant is contained by the job object.
#[cfg(windows)]
#[test]
fn backgrounded_descendant_cannot_mutate_after_collection() {
    let h = Harness::new();
    let run = start_run(
        &h,
        draft(Some(
            "start /b cmd /c \"ping -n 4 127.0.0.1 > nul & echo x > escaped_normal.txt\"",
        )),
    );
    let out = h.ctl(&[
        "ctl",
        "evidence",
        "collect",
        "--run",
        &run,
        "--requirements",
        "R1",
        "--json",
    ]);
    assert_eq!(out["evidence"][0]["contained"], json!(true), "{out}");
    sleep(Duration::from_secs(5));
    assert!(
        !h.exists("escaped_normal.txt"),
        "descendant survived teardown and mutated the workspace"
    );
}

/// B: equal dirty-file counts with different contents are different repo
/// identities — attribution rests on the diff hash, not a count.
#[test]
fn equal_dirty_counts_with_different_contents_attribute_differently() {
    let h = Harness::new();
    let run = start_run(&h, draft(Some("echo hi")));

    h.write_file("attr.txt", "one\n");
    let first = h.ctl(&[
        "ctl",
        "evidence",
        "collect",
        "--run",
        &run,
        "--requirements",
        "R1",
        "--json",
    ]);
    h.write_file("attr.txt", "two\n");
    let second = h.ctl(&[
        "ctl",
        "evidence",
        "collect",
        "--run",
        &run,
        "--requirements",
        "R1",
        "--json",
    ]);

    let a = &first["evidence"][0]["repo"];
    let b = &second["evidence"][0]["repo"];
    assert_eq!(a["head_changed"], json!(false), "{first}");
    assert_ne!(
        a["diff_hash_after"], b["diff_hash_after"],
        "same dirty count, different contents must be different identities"
    );
}

/// B: a command that changes HEAD is explicitly reported on the record.
#[test]
fn command_that_changes_head_is_reported() {
    let h = Harness::new();
    let run = start_run(
        &h,
        draft(Some(
            "git -c user.email=t@t -c user.name=t commit --allow-empty -m oob",
        )),
    );
    let out = h.ctl(&[
        "ctl",
        "evidence",
        "collect",
        "--run",
        &run,
        "--requirements",
        "R1",
        "--json",
    ]);
    let repo = &out["evidence"][0]["repo"];
    assert_eq!(repo["head_changed"], json!(true), "{out}");
    assert_ne!(repo["head_before"], repo["head_after"], "{out}");
    let note = out["evidence"][0]["note"].as_str().expect("note present");
    assert!(note.contains("changed HEAD"), "{out}");
}

/// B: when git facts are unavailable, evidence collection fails closed (no
/// event recorded), the provenance scan halts `run next`, and the planning
/// packet reports structured nulls with warnings — never fabricated cleans.
#[test]
fn unavailable_git_facts_fail_closed_and_render_unavailability() {
    let h = Harness::new();
    let run = start_run(&h, draft(Some("echo hi")));
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    let spec_ref = status["spec_ref"]
        .as_str()
        .expect("spec_ref present")
        .to_string();

    // Break HEAD resolution: point it at a branch that does not exist.
    h.write_file(".git/HEAD", "ref: refs/heads/does-not-exist\n");

    // Evidence identity capture fails; nothing is recorded.
    let refused = h.ctl_raw(&[
        "ctl",
        "evidence",
        "collect",
        "--run",
        &run,
        "--requirements",
        "R1",
        "--json",
    ]);
    assert_eq!(refused["ok"], json!(false), "{refused}");
    let msg = refused["error"]["message"]
        .as_str()
        .expect("error message present");
    assert!(msg.contains("repository identity"), "{refused}");
    let log = h
        .read_home_file_containing(&format!("{run}/events.jsonl"))
        .expect("run event log exists");
    assert!(
        !log.contains("evidence_recorded"),
        "failed identity capture still recorded evidence:\n{log}"
    );

    // The planning packet reports nulls plus warnings, not empty strings.
    let packet = h.ctl(&["ctl", "packet", "planning", "--spec", &spec_ref, "--json"]);
    assert_eq!(packet["workspace"]["git"]["head"], json!(null), "{packet}");
    let warnings = packet["workspace"]["warnings"]
        .as_array()
        .expect("warnings present");
    assert!(!warnings.is_empty(), "{packet}");
}

/// B: an unreadable diff halts `run next` (the provenance scan) instead of
/// silently scanning nothing.
#[test]
fn unreadable_diff_halts_run_next() {
    let h = Harness::new();
    let run = start_run(&h, draft(None));

    // Claim and hand off so the provenance scan targets the task diff.
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
    h.write_file("src/T1.txt", "work\n");
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
        &json!({ "task": "T1", "round": 1, "summary": "did it" }),
    );

    // Corrupt only the task's stored baseline (leaving the run's base commit
    // intact, so out-of-band detection stays quiet) so the scan's diff cannot
    // be produced.
    let log_path = h
        .home_path_containing(&format!("{run}/events.jsonl"))
        .expect("run event log path");
    let text = fs_err::read_to_string(&log_path).expect("read log");
    let mut out = String::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let mut v: Value = serde_json::from_str(line).expect("json line");
        if v["type"] == json!("task_claimed") {
            v["baseline_commit"] = json!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
        }
        out.push_str(&serde_json::to_string(&v).expect("serialize line"));
        out.push('\n');
    }
    fs_err::write(&log_path, out).expect("write log");

    let refused = h.ctl_raw(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(refused["ok"], json!(false), "{refused}");
}

/// Minimal spec whose command evidence declares a fail-before/pass-after
/// control.
fn controlled_draft(command: &str) -> Value {
    json!({
        "goal": "fix the bug", "scope": { "in": ["x"] }, "risk": "high",
        "requirements": [{ "id": "R1", "statement": "the bug is fixed",
            "evidence": [{ "id": "E1", "kind": "command", "command": command,
                           "control": "fail_before_pass_after" }] }],
        "tasks": [{ "id": "T1", "title": "the fix", "requirements": ["R1"] }]
    })
}

fn collect_r1(h: &Harness, run: &str) -> Value {
    let collected = h.ctl(&[
        "ctl",
        "evidence",
        "collect",
        "--run",
        run,
        "--requirements",
        "R1",
        "--json",
    ]);
    collected["evidence"][0].clone()
}

// D: the control proves fail-before/pass-after — the command fails against
// the pinned baseline in an isolated worktree and passes against the
// candidate — without the live worktree being reset or mutated.
#[test]
fn control_proves_fail_before_pass_after_without_touching_the_live_worktree() {
    let h = Harness::new();
    let base = h.git(&["rev-parse", "HEAD"]).trim().to_string();
    let run = start_run(
        &h,
        controlled_draft("git ls-files --error-unmatch fixed.txt"),
    );

    // The "fix": the file exists (tracked) in the candidate state only.
    h.write_file("fixed.txt", "fixed\n");
    h.git(&["add", "fixed.txt"]);
    let head_before = h.git(&["rev-parse", "HEAD"]).trim().to_string();

    let ev = collect_r1(&h, &run);
    assert_eq!(ev["exit_code"], json!(0), "{ev}");
    let control = &ev["control"];
    assert_eq!(control["kind"], json!("fail_before_pass_after"));
    assert_eq!(control["status"], json!("passed"), "{control}");
    assert_eq!(control["baseline"]["commit"], json!(base));
    assert_ne!(control["baseline"]["exit_code"], json!(0));
    assert_eq!(control["baseline"]["contained"], json!(true));
    assert!(
        control["baseline"]["stdout_hash"]
            .as_str()
            .expect("baseline stdout hash")
            .starts_with("sha256:")
    );

    // Isolation: the worktree lived outside the repo, was removed, and its
    // absence verified.
    assert_eq!(control["isolation"]["cleanup"], json!("removed"));
    let isolation_path = control["isolation"]["path"]
        .as_str()
        .expect("isolation path recorded");
    assert!(
        !std::path::Path::new(isolation_path).exists(),
        "isolation worktree must be gone: {isolation_path}"
    );
    assert!(
        !isolation_path.starts_with(h.repo_path().to_str().expect("utf8 repo path")),
        "isolation worktree must not live inside the repo: {isolation_path}"
    );

    // The live worktree is untouched: same HEAD, same dirty set.
    assert_eq!(h.git(&["rev-parse", "HEAD"]).trim(), head_before);
    assert_eq!(h.git(&["status", "--porcelain"]).trim(), "A  fixed.txt");
    assert_eq!(
        h.git(&["worktree", "list"]).trim().lines().count(),
        1,
        "no worktree registration may remain"
    );

    // The baseline artifact is stored and the control replays from the log.
    let baseline_artifact = control["baseline"]["artifact"]
        .as_str()
        .expect("baseline artifact recorded");
    assert!(
        h.read_home_file_containing(baseline_artifact).is_some(),
        "baseline artifact stored: {baseline_artifact}"
    );
    let log = h
        .read_home_file_containing(&format!("{run}/events.jsonl"))
        .expect("run events log stored");
    let recorded: Value = log
        .lines()
        .find(|l| l.contains("\"type\":\"evidence_recorded\""))
        .and_then(|l| serde_json::from_str(l).ok())
        .expect("an evidence_recorded event");
    assert_eq!(recorded["evidence"]["control"]["status"], json!("passed"));
}

// D: a command that also passes on the baseline cannot distinguish before
// from after — the control fails even though the candidate run passed.
#[test]
fn control_fails_when_the_baseline_also_passes() {
    let h = Harness::new();
    let run = start_run(&h, controlled_draft("echo vacuous"));
    let ev = collect_r1(&h, &run);
    assert_eq!(ev["exit_code"], json!(0));
    assert_eq!(ev["control"]["status"], json!("failed"), "{ev}");
    assert!(
        ev["control"]["note"]
            .as_str()
            .expect("control note present")
            .contains("baseline command passed")
    );
    assert_eq!(ev["control"]["isolation"]["cleanup"], json!("removed"));
}

// D: a failing candidate fails the control (and the evidence itself).
#[test]
fn control_fails_when_the_candidate_fails() {
    let h = Harness::new();
    let run = start_run(&h, controlled_draft("exit 3"));
    let ev = collect_r1(&h, &run);
    assert_eq!(ev["exit_code"], json!(3));
    assert_eq!(ev["control"]["status"], json!("failed"), "{ev}");
    assert!(
        ev["control"]["note"]
            .as_str()
            .expect("control note present")
            .contains("candidate command failed")
    );
}

// D: an unavailable baseline environment is blocked — never passed, never a
// synthesized failure.
#[test]
fn unavailable_baseline_is_blocked_not_failed() {
    let h = Harness::new();
    let run = start_run(&h, controlled_draft("echo hi"));

    // Point the stored run baseline at a commit that does not exist, so the
    // isolation worktree cannot be created.
    let log_path = h
        .home_path_containing(&format!("{run}/events.jsonl"))
        .expect("run event log path");
    let text = fs_err::read_to_string(&log_path).expect("read log");
    let mut out = String::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let mut v: Value = serde_json::from_str(line).expect("json line");
        if v["type"] == json!("run_started") {
            v["base_commit"] = json!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
        }
        out.push_str(&serde_json::to_string(&v).expect("serialize line"));
        out.push('\n');
    }
    fs_err::write(&log_path, out).expect("write log");

    let ev = collect_r1(&h, &run);
    assert_eq!(ev["exit_code"], json!(0), "candidate still runs: {ev}");
    assert_eq!(ev["control"]["status"], json!("blocked"), "{ev}");
    assert!(
        ev["control"]["note"]
            .as_str()
            .expect("control note present")
            .contains("baseline worktree setup failed")
    );
    assert!(ev["control"]["baseline"].is_null());
}

// D: control is linted — only valid on kind: command, only known values.
#[test]
fn control_lint_rejects_non_command_and_unknown_values() {
    let h = Harness::new();
    let start = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "lint test", "title": "Lint test" }),
    );
    let spec_ref = start["spec_ref"]
        .as_str()
        .expect("spec_ref present")
        .to_string();
    let drafted = h.ctl_in(
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
                { "id": "R1", "statement": "s",
                  "evidence": [{ "id": "E1", "kind": "review",
                                 "control": "fail_before_pass_after" }] },
                { "id": "R2", "statement": "s",
                  "evidence": [{ "id": "E1", "kind": "command", "command": "echo hi",
                                 "control": "prove_it_backwards" }] }
            ],
            "tasks": [{ "id": "T1", "title": "t", "requirements": ["R1", "R2"] }]
        }),
    );
    assert_eq!(drafted["lint"]["clean"], json!(false), "{drafted}");
    let codes: Vec<&str> = drafted["lint"]["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .filter_map(|f| f["code"].as_str())
        .collect();
    assert!(codes.contains(&"control_on_non_command"), "{codes:?}");
    assert!(codes.contains(&"invalid_control"), "{codes:?}");
}
