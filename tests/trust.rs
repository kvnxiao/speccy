//! M2 done-when: a run cannot reach verified without recorded evidence
//! (covered in e2e), pasted command output is refused, a second agent gets
//! lease_held, and the provenance scan flags a seeded leak in a product file.

mod common;

use std::thread::sleep;
use std::time::Duration;

use common::Harness;
use serde_json::{json, Value};

/// Minimal single-task spec; `cmd_evidence` swaps R1's proof to a shell command.
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
    let spec_ref = start["spec_ref"].as_str().unwrap().to_string();
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
    let revision = approved["approved_revision"].as_str().unwrap().to_string();
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
    started["run_id"].as_str().unwrap().to_string()
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
    assert!(b["error"]["message"].as_str().unwrap().contains("claude:A"));
}

#[test]
fn mutating_op_requires_the_live_lease() {
    let h = Harness::new();
    let run = start_run(&h, draft(None));
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let task = d["subject"]["task"].as_str().unwrap();
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
    let task = d["subject"]["task"].as_str().unwrap().to_string();
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
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
    assert!(resumed["resume"]["dirty_diff"]["files"].as_u64().unwrap() >= 1);
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
    assert!(ev["stdout_hash"].as_str().unwrap().starts_with("sha256:"));

    // Pasting command output through evidence record is refused.
    let refused = h.ctl_in_raw(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "kind": "command", "collected_by": "agent" }),
    );
    assert_eq!(refused["error"]["code"], json!("validation_failed"));
    assert!(refused["error"]["message"]
        .as_str()
        .unwrap()
        .contains("evidence collect"));
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
        "ctl", "evidence", "collect", "--run", &run, "--requirements", "R1", "--json",
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
        "ctl", "evidence", "collect", "--run", &run, "--requirements", "R1", "--json",
    ]);
    let note = collected["evidence"][0]["note"].as_str().unwrap_or("");
    assert!(note.contains("truncated"), "expected truncation note, got {note:?}");
}

#[test]
fn provenance_leak_in_product_file_blocks_integration() {
    let h = Harness::new();
    let run = start_run(&h, draft(None));

    // Claim + build T1; the worker leaks a Speccy identifier into a product file.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", &run, "--task", "T1", "--agent", "a", "--lease", &lease,
        "--json",
    ]);
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("dispatch_worker"));
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
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
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
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
        packet["provenance_scan"]["hits"].as_u64().unwrap() >= 1,
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
        .unwrap()
        .iter()
        .any(|t| t["to"] == json!("integrated"));
    assert!(
        !integrated,
        "task must not integrate with an unresolved leak"
    );
}
