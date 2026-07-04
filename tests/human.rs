//! M3 done-when: review renders the right state-aware packet and infers the
//! current spec, rework returns the run to implementing with an appended
//! RT<n>, and accept is idempotent after landing.

mod common;

use common::{approve_minimal, drive_to_gate, Harness};
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
    let run = started["run_id"].as_str().unwrap().to_string();
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
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
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

    // Accept records the landing; the spec becomes accepted and leaves default list.
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
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
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
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", &run, "--task", "RT1", "--agent", "a", "--lease", &lease,
        "--json",
    ]);
    let packet = h.ctl(&[
        "ctl", "packet", "task", "--run", &run, "--task", "RT1", "--json",
    ]);
    assert!(packet["seed_feedback"]
        .as_str()
        .unwrap()
        .contains("standard error layout"));
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
    let run = started["run_id"].as_str().unwrap().to_string();
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
        .unwrap()
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
    let run = started["run_id"].as_str().unwrap().to_string();

    // Fail R1 through all three task repair rounds → escalation.
    let mut escalated = None;
    for _ in 0..40 {
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
                let round = d["round"]["current"].as_u64().unwrap();
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
    let md = packet["markdown"].as_str().unwrap();
    assert!(md.contains("R1"), "{md}");
    assert!(md.contains("Recommended: amend the spec"), "{md}");

    // Waiving R1 resolves it and resumes the run.
    let lease = d["lease"]["token"].as_str().unwrap().to_string();
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
