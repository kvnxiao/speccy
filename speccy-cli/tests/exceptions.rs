//! M5: the non-happy paths from WALKTHROUGH.md have integration coverage —
//! amendment-supersede at a gate, the critical accepted-risk confirmation gate,
//! the command allow policy refusing at collect, a blocked requirement
//! escalating directly, and a resource cap parking the run.

#![expect(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    reason = "integration-test helpers assert on known-shape CLI/JSON output; indexing, expect, and panic are the idiomatic way a test fails and never reach shipped code"
)]

mod common;

use common::Harness;
use common::approve_minimal;
use common::drive_to_gate;
use serde_json::Value;
use serde_json::json;

/// Approve a spec from an explicit draft body; returns `(spec_ref, revision)`.
fn approve(h: &Harness, title: &str, draft: Value) -> (String, String) {
    let start = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "x", "title": title }),
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
        &draft,
    );
    assert_eq!(drafted["lint"]["clean"], json!(true), "{drafted}");
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
    (
        spec_ref,
        approved["approved_revision"]
            .as_str()
            .expect("approved_revision present")
            .to_string(),
    )
}

fn claim_and_handoff(h: &Harness, run: &str, task: &str) -> String {
    let d = h.ctl(&["ctl", "run", "next", "--run", run, "--agent", "a", "--json"]);
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", run, "--task", task, "--agent", "a", "--lease", &lease,
        "--json",
    ]);
    let d = h.ctl(&["ctl", "run", "next", "--run", run, "--agent", "a", "--json"]);
    let lease = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();
    h.write_file(&format!("src/{task}.txt"), "work\n");
    h.ctl_in(
        &[
            "ctl",
            "task",
            "record-handoff",
            "--run",
            run,
            "--lease",
            &lease,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "task": task, "round": 1, "summary": "did it", "requirements_claimed": ["R1"] }),
    );
    // Return the current lease for the following verification set-status.
    h.ctl(&["ctl", "run", "next", "--run", run, "--agent", "a", "--json"])["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string()
}

/// Drive a single-task/single-requirement run through the task gate (R1 passed,
/// T1 integrated) to the run-scope verifier; returns the lease there.
fn to_run_gate(h: &Harness, run: &str) -> String {
    let lease = claim_and_handoff(h, run, "T1");
    let ev = h.ctl_in(
        &[
            "ctl", "evidence", "record", "--run", run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "kind": "review", "collected_by": "v" }),
    );
    h.ctl_in(
        &[
            "ctl",
            "requirement",
            "set-status",
            "--run",
            run,
            "--lease",
            &lease,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "updates": [{ "requirement": "R1", "status": "passed", "evidence": [ev["id"]] }] }),
    );
    // T1 integrates; the next directive is the run-scope verifier.
    let d = h.ctl(&["ctl", "run", "next", "--run", run, "--agent", "a", "--json"]);
    assert_eq!(d["action"], json!("dispatch_verifier"), "{d}");
    assert_eq!(d["round"]["scope"], json!("run"), "{d}");
    d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string()
}

/// Set a single requirement's status under a live lease (test convenience).
fn set_status(h: &Harness, run: &str, lease: &str, update: Value) {
    h.ctl_in(
        &[
            "ctl",
            "requirement",
            "set-status",
            "--run",
            run,
            "--lease",
            lease,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "updates": [update] }),
    );
}

/// Rewrite the `ts` of matching events in the run's stored log; the test owns
/// `SPECCY_HOME`. Each edit is `(type, optional "to" filter, new RFC3339 ts)`.
fn rewrite_event_ts(h: &Harness, run: &str, edits: &[(&str, Option<&str>, &str)]) {
    let path = h
        .home_path_containing(&format!("{run}/events.jsonl"))
        .expect("run events log path");
    let text = fs_err::read_to_string(&path).expect("read log");
    let mut out = String::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut v: Value = serde_json::from_str(line).expect("json line");
        for (ty, to_filter, new_ts) in edits {
            if v["type"] == json!(*ty) && to_filter.is_none_or(|t| v["to"] == json!(t)) {
                v["ts"] = json!(*new_ts);
            }
        }
        out.push_str(&serde_json::to_string(&v).expect("serialize line"));
        out.push('\n');
    }
    fs_err::write(&path, out).expect("write log");
}

/// Record a run-scoped gate decision under a live lease (test convenience).
fn gate_decision(h: &Harness, run: &str, lease: &str, decision: Value) -> Value {
    h.ctl_in(
        &[
            "ctl",
            "run",
            "record-decision",
            "--run",
            run,
            "--lease",
            lease,
            "--input",
            "-",
            "--json",
        ],
        &decision,
    )
}

#[test]
fn amendment_supersede_cancels_the_parked_run_and_reuses_the_branch() {
    let h = Harness::new();
    let (spec_ref, rev1) = approve_minimal(&h, "Amend me");
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev1,
        "--json",
    ]);
    let run1 = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();
    let branch = started["branch"]
        .as_str()
        .expect("branch present")
        .to_string();

    // Open revision 2 (seeded from the approved revision) and approve it as a
    // superseding amendment of run1.
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
        &json!({ "set": {} }),
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
        &json!({ "type": "approve", "revision": "spec_rev_002-draft",
                 "approved_in_prose": "go",
                 "supersedes": { "run_id": run1 } }),
    );
    assert_eq!(approved["approved_revision"], json!("spec_rev_002"));
    assert_eq!(approved["superseded_run"], json!(run1));

    // run1 is cancelled atomically.
    let status = h.ctl(&["ctl", "run", "status", "--run", &run1, "--json"]);
    assert_eq!(status["run_state"], json!("cancelled"));

    // The superseding run reuses the same branch (reconciles rather than redoes).
    let run2 = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        "spec_rev_002",
        "--json",
    ]);
    assert_eq!(run2["branch"], json!(branch));
}

#[test]
fn critical_accepted_risk_confirmation_gate() {
    let h = Harness::new();
    let (spec_ref, rev) = approve(
        &h,
        "Critical",
        json!({
            "goal": "g", "scope": { "in": ["x"] }, "risk": "critical",
            "requirements": [{ "id": "R1", "statement": "s",
                "evidence": [{ "id": "E1", "kind": "review" }] }],
            "tasks": [{ "id": "T1", "requirements": ["R1"] }]
        }),
    );
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ]);
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Resolve R1 as review-only (accepted risk) at both task and run scope.
    for _ in 0..10 {
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
                h.write_file("src/x.txt", "work\n");
                h.ctl_in(
                    &["ctl", "task", "record-handoff", "--run", &run, "--lease", &lease, "--input", "-", "--json"],
                    &json!({ "task": "T1", "round": 1, "summary": "did it", "requirements_claimed": ["R1"] }),
                );
            }
            "dispatch_verifier" => {
                let ev = h.ctl_in(
                    &[
                        "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
                    ],
                    &json!({ "requirement": "R1", "kind": "review", "collected_by": "v" }),
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
                    &json!({ "updates": [{ "requirement": "R1", "status": "review_passed",
                        "evidence": [ev["id"]], "residual_risk": "not proven locally" }] }),
                );
            }
            "await_human_gate" => {
                // Critical spec parks at the accepted-risk confirmation gate.
                assert_eq!(
                    d["subject"]["gate"],
                    json!("accepted_risk_confirmation"),
                    "{d}"
                );
                assert_eq!(d["run_state"], json!("verifying"));
                let lease = d["lease"]["token"]
                    .as_str()
                    .expect("lease token present")
                    .to_string();
                h.ctl_in(
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
                    &json!({ "type": "confirm_accepted_risk", "reason": "risk accepted" }),
                );
                // After confirmation the run reaches verified.
                let done = h.ctl(&[
                    "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
                ]);
                assert_eq!(done["run_state"], json!("verified"), "{done}");
                assert_eq!(done["subject"]["gate"], json!("ship_decision"));
                return;
            }
            other => panic!("unexpected {other}: {d}"),
        }
    }
    panic!("never reached the accepted-risk gate");
}

#[test]
fn command_allow_policy_refuses_at_collect() {
    let h = Harness::new();
    let (spec_ref, rev) = approve(
        &h,
        "Cmd policy",
        json!({
            "goal": "g", "scope": { "in": ["x"] }, "risk": "standard",
            "requirements": [{ "id": "R1", "statement": "s",
                "evidence": [{ "id": "E1", "kind": "command", "command": "echo hi" }] }],
            "tasks": [{ "id": "T1", "requirements": ["R1"] }]
        }),
    );
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ]);
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // A policy added after approval refuses the declared command at collect.
    h.write_file(
        ".speccy/project.yaml",
        "evidence:\n  command_policy:\n    allow:\n      - \"npm test*\"\n",
    );
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
    assert_eq!(refused["ok"], json!(false));
    assert_eq!(refused["error"]["code"], json!("validation_failed"));
    assert!(
        refused["error"]["message"]
            .as_str()
            .expect("error message present")
            .contains("allow pattern")
    );
}

#[test]
fn blocked_requirement_escalates_without_repair() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Blocked");
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ]);
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    let lease = claim_and_handoff(&h, &run, "T1");
    // The verifier marks R1 blocked (missing environment).
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
        &json!({ "updates": [{ "requirement": "R1", "status": "blocked",
                               "note": "needs staging credentials" }] }),
    );
    // Next directive escalates directly — no repair round is spent.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("await_human_gate"), "{d}");
    assert_eq!(d["subject"]["gate"], json!("escalation"));
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    let t1 = status["tasks"]
        .as_array()
        .expect("tasks array present")
        .iter()
        .find(|t| t["id"] == json!("T1"))
        .expect("T1 present");
    assert_eq!(
        t1["round"],
        json!(1),
        "blocked must not spend a repair round"
    );
}

#[test]
fn waiver_at_escalation_must_target_an_unresolved_requirement() {
    let h = Harness::new();
    let (spec_ref, rev) = approve(
        &h,
        "Scoped waiver",
        json!({
            "goal": "g", "scope": { "in": ["x"] }, "risk": "standard",
            "requirements": [
                { "id": "R1", "statement": "already proven",
                  "evidence": [{ "id": "E1", "kind": "review" }] },
                { "id": "R2", "statement": "missing setup",
                  "evidence": [{ "id": "E1", "kind": "review" }] }
            ],
            "tasks": [{ "id": "T1", "requirements": ["R1", "R2"] }]
        }),
    );
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ]);
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();
    let lease = claim_and_handoff(&h, &run, "T1");
    let ev = h.ctl_in(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "kind": "review", "collected_by": "v" }),
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
        &json!({ "updates": [
            { "requirement": "R1", "status": "passed", "evidence": [ev["id"]] },
            { "requirement": "R2", "status": "blocked", "note": "missing service" }
        ] }),
    );
    let gate = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(gate["subject"]["gate"], json!("escalation"), "{gate}");
    let lease = gate["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();

    let refused = h.ctl_in_raw(
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
        &json!({ "type": "waive", "requirement": "R1",
                 "reason": "wrong target", "residual_risk": "none" }),
    );
    assert_eq!(refused["error"]["code"], json!("invalid_transition"));
    assert!(
        refused["error"]["message"]
            .as_str()
            .expect("error message present")
            .contains("unresolved requirement")
    );
}

/// Drive a run, failing R1 the first `fail_run_gates` times a run-scope
/// verifier round is dispatched (passing everything else), until a terminal
/// directive. Returns `(final_directive, rt_tasks_created)`.
fn drive_failing_run_gate(h: &Harness, run: &str, fail_run_gates: u32) -> (Value, usize) {
    let mut failed = 0u32;
    for _ in 0..60 {
        let d = h.ctl(&["ctl", "run", "next", "--run", run, "--agent", "a", "--json"]);
        let lease = d["lease"]["token"]
            .as_str()
            .expect("lease token present")
            .to_string();
        match d["action"].as_str().expect("action present") {
            "claim_task" => {
                let task = d["subject"]["task"].as_str().expect("task present");
                h.ctl(&[
                    "ctl", "task", "claim", "--run", run, "--task", task, "--agent", "a",
                    "--lease", &lease, "--json",
                ]);
            }
            "dispatch_worker" => {
                let task = d["subject"]["task"]
                    .as_str()
                    .expect("task present")
                    .to_string();
                let round = d["round"]["current"]
                    .as_u64()
                    .expect("round current present");
                h.write_file(&format!("src/{task}_r{round}.txt"), "work\n");
                h.ctl_in(
                    &[
                        "ctl",
                        "task",
                        "record-handoff",
                        "--run",
                        run,
                        "--lease",
                        &lease,
                        "--input",
                        "-",
                        "--json",
                    ],
                    &json!({ "task": task, "round": round, "summary": "did it" }),
                );
            }
            "dispatch_verifier" => {
                let scope = d["round"]["scope"]
                    .as_str()
                    .expect("round scope present")
                    .to_string();
                let reqs: Vec<String> = d["subject"]["requirements"]
                    .as_array()
                    .expect("requirements array present")
                    .iter()
                    .map(|v| v.as_str().expect("requirement id is a string").to_string())
                    .collect();
                if scope == "run" && failed < fail_run_gates {
                    failed += 1;
                    // Final validation demotes R1: record a blocking finding and
                    // fail it, forcing a run-level repair round.
                    let f = h.ctl_in(
                        &[
                            "ctl", "finding", "record", "--run", run, "--input", "-", "--json",
                        ],
                        &json!({ "requirement": "R1", "severity": "blocking",
                                 "note": "regression at integration", "recorded_by": "v" }),
                    );
                    h.ctl_in(
                        &[
                            "ctl",
                            "requirement",
                            "set-status",
                            "--run",
                            run,
                            "--lease",
                            &lease,
                            "--input",
                            "-",
                            "--json",
                        ],
                        &json!({ "updates": [{ "requirement": "R1", "status": "failed",
                                               "findings": [f["id"]] }] }),
                    );
                } else {
                    let mut updates = Vec::new();
                    for r in &reqs {
                        let ev = h.ctl_in(
                            &[
                                "ctl", "evidence", "record", "--run", run, "--input", "-", "--json",
                            ],
                            &json!({ "requirement": r, "kind": "review", "collected_by": "v" }),
                        );
                        updates.push(
                            json!({ "requirement": r, "status": "passed", "evidence": [ev["id"]] }),
                        );
                    }
                    h.ctl_in(
                        &[
                            "ctl",
                            "requirement",
                            "set-status",
                            "--run",
                            run,
                            "--lease",
                            &lease,
                            "--input",
                            "-",
                            "--json",
                        ],
                        &json!({ "updates": updates }),
                    );
                }
            }
            _ => {
                let status = h.ctl(&["ctl", "run", "status", "--run", run, "--json"]);
                let rts = status["tasks"]
                    .as_array()
                    .expect("tasks array present")
                    .iter()
                    .filter(|t| {
                        t["id"]
                            .as_str()
                            .expect("task id is a string")
                            .starts_with("RT")
                    })
                    .count();
                return (d, rts);
            }
        }
    }
    panic!("loop did not terminate");
}

#[test]
fn run_level_repair_loop_reproves_then_verifies() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Run repair");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Fail the first run-gate review; the controller spawns an RT repair task,
    // loops through implementing, and re-verifies to `verified`.
    let (d, rts) = drive_failing_run_gate(&h, &run, 1);
    assert_eq!(d["action"], json!("await_human_gate"), "{d}");
    assert_eq!(d["subject"]["gate"], json!("ship_decision"), "{d}");
    assert_eq!(d["run_state"], json!("verified"));
    assert_eq!(
        rts, 1,
        "exactly one run-level repair task should be created"
    );
}

#[test]
fn run_level_repair_cap_exhaustion_escalates() {
    let h = Harness::new();
    h.write_file(".speccy/project.yaml", "caps:\n  run_review_rounds: 2\n");
    h.git(&["add", "-A"]);
    h.git(&["commit", "-m", "policy"]);
    let (spec_ref, rev) = approve_minimal(&h, "Run repair cap");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Fail every run-gate review. With cap 2 the run does one repair round
    // (RT1) then escalates when the cap is exhausted.
    let (d, rts) = drive_failing_run_gate(&h, &run, 99);
    assert_eq!(d["action"], json!("await_human_gate"), "{d}");
    assert_eq!(d["subject"]["gate"], json!("escalation"), "{d}");
    assert_eq!(rts, 1, "one repair round consumed before the cap escalated");
}

#[test]
fn max_tasks_cap_parks_the_run() {
    let h = Harness::new();
    h.write_file(".speccy/project.yaml", "caps:\n  max_tasks: 1\n");
    // Commit the policy so run start sees a clean worktree.
    h.git(&["add", "-A"]);
    h.git(&["commit", "-m", "policy"]);
    let (spec_ref, rev) = approve(
        &h,
        "Too many tasks",
        json!({
            "goal": "g", "scope": { "in": ["x"] }, "risk": "standard",
            "requirements": [
                { "id": "R1", "statement": "a", "evidence": [{ "id": "E1", "kind": "review" }] },
                { "id": "R2", "statement": "b", "evidence": [{ "id": "E1", "kind": "review" }] }
            ],
            "tasks": [
                { "id": "T1", "requirements": ["R1"] },
                { "id": "T2", "requirements": ["R2"] }
            ]
        }),
    );
    let started = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ]);
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();
    // Two tasks exceed the cap of one → the run parks at an escalated policy gate.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["run_state"], json!("escalated"), "{d}");
}

#[test]
fn run_start_on_existing_branch_records_branch_tip_as_base() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Reuse branch");
    let run1 = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();
    // Drive run1 to verified so the branch advances past its base with snapshots.
    let d = drive_to_gate(&h, &run1);
    assert_eq!(d["run_state"], json!("verified"), "{d}");

    // Return to main so HEAD differs from the run branch tip, then reuse the
    // branch for a second run.
    h.git(&["checkout", "main"]);
    let run2 = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // The reused run's base is the branch tip captured *after* checkout, so the
    // first directive claims the first task — it does not misread the branch
    // tip as an out-of-band commit and escalate.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run2, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("claim_task"), "{d}");
    assert_eq!(d["run_state"], json!("implementing"), "{d}");
    assert_eq!(d["applied_transitions"], json!([]), "{d}");
}

#[test]
fn out_of_band_commit_parks_the_run() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Out of band");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Claim T1 so the run is actively implementing.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"].as_str().expect("lease").to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", &run, "--task", "T1", "--agent", "a", "--lease", &lease,
        "--json",
    ]);

    // A human commits out-of-band on the run branch.
    h.write_file("hand-edit.txt", "human change\n");
    h.git(&["add", "-A"]);
    h.git(&["commit", "-m", "human out-of-band commit"]);

    // The run parks at an escalated policy gate; the out-of-band escalation
    // takes NO snapshot (a snapshot commit would bury the human's commit).
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("await_human_gate"), "{d}");
    assert_eq!(d["subject"]["gate"], json!("escalation"), "{d}");
    assert_eq!(d["run_state"], json!("escalated"), "{d}");
    let t = &d["applied_transitions"][0];
    assert_eq!(t["to"], json!("escalated"), "{d}");
    assert_eq!(
        t["snapshot"],
        json!(null),
        "out-of-band escalation must take no snapshot: {d}"
    );
}

#[test]
fn resource_cap_escalation_commits_the_inflight_diff() {
    let h = Harness::new();
    h.write_file(".speccy/project.yaml", "caps:\n  max_tasks: 0\n");
    h.git(&["add", "-A"]);
    h.git(&["commit", "-m", "policy"]);
    let (spec_ref, rev) = approve_minimal(&h, "Cap snapshot");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // An in-flight edit exists when the resource cap trips.
    h.write_file("src/inflight.txt", "work in progress\n");

    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["run_state"], json!("escalated"), "{d}");
    assert_eq!(d["subject"]["gate"], json!("escalation"), "{d}");
    // Unlike out-of-band, a resource-cap escalation commits the in-flight diff
    // as a labeled snapshot.
    let t = &d["applied_transitions"][0];
    assert_eq!(t["to"], json!("escalated"), "{d}");
    assert!(
        t["snapshot"].as_str().is_some_and(|s| !s.is_empty()),
        "resource-cap escalation must commit a snapshot: {d}"
    );
    // The worktree is clean afterward — the snapshot captured the edit.
    let porcelain = h.git(&["status", "--porcelain"]);
    assert!(
        porcelain.trim().is_empty(),
        "worktree not clean after snapshot: {porcelain:?}"
    );
}

#[test]
fn accepted_risk_gate_waits_until_all_requirements_resolved() {
    let h = Harness::new();
    let (spec_ref, rev) = approve(
        &h,
        "Critical gate ordering",
        json!({
            "goal": "g", "scope": { "in": ["x"] }, "risk": "critical",
            "requirements": [
                { "id": "R1", "statement": "accepted risk",
                  "evidence": [{ "id": "E1", "kind": "review" }] },
                { "id": "R2", "statement": "must be proven",
                  "evidence": [{ "id": "E1", "kind": "review" }] }
            ],
            "tasks": [{ "id": "T1", "requirements": ["R1", "R2"] }]
        }),
    );
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // R1 is always accepted risk (review_passed). R2 fails the first run-gate
    // review, forcing a run-repair round; the accepted-risk gate must not fire
    // while R2 is unresolved, only after R2 is re-proven.
    let mut r2_failed = false;
    for _ in 0..40 {
        let d = h.ctl(&[
            "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
        ]);
        let lease = d["lease"]["token"].as_str().expect("lease").to_string();
        match d["action"].as_str().expect("action present") {
            "claim_task" => {
                let task = d["subject"]["task"].as_str().expect("task present");
                h.ctl(&[
                    "ctl", "task", "claim", "--run", &run, "--task", task, "--agent", "a",
                    "--lease", &lease, "--json",
                ]);
            }
            "dispatch_worker" => {
                let task = d["subject"]["task"]
                    .as_str()
                    .expect("task present")
                    .to_string();
                let round = d["round"]["current"].as_u64().expect("round present");
                h.write_file(&format!("src/{task}_r{round}.txt"), "work\n");
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
                    &json!({ "task": task, "round": round, "summary": "did it" }),
                );
            }
            "dispatch_verifier" => {
                let scope = d["round"]["scope"]
                    .as_str()
                    .expect("scope present")
                    .to_string();
                let reqs: Vec<String> = d["subject"]["requirements"]
                    .as_array()
                    .expect("reqs present")
                    .iter()
                    .map(|v| v.as_str().expect("req string").to_string())
                    .collect();
                let mut updates = Vec::new();
                for r in &reqs {
                    if r == "R2" && scope == "run" && !r2_failed {
                        r2_failed = true;
                        let f = h.ctl_in(
                            &[
                                "ctl", "finding", "record", "--run", &run, "--input", "-", "--json",
                            ],
                            &json!({ "requirement": "R2", "severity": "blocking",
                                     "note": "regression at integration", "recorded_by": "v" }),
                        );
                        updates.push(
                            json!({ "requirement": "R2", "status": "failed", "findings": [f["id"]] }),
                        );
                    } else {
                        let ev = h.ctl_in(
                            &[
                                "ctl", "evidence", "record", "--run", &run, "--input", "-",
                                "--json",
                            ],
                            &json!({ "requirement": r, "kind": "review", "collected_by": "v" }),
                        );
                        if r == "R1" {
                            updates.push(json!({ "requirement": "R1", "status": "review_passed",
                                "evidence": [ev["id"]], "residual_risk": "accepted, not proven locally" }));
                        } else {
                            updates.push(
                                json!({ "requirement": r, "status": "passed", "evidence": [ev["id"]] }),
                            );
                        }
                    }
                }
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
                    &json!({ "updates": updates }),
                );
            }
            "await_human_gate" => {
                assert_eq!(
                    d["subject"]["gate"],
                    json!("accepted_risk_confirmation"),
                    "{d}"
                );
                assert!(
                    r2_failed,
                    "gate appeared before R2 ever failed at the run gate"
                );
                // By the time the gate fires, R2 has been re-proven — the gate
                // never pre-empted the unresolved requirement.
                let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
                let r2 = status["requirements"]
                    .as_array()
                    .expect("reqs")
                    .iter()
                    .find(|r| r["id"] == json!("R2"))
                    .expect("R2 present")
                    .clone();
                assert_eq!(
                    r2["status"],
                    json!("passed"),
                    "gate pre-empted an unresolved R2: {status}"
                );
                h.ctl_in(
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
                    &json!({ "type": "confirm_accepted_risk", "reason": "risk accepted" }),
                );
                let done = h.ctl(&[
                    "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
                ]);
                assert_eq!(done["run_state"], json!("verified"), "{done}");
                assert_eq!(done["subject"]["gate"], json!("ship_decision"), "{done}");
                return;
            }
            other => panic!("unexpected {other}: {d}"),
        }
    }
    panic!("never reached the accepted-risk gate");
}

#[test]
fn waiver_at_escalation_gate_completes_the_run_without_a_new_review_round() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Waive completes");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    let lease = to_run_gate(&h, &run);
    // Block R1 at the run gate → run-level escalation.
    set_status(
        &h,
        &run,
        &lease,
        json!({ "requirement": "R1", "status": "blocked", "note": "needs staging" }),
    );
    let gate = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(gate["subject"]["gate"], json!("escalation"), "{gate}");
    let lease = gate["lease"]["token"].as_str().expect("lease").to_string();

    // Waive R1 → every requirement resolved; the run resumes to verifying.
    let waived = gate_decision(
        &h,
        &run,
        &lease,
        json!({ "type": "waive", "requirement": "R1",
                "reason": "accept the gap", "residual_risk": "manual check pending" }),
    );
    assert_eq!(waived["run_state"], json!("verifying"), "{waived}");

    // One `run next` completes to verified — no new review round is opened.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["run_state"], json!("verified"), "{d}");
    assert_eq!(d["subject"]["gate"], json!("ship_decision"), "{d}");

    // Exactly one implementing→verifying transition ever recorded: the resume
    // re-entered verifying without opening a round.
    let log = h
        .read_home_file_containing(&format!("{run}/events.jsonl"))
        .expect("run events.jsonl present");
    let entered = log
        .matches("run_state_transitioned\",\"to\":\"verifying\"")
        .count();
    assert_eq!(entered, 1, "resume opened a new review round:\n{log}");
}

#[test]
fn provide_setup_reopens_the_current_review_round() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Provide setup run gate");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    let lease = to_run_gate(&h, &run);
    set_status(
        &h,
        &run,
        &lease,
        json!({ "requirement": "R1", "status": "blocked", "note": "needs staging" }),
    );
    let gate = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(gate["subject"]["gate"], json!("escalation"), "{gate}");
    let lease = gate["lease"]["token"].as_str().expect("lease").to_string();

    gate_decision(
        &h,
        &run,
        &lease,
        json!({ "type": "provide_setup", "reason": "staging is up now" }),
    );

    // Resume re-opens the SAME review round (current 1), not a fresh round.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("dispatch_verifier"), "{d}");
    assert_eq!(d["round"]["scope"], json!("run"), "{d}");
    assert_eq!(d["round"]["current"], json!(1), "{d}");
}

#[test]
fn cap_exhausted_escalation_resumes_within_cap() {
    let h = Harness::new();
    h.write_file(".speccy/project.yaml", "caps:\n  run_review_rounds: 1\n");
    h.git(&["add", "-A"]);
    h.git(&["commit", "-m", "policy"]);
    let (spec_ref, rev) = approve_minimal(&h, "Cap resume");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    let lease = to_run_gate(&h, &run);
    // Fail R1 at run-gate round 1; with cap 1 this exhausts the cap → escalate.
    let f = h.ctl_in(
        &[
            "ctl", "finding", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "severity": "blocking",
                 "note": "regression", "recorded_by": "v" }),
    );
    set_status(
        &h,
        &run,
        &lease,
        json!({ "requirement": "R1", "status": "failed", "findings": [f["id"]] }),
    );
    let gate = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(gate["subject"]["gate"], json!("escalation"), "{gate}");
    let lease = gate["lease"]["token"].as_str().expect("lease").to_string();

    gate_decision(
        &h,
        &run,
        &lease,
        json!({ "type": "provide_setup", "reason": "retry" }),
    );

    // Resume stays within the cap: round { current: 1, max: 1 }, never 2 > 1.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("dispatch_verifier"), "{d}");
    assert_eq!(d["round"]["current"], json!(1), "{d}");
    assert_eq!(d["round"]["max"], json!(1), "{d}");
}

#[test]
fn provide_setup_at_task_escalation_redispatches_the_worker() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Provide setup task gate");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Block R1 at the task gate → task-level escalation.
    let lease = claim_and_handoff(&h, &run, "T1");
    set_status(
        &h,
        &run,
        &lease,
        json!({ "requirement": "R1", "status": "blocked", "note": "needs staging" }),
    );
    let gate = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(gate["subject"]["gate"], json!("escalation"), "{gate}");
    let lease = gate["lease"]["token"].as_str().expect("lease").to_string();

    gate_decision(
        &h,
        &run,
        &lease,
        json!({ "type": "provide_setup", "reason": "staging is up now" }),
    );

    // The stuck task re-opens to a worker dispatch at its same round — not an
    // immediate re-escalation.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("dispatch_worker"), "{d}");
    assert_eq!(d["subject"]["task"], json!("T1"), "{d}");
    assert_eq!(d["round"]["current"], json!(1), "{d}");
}

#[test]
fn waiver_at_task_escalation_integrates_the_task() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Waive task gate");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    let lease = claim_and_handoff(&h, &run, "T1");
    set_status(
        &h,
        &run,
        &lease,
        json!({ "requirement": "R1", "status": "blocked", "note": "needs staging" }),
    );
    let gate = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(gate["subject"]["gate"], json!("escalation"), "{gate}");
    let lease = gate["lease"]["token"].as_str().expect("lease").to_string();

    // Waiving R1 fully resolves T1; the next `run next` integrates it.
    gate_decision(
        &h,
        &run,
        &lease,
        json!({ "type": "waive", "requirement": "R1",
                "reason": "accept the gap", "residual_risk": "manual check pending" }),
    );
    h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    let t1 = status["tasks"]
        .as_array()
        .expect("tasks array present")
        .iter()
        .find(|t| t["id"] == json!("T1"))
        .expect("T1 present")
        .clone();
    assert_eq!(t1["status"], json!("integrated"), "{status}");
}

#[test]
fn task_repair_cap_exhaustion_escalates() {
    let h = Harness::new();
    h.write_file(".speccy/project.yaml", "caps:\n  task_repair_rounds: 1\n");
    h.git(&["add", "-A"]);
    h.git(&["commit", "-m", "policy"]);
    let (spec_ref, rev) = approve_minimal(&h, "Task cap");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Fail R1 at task-gate round 1; with a repair cap of 1 the task escalates
    // instead of opening a round 2.
    let lease = claim_and_handoff(&h, &run, "T1");
    let f = h.ctl_in(
        &[
            "ctl", "finding", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "severity": "blocking",
                 "note": "regression", "recorded_by": "v" }),
    );
    set_status(
        &h,
        &run,
        &lease,
        json!({ "requirement": "R1", "status": "failed", "findings": [f["id"]] }),
    );
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("await_human_gate"), "{d}");
    assert_eq!(d["subject"]["gate"], json!("escalation"), "{d}");
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    let t1 = status["tasks"]
        .as_array()
        .expect("tasks array present")
        .iter()
        .find(|t| t["id"] == json!("T1"))
        .expect("T1 present")
        .clone();
    assert_eq!(
        t1["round"],
        json!(1),
        "cap-exhausted task must not open round 2: {status}"
    );
}

#[test]
fn wall_clock_cap_parks_the_run() {
    let h = Harness::new();
    h.write_file(
        ".speccy/project.yaml",
        "caps:\n  max_run_wall_clock_minutes: 0\n",
    );
    h.git(&["add", "-A"]);
    h.git(&["commit", "-m", "policy"]);
    let (spec_ref, rev) = approve_minimal(&h, "Wall clock");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Backdate the run start far into the past → large accumulated active time.
    rewrite_event_ts(&h, &run, &[("run_started", None, "2020-01-01T00:00:00Z")]);
    // An in-flight edit is present when the cap trips.
    h.write_file("src/wip.txt", "work\n");

    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["run_state"], json!("escalated"), "{d}");
    assert_eq!(d["subject"]["gate"], json!("escalation"), "{d}");
    let t = &d["applied_transitions"][0];
    assert_eq!(t["to"], json!("escalated"), "{d}");
    assert!(
        t["snapshot"].as_str().is_some_and(|s| !s.is_empty()),
        "wall-clock escalation must commit a snapshot: {d}"
    );
}

#[test]
fn wall_clock_cap_excludes_parked_gate_time() {
    let h = Harness::new();
    h.write_file(
        ".speccy/project.yaml",
        "caps:\n  max_run_wall_clock_minutes: 60\n",
    );
    h.git(&["add", "-A"]);
    h.git(&["commit", "-m", "policy"]);
    let (spec_ref, rev) = approve_minimal(&h, "Parked exclusion");
    let run = h.ctl(&[
        "ctl",
        "run",
        "start",
        "--spec",
        &spec_ref,
        "--revision",
        &rev,
        "--json",
    ])["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Drive to a task-level escalation (blocked R1).
    let lease = claim_and_handoff(&h, &run, "T1");
    set_status(
        &h,
        &run,
        &lease,
        json!({ "requirement": "R1", "status": "blocked", "note": "needs staging" }),
    );
    let gate = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(gate["subject"]["gate"], json!("escalation"), "{gate}");
    let lease = gate["lease"]["token"].as_str().expect("lease").to_string();

    // Rewrite the log so the run was active only ~2 minutes, then parked for
    // years: start at 2020-01-01T00:00:00Z, escalated at 2020-01-01T00:02:00Z.
    // The old wall-clock computation (now − start) would blow past the 60m cap;
    // active time (2m) is well under it.
    rewrite_event_ts(
        &h,
        &run,
        &[
            ("run_started", None, "2020-01-01T00:00:00Z"),
            (
                "run_state_transitioned",
                Some("escalated"),
                "2020-01-01T00:02:00Z",
            ),
        ],
    );

    // Resume; the long parked gap must not count toward the wall-clock cap, so
    // the run redispatches the worker instead of re-escalating.
    gate_decision(
        &h,
        &run,
        &lease,
        json!({ "type": "provide_setup", "reason": "staging is up now" }),
    );
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("dispatch_worker"), "{d}");
    assert_eq!(d["subject"]["task"], json!("T1"), "{d}");
}
