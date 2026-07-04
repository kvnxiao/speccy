//! M5: the non-happy paths from WALKTHROUGH.md have integration coverage —
//! amendment-supersede at a gate, the critical accepted-risk confirmation gate,
//! the command allow policy refusing at collect, a blocked requirement
//! escalating directly, and a resource cap parking the run.

mod common;

use common::{approve_minimal, Harness};
use serde_json::{json, Value};

/// Approve a spec from an explicit draft body; returns `(spec_ref, revision)`.
fn approve(h: &Harness, title: &str, draft: Value) -> (String, String) {
    let start = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "x", "title": title }),
    );
    let spec_ref = start["spec_ref"].as_str().unwrap().to_string();
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
        approved["approved_revision"].as_str().unwrap().to_string(),
    )
}

fn claim_and_handoff(h: &Harness, run: &str, task: &str) -> String {
    let d = h.ctl(&["ctl", "run", "next", "--run", run, "--agent", "a", "--json"]);
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", run, "--task", task, "--agent", "a", "--lease", &lease,
        "--json",
    ]);
    let d = h.ctl(&["ctl", "run", "next", "--run", run, "--agent", "a", "--json"]);
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
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
        .unwrap()
        .to_string()
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
    let run1 = started["run_id"].as_str().unwrap().to_string();
    let branch = started["branch"].as_str().unwrap().to_string();

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
    let run = started["run_id"].as_str().unwrap().to_string();

    // Resolve R1 as review-only (accepted risk) at both task and run scope.
    for _ in 0..10 {
        let d = h.ctl(&[
            "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
        ]);
        let lease = d["lease"]["token"].as_str().unwrap().to_string();
        match d["action"].as_str().unwrap() {
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
                let lease = d["lease"]["token"].as_str().unwrap().to_string();
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
    let run = started["run_id"].as_str().unwrap().to_string();

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
    assert!(refused["error"]["message"]
        .as_str()
        .unwrap()
        .contains("allow pattern"));
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
    let run = started["run_id"].as_str().unwrap().to_string();

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
        .unwrap()
        .iter()
        .find(|t| t["id"] == json!("T1"))
        .unwrap();
    assert_eq!(
        t1["round"],
        json!(1),
        "blocked must not spend a repair round"
    );
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
    let run = started["run_id"].as_str().unwrap().to_string();
    // Two tasks exceed the cap of one → the run parks at an escalated policy gate.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["run_state"], json!("escalated"), "{d}");
}
