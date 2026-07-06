//! M1 done-when: a fake harness drives request → spec card → approve → task
//! → snapshot → verified review packet through the real CLI, and a killed
//! session resumes correctly because the snapshot is real.

#![expect(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::single_match_else,
    reason = "integration-test helpers assert on known-shape CLI/JSON output; indexing, expect, and panic are the idiomatic way a test fails and never reach shipped code"
)]

mod common;

use common::Harness;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;

/// A clean, lint-passing draft: two review-evidence requirements over two
/// tasks.
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
    let spec_ref = start["spec_ref"]
        .as_str()
        .expect("spec_ref present")
        .to_string();

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
        .expect("findings is an array")
        .iter()
        .map(|f| f["code"].as_str().expect("finding code present"))
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
    let revision = approved["approved_revision"]
        .as_str()
        .expect("approved_revision present")
        .to_string();
    (spec_ref, revision)
}

/// Drive the autonomous loop to a terminal directive; return the final
/// directive.
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
        let lease = d["lease"]["token"]
            .as_str()
            .expect("lease token present")
            .to_string();
        match d["action"].as_str().expect("action present") {
            "claim_task" => {
                let task = d["subject"]["task"].as_str().expect("task present");
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
                let task = d["subject"]["task"]
                    .as_str()
                    .expect("task present")
                    .to_string();
                let round = d["round"]["current"]
                    .as_u64()
                    .expect("round current present");
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
                    .expect("requirements is an array")
                    .iter()
                    .map(|v| v.as_str().expect("requirement id present").to_string())
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
                            let id = ev["id"].as_str().expect("evidence id present").to_string();
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
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();
    assert!(
        started["branch"]
            .as_str()
            .expect("branch present")
            .starts_with("speccy/")
    );

    let terminal = drive_loop(&h, &run);
    assert_eq!(terminal["action"], json!("await_human_gate"));
    assert_eq!(terminal["subject"]["gate"], json!("ship_decision"));
    assert_eq!(terminal["run_state"], json!("verified"));

    // The review packet renders the verified result.
    let review = h.ctl(&["ctl", "packet", "review", "--run", &run, "--json"]);
    let md = review["markdown"].as_str().expect("markdown present");
    assert!(
        md.contains("verified"),
        "review packet missing verified: {md}"
    );
    assert_eq!(review["buckets"]["proven"], json!(2));

    // run status confirms verified.
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    assert_eq!(status["run_state"], json!("verified"));

    // Real snapshots: the branch has two integrated commits under the Speccy
    // identity.
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
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

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

/// SCHEMAS § Directive requires absent optionals to serialize as explicit
/// `null`, not be omitted. `contains_key` proves the key is present (mere
/// `is_null()` cannot distinguish an omitted key from a null one).
#[test]
fn claim_task_directive_serializes_absent_fields_as_null() {
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
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("claim_task"));

    let obj = d.as_object().expect("directive is an object");
    assert!(obj.contains_key("round"), "round key must be present: {d}");
    assert!(
        obj["round"].is_null(),
        "round must be null at claim_task: {d}"
    );

    let subject = d["subject"].as_object().expect("subject is an object");
    assert!(
        subject.contains_key("gate"),
        "gate key must be present: {d}"
    );
    assert!(
        subject["gate"].is_null(),
        "gate must be null at claim_task: {d}"
    );
    assert!(
        subject.contains_key("personas"),
        "personas key must be present: {d}"
    );
    assert!(
        subject.contains_key("requirements"),
        "requirements key must be present: {d}"
    );
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
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Claim T1, then "worker" edits files but the session dies before handoff.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
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
        "ctl", "task", "claim", "--run", &run, "--task", &task, "--agent", "a", "--lease", &lease,
        "--json",
    ]);
    let worker = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(worker["action"], json!("dispatch_worker"));
    h.write_file("src/partial.txt", "half-done work\n");

    // Fresh session (same agent reconnecting): run next resumes to the same
    // directive. A *different* agent within the lease TTL is covered by the
    // lease-contention test in trust.rs.
    let resumed = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(resumed["action"], json!("dispatch_worker"));
    assert_eq!(resumed["subject"]["task"], json!(task));
    assert_eq!(resumed["round"]["current"], json!(1));
    // The uncommitted diff is still there — nothing was rolled back.
    assert!(
        !h.git(&["status", "--porcelain", "--untracked-files=all"])
            .trim()
            .is_empty()
    );
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
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    // Take T1 all the way to integrated (real snapshot commit).
    let mut integrated_t1 = false;
    for _ in 0..30 {
        let d = h.ctl(&[
            "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
        ]);
        let lease = d["lease"]["token"]
            .as_str()
            .expect("lease token present")
            .to_string();
        // Detect the integration transition in applied_transitions.
        for t in d["applied_transitions"]
            .as_array()
            .expect("applied_transitions is an array")
        {
            if t["subject"] == json!("task:T1") && t["to"] == json!("integrated") {
                assert!(
                    t["snapshot"].is_string(),
                    "integration must record a snapshot: {t}"
                );
                integrated_t1 = true;
            }
        }
        match d["action"].as_str().expect("action present") {
            "claim_task" => {
                let task = d["subject"]["task"].as_str().expect("task present");
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
                let task = d["subject"]["task"]
                    .as_str()
                    .expect("task present")
                    .to_string();
                let round = d["round"]["current"]
                    .as_u64()
                    .expect("round current present");
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
                    .expect("requirements is an array")
                    .iter()
                    .map(|v| v.as_str().expect("requirement id present").to_string())
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

/// A spec with multiple carry-forward decisions surfaces all of them as
/// `hints` in another spec's planning packet (not just the first).
#[test]
fn planning_packet_surfaces_all_carry_forward_hints() {
    let h = Harness::new();

    // Spec A: two carry-forward decisions.
    let a = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "session hardening", "title": "Session hardening" }),
    );
    let a_ref = a["spec_ref"]
        .as_str()
        .expect("spec_ref present")
        .to_string();
    for note in ["tokens stored hashed", "rotate session on login"] {
        h.ctl_in(
            &[
                "ctl",
                "spec",
                "record-decision",
                "--spec",
                &a_ref,
                "--input",
                "-",
                "--json",
            ],
            &json!({ "type": "scope_change", "revision": "spec_rev_001",
                     "note": note, "carry_forward": true }),
        );
    }

    // Spec B: the one being planned.
    let b = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "passwordless login" }),
    );
    let b_ref = b["spec_ref"]
        .as_str()
        .expect("spec_ref present")
        .to_string();

    let planning = h.ctl(&["ctl", "packet", "planning", "--spec", &b_ref, "--json"]);
    let candidates = planning["prior_context_candidates"]
        .as_array()
        .expect("prior_context_candidates array");
    let entry = candidates
        .iter()
        .find(|c| c["spec_ref"] == json!(a_ref))
        .expect("spec A surfaces as prior context");
    let hints = entry["hints"].as_array().expect("hints array");
    assert_eq!(
        hints.len(),
        2,
        "both carry-forward decisions surface: {entry}"
    );
    let joined = hints
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(joined.contains("tokens stored hashed"), "{joined}");
    assert!(joined.contains("rotate session on login"), "{joined}");
}

/// P4: back-to-back `run next` calls with ample TTL remaining do not rewrite
/// the lease — same token and same expiry (DESIGN § Run Lease renewal slack).
#[test]
fn lease_renewal_is_skipped_while_ample_ttl_remains() {
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
    let run = started["run_id"]
        .as_str()
        .expect("run_id present")
        .to_string();

    let d1 = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let d2 = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(
        d1["lease"]["expires_at"], d2["lease"]["expires_at"],
        "lease was renewed despite ample TTL: {d1} vs {d2}"
    );
    assert_eq!(d1["lease"]["token"], d2["lease"]["token"]);
}
