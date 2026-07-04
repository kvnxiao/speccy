//! M1 done-when: a fake harness drives request → spec card → approve → task
//! → snapshot → verified review packet through the real CLI, and a killed
//! session resumes correctly because the snapshot is real.

mod common;

use std::collections::HashMap;

use common::Harness;
use serde_json::{json, Value};

/// A clean, lint-passing draft: two review-evidence requirements over two tasks.
fn clean_draft() -> Value {
    json!({
        "goal": "Users can sign in through single-use emailed magic links",
        "scope": { "in": ["request link by email", "expiry"], "out": ["OAuth"] },
        "risk": "high",
        "requirements": [
            { "id": "R-AUTH-001", "statement": "A user can request a magic link by email.",
              "evidence": [ { "id": "E1", "kind": "review", "note": "diff review" } ] },
            { "id": "R-AUTH-002", "statement": "An expired link creates no session.",
              "evidence": [ { "id": "E1", "kind": "review", "note": "diff review" } ] }
        ],
        "tasks": [
            { "id": "T1", "title": "Token model + endpoints", "requirements": ["R-AUTH-001"] },
            { "id": "T2", "title": "Expired-link UI", "requirements": ["R-AUTH-002"] }
        ]
    })
}

/// Create + draft + approve a spec; return `(spec_ref, revision)`.
fn approved_spec(h: &Harness) -> (String, String) {
    let start = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "passwordless login", "title": "Passwordless login" }),
    );
    let spec_ref = start["spec_ref"].as_str().unwrap().to_string();

    // A dirty draft first: invalid risk + a requirement missing evidence.
    let dirty = h.ctl_in(
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
            "goal": "g", "scope": {"in": ["x"]}, "risk": "medium",
            "requirements": [{ "id": "R-AUTH-001", "statement": "s" }],
            "tasks": [{ "id": "T1", "requirements": ["R-AUTH-001"] }]
        }),
    );
    assert_eq!(dirty["lint"]["clean"], json!(false));
    let codes: Vec<&str> = dirty["lint"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"invalid_risk_tier"), "{codes:?}");
    assert!(codes.contains(&"missing_evidence_request"), "{codes:?}");

    // Approval refused while dirty.
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
        &json!({ "type": "approve", "revision": "spec_rev_001-draft", "approved_in_prose": "go" }),
    );
    assert_eq!(refused["ok"], json!(false));
    assert_eq!(refused["error"]["code"], json!("validation_failed"));

    // Patch to a clean draft.
    let patched = h.ctl_in(
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
        &json!({ "set": clean_draft() }),
    );
    assert_eq!(patched["lint"]["clean"], json!(true), "{patched}");

    // Approve.
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
    assert_eq!(approved["spec_status"], json!("approved"));
    let revision = approved["approved_revision"].as_str().unwrap().to_string();
    (spec_ref, revision)
}

/// Drive the autonomous loop to a terminal directive; return the final directive.
fn drive_loop(h: &Harness, run: &str) -> Value {
    let mut req_evidence: HashMap<String, String> = HashMap::new();
    for _ in 0..80 {
        let d = h.ctl(&[
            "ctl",
            "run",
            "next",
            "--run",
            run,
            "--agent",
            "claude:sess_1",
            "--json",
        ]);
        let lease = d["lease"]["token"].as_str().unwrap().to_string();
        match d["action"].as_str().unwrap() {
            "claim_task" => {
                let task = d["subject"]["task"].as_str().unwrap();
                h.ctl(&[
                    "ctl",
                    "task",
                    "claim",
                    "--run",
                    run,
                    "--task",
                    task,
                    "--agent",
                    "claude:sess_1",
                    "--lease",
                    &lease,
                    "--json",
                ]);
            }
            "dispatch_worker" => {
                let task = d["subject"]["task"].as_str().unwrap().to_string();
                let round = d["round"]["current"].as_u64().unwrap();
                h.ctl(&[
                    "ctl", "packet", "task", "--run", run, "--task", &task, "--json",
                ]);
                // Worker edits its task scope (real diff for the snapshot).
                h.write_file(&format!("src/{task}_r{round}.txt"), "work\n");
                let reqs = d["subject"]["requirements"].clone();
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
                    &json!({
                        "task": task, "round": round, "summary": "implemented task",
                        "files_touched": [format!("src/{task}_r{round}.txt")],
                        "requirements_claimed": reqs
                    }),
                );
            }
            "dispatch_verifier" => {
                let reqs: Vec<String> = d["subject"]["requirements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                let joined = reqs.join(",");
                h.ctl(&[
                    "ctl",
                    "packet",
                    "verification",
                    "--run",
                    run,
                    "--requirements",
                    &joined,
                    "--json",
                ]);
                let mut updates = Vec::new();
                for r in &reqs {
                    let ev_id = match req_evidence.get(r) {
                        Some(id) => id.clone(),
                        None => {
                            let ev = h.ctl_in(
                                &["ctl", "evidence", "record", "--run", run, "--input", "-", "--json"],
                                &json!({ "requirement": r, "kind": "review",
                                         "collected_by": "claude:verifier", "note": "diff reviewed" }),
                            );
                            let id = ev["id"].as_str().unwrap().to_string();
                            req_evidence.insert(r.clone(), id.clone());
                            id
                        }
                    };
                    updates
                        .push(json!({ "requirement": r, "status": "passed", "evidence": [ev_id] }));
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
            "await_human_gate" | "halt" => return d,
            other => panic!("unexpected action {other}: {d}"),
        }
    }
    panic!("loop did not terminate");
}

#[test]
fn full_loop_reaches_verified_with_review_packet() {
    let h = Harness::new();
    let (spec_ref, revision) = approved_spec(&h);

    // Planning packet is deterministic and real.
    let planning = h.ctl(&["ctl", "packet", "planning", "--spec", &spec_ref, "--json"]);
    assert_eq!(planning["request"], json!("passwordless login"));

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
    assert_eq!(started["run_state"], json!("implementing"));
    let run = started["run_id"].as_str().unwrap().to_string();
    assert!(started["branch"].as_str().unwrap().starts_with("speccy/"));

    let terminal = drive_loop(&h, &run);
    assert_eq!(terminal["action"], json!("await_human_gate"));
    assert_eq!(terminal["subject"]["gate"], json!("ship_decision"));
    assert_eq!(terminal["run_state"], json!("verified"));

    // The review packet renders the verified result.
    let review = h.ctl(&["ctl", "packet", "review", "--run", &run, "--json"]);
    let md = review["markdown"].as_str().unwrap();
    assert!(
        md.contains("verified"),
        "review packet missing verified: {md}"
    );
    assert_eq!(review["buckets"]["proven"], json!(2));

    // run status confirms verified.
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    assert_eq!(status["run_state"], json!("verified"));

    // Real snapshots: the branch has two integrated commits under the Speccy identity.
    let log = h.git(&["log", "--pretty=%an %s"]);
    assert_eq!(
        log.matches("Speccy").count(),
        2,
        "expected 2 snapshot commits: {log}"
    );
    assert!(log.contains("T1 integrated"), "{log}");
    assert!(log.contains("T2 integrated"), "{log}");

    // Human review packet renders too.
    let human = h.human(&["review", &spec_ref]);
    assert!(
        human.contains("Ready to ship") || human.contains("verified"),
        "{human}"
    );
}

#[test]
fn run_next_is_idempotent() {
    let h = Harness::new();
    let (spec_ref, revision) = approved_spec(&h);
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
    let run = started["run_id"].as_str().unwrap().to_string();

    let d1 = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let d2 = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    // Same directive apart from the per-call lease and applied_transitions.
    assert_eq!(d1["action"], d2["action"]);
    assert_eq!(d1["subject"], d2["subject"]);
    assert_eq!(d1["action"], json!("claim_task"));
    assert_eq!(d2["applied_transitions"], json!([]));
}

#[test]
fn resume_mid_build_returns_same_worker_directive() {
    let h = Harness::new();
    let (spec_ref, revision) = approved_spec(&h);
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
    let run = started["run_id"].as_str().unwrap().to_string();

    // Claim T1, then "worker" edits files but the session dies before handoff.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let task = d["subject"]["task"].as_str().unwrap().to_string();
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", &run, "--task", &task, "--agent", "a", "--lease", &lease,
        "--json",
    ]);
    let worker = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(worker["action"], json!("dispatch_worker"));
    h.write_file("src/partial.txt", "half-done work\n");

    // Fresh session (new process): run next resumes to the same directive.
    let resumed = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "b", "--json",
    ]);
    assert_eq!(resumed["action"], json!("dispatch_worker"));
    assert_eq!(resumed["subject"]["task"], json!(task));
    assert_eq!(resumed["round"]["current"], json!(1));
    // The uncommitted diff is still there — nothing was rolled back.
    assert!(!h
        .git(&["status", "--porcelain", "--untracked-files=all"])
        .trim()
        .is_empty());
}

#[test]
fn integrated_task_snapshot_survives_resume() {
    let h = Harness::new();
    let (spec_ref, revision) = approved_spec(&h);
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
    let run = started["run_id"].as_str().unwrap().to_string();

    // Take T1 all the way to integrated (real snapshot commit).
    let mut integrated_t1 = false;
    for _ in 0..30 {
        let d = h.ctl(&[
            "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
        ]);
        let lease = d["lease"]["token"].as_str().unwrap().to_string();
        // Detect the integration transition in applied_transitions.
        for t in d["applied_transitions"].as_array().unwrap() {
            if t["subject"] == json!("task:T1") && t["to"] == json!("integrated") {
                assert!(
                    t["snapshot"].is_string(),
                    "integration must record a snapshot: {t}"
                );
                integrated_t1 = true;
            }
        }
        match d["action"].as_str().unwrap() {
            "claim_task" => {
                let task = d["subject"]["task"].as_str().unwrap();
                if task == "T2" {
                    // T1 already integrated; resume landed us at T2. Done.
                    assert!(
                        integrated_t1,
                        "T1 should have integrated before T2 is claimed"
                    );
                    // Idempotent: a second call still says claim T2, applies nothing.
                    let again = h.ctl(&[
                        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
                    ]);
                    assert_eq!(again["subject"]["task"], json!("T2"));
                    assert_eq!(again["applied_transitions"], json!([]));
                    return;
                }
                h.ctl(&[
                    "ctl", "task", "claim", "--run", &run, "--task", task, "--agent", "a",
                    "--lease", &lease, "--json",
                ]);
            }
            "dispatch_worker" => {
                let task = d["subject"]["task"].as_str().unwrap().to_string();
                let round = d["round"]["current"].as_u64().unwrap();
                h.write_file(&format!("src/{task}_r{round}.txt"), "work\n");
                let reqs = d["subject"]["requirements"].clone();
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
                    &json!({ "task": task, "round": round, "summary": "done",
                             "files_touched": [], "requirements_claimed": reqs }),
                );
            }
            "dispatch_verifier" => {
                let reqs: Vec<String> = d["subject"]["requirements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                let mut updates = Vec::new();
                for r in &reqs {
                    let ev = h.ctl_in(
                        &[
                            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
                        ],
                        &json!({ "requirement": r, "kind": "review", "collected_by": "v" }),
                    );
                    updates.push(json!({ "requirement": r, "status": "passed",
                                         "evidence": [ev["id"]] }));
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
            other => panic!("unexpected {other}"),
        }
    }
    panic!("never reached T2 claim");
}
