//! M6 hardening: crash-matrix resume, concurrency stress, exhaustive golden
//! renders, a full lifecycle through the real CLI, and doctor drift detection.

#![expect(
    clippy::too_many_lines,
    reason = "end-to-end lifecycle tests drive the full loop in one function; splitting them obscures the scenario"
)]

mod common;

use common::Harness;
use common::approve_minimal;
use common::drive_to_gate;
use serde_json::json;
use speccy_core::config::ProjectConfig;
use speccy_core::render::Harness as Target;
use speccy_core::render::render_pack;
use std::collections::HashSet;
use std::thread;

/// At every loop phase, a second `run next` (a fresh session) returns the same
/// directive — the whole crash-recovery story is `run next` idempotency.
#[test]
fn crash_matrix_run_next_is_idempotent_at_every_phase() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Crash matrix");
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

    for _ in 0..40 {
        let d1 = h.ctl(&[
            "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
        ]);
        // Simulate a crash: a fresh session re-derives the directive.
        let d2 = h.ctl(&[
            "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
        ]);
        assert_eq!(
            d1["action"], d2["action"],
            "action drifted on resume: {d1} vs {d2}"
        );
        assert_eq!(d1["subject"], d2["subject"], "subject drifted on resume");
        assert_eq!(d1["round"], d2["round"], "round drifted on resume");
        assert_eq!(
            d2["applied_transitions"],
            json!([]),
            "resume re-applied a transition"
        );

        let lease = d2["lease"]["token"]
            .as_str()
            .expect("lease token present")
            .to_string();
        match d2["action"].as_str().expect("action present") {
            "claim_task" => {
                let task = d2["subject"]["task"].as_str().expect("task present");
                h.ctl(&[
                    "ctl", "task", "claim", "--run", &run, "--task", task, "--agent", "a",
                    "--lease", &lease, "--json",
                ]);
            }
            "dispatch_worker" => {
                let task = d2["subject"]["task"]
                    .as_str()
                    .expect("task present")
                    .to_string();
                let round = d2["round"]["current"]
                    .as_u64()
                    .expect("round current present");
                h.write_file(&format!("src/{task}_{round}.txt"), "work\n");
                let reqs = d2["subject"]["requirements"].clone();
                h.ctl_in(
                    &["ctl", "task", "record-handoff", "--run", &run, "--lease", &lease, "--input", "-", "--json"],
                    &json!({ "task": task, "round": round, "summary": "did it", "requirements_claimed": reqs }),
                );
            }
            "dispatch_verifier" => {
                let reqs: Vec<String> = d2["subject"]["requirements"]
                    .as_array()
                    .expect("requirements array")
                    .iter()
                    .map(|v| v.as_str().expect("requirement is string").to_string())
                    .collect();
                let mut updates = Vec::new();
                for r in &reqs {
                    let ev = h.ctl_in(
                        &[
                            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
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
                assert_eq!(d2["run_state"], json!("verified"));
                return;
            }
            other => panic!("unexpected {other}"),
        }
    }
    panic!("loop did not terminate");
}

/// Concurrent lease-free `finding record` writers all land without corrupting
/// the event log (the per-workspace store lock serializes appends).
#[test]
fn concurrent_finding_records_all_land() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Concurrency");
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

    let ids: HashSet<String> = thread::scope(|scope| {
        let handles: Vec<_> = (0..12)
            .map(|i| {
                let h = &h;
                let run = run.as_str();
                scope.spawn(move || {
                    let f = h.ctl_in(
                        &[
                            "ctl", "finding", "record", "--run", run, "--input", "-", "--json",
                        ],
                        &json!({ "requirement": "R1", "severity": "advisory",
                                 "note": format!("finding {i}"), "recorded_by": "v" }),
                    );
                    f["id"].as_str().expect("finding id present").to_string()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("thread joins"))
            .collect()
    });

    // All 12 findings got distinct IDs and the run still replays cleanly.
    assert_eq!(ids.len(), 12);
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    assert_eq!(status["run_state"], json!("implementing"));
}

/// Concurrent command-evidence collection serializes on the command lock;
/// both collections land.
#[test]
fn concurrent_command_collect_serializes() {
    let h = Harness::new();
    let start = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "x", "title": "Cmd concurrency" }),
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
            "requirements": [{ "id": "R1", "statement": "s",
                "evidence": [{ "id": "E1", "kind": "command", "command": "echo hi" }] }],
            "tasks": [{ "id": "T1", "requirements": ["R1"] }]
        }),
    );
    h.ctl_in(
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

    let exit_codes: Vec<i64> = thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let h = &h;
                let run = run.as_str();
                scope.spawn(move || {
                    let out = h.ctl(&[
                        "ctl",
                        "evidence",
                        "collect",
                        "--run",
                        run,
                        "--requirements",
                        "R1",
                        "--json",
                    ]);
                    out["evidence"][0]["exit_code"]
                        .as_i64()
                        .expect("exit_code present")
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("thread joins"))
            .collect()
    });
    assert_eq!(exit_codes, vec![0, 0]);
    // The run still replays cleanly after concurrent command execution.
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    assert_eq!(status["run_state"], json!("implementing"));
}

/// C2 regression: `run next` holds the store lock across its whole cycle, so
/// many concurrent processes (same agent, all renewing the one live lease)
/// apply the implementing→verifying derived transition exactly once. Pre-fix
/// they can each read the same pre-transition state and append `verifying`
/// more than once; post-fix that is impossible. Probabilistic before the fix,
/// never false-failing after it.
#[test]
fn concurrent_run_next_applies_derived_transitions_once() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Concurrent derive");
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

    // Drive T1 to in_review with R1 resolved, stopping *before* the `run next`
    // that would integrate it and cross into verifying.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"].as_str().expect("lease").to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", &run, "--task", "T1", "--agent", "a", "--lease", &lease,
        "--json",
    ]);
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"].as_str().expect("lease").to_string();
    h.write_file("src/t1.txt", "work\n");
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
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"].as_str().expect("lease").to_string();
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
        &json!({ "updates": [{ "requirement": "R1", "status": "passed", "evidence": [ev["id"]] }] }),
    );

    // Fan out concurrent `run next` calls, all as agent "a" so they renew the
    // single live lease and all enter the cycle at once.
    thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let h = &h;
                let run = run.as_str();
                scope.spawn(move || {
                    h.ctl_raw(&["ctl", "run", "next", "--run", run, "--agent", "a", "--json"]);
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("thread joins");
        }
    });

    // The implementing→verifying transition was applied exactly once.
    let log = h
        .read_home_file_containing(&format!("{run}/events.jsonl"))
        .expect("run events.jsonl present");
    let verifying = log.matches("\"to\":\"verifying\"").count();
    assert_eq!(
        verifying, 1,
        "verifying transition applied {verifying} times:\n{log}"
    );

    // The run sits in verifying at run-review round 1, not 2 from a double-apply.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["run_state"], json!("verifying"), "{d}");
    assert_eq!(d["round"]["current"], json!(1), "{d}");
    assert_eq!(d["round"]["scope"], json!("run"), "{d}");
}

/// The run-scope provenance scan runs over the integrated diff at the run gate
/// and blocks verification: a task-scope diff can be clean while the integrated
/// worktree carries a leak, recorded as a task-less controller finding.
#[test]
fn run_scope_provenance_scan_blocks_verification() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Run-scope provenance");
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

    // Claim + hand off T1 with a clean file, so the task-scope scan passes.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"].as_str().expect("lease").to_string();
    h.ctl(&[
        "ctl", "task", "claim", "--run", &run, "--task", "T1", "--agent", "a", "--lease", &lease,
        "--json",
    ]);
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"].as_str().expect("lease").to_string();
    h.write_file("src/t1.txt", "work\n");
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
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"].as_str().expect("lease").to_string();
    let ev = h.ctl_in(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "kind": "review", "collected_by": "v" }),
    );
    h.ctl_in(
        &[
            "ctl", "requirement", "set-status", "--run", &run, "--lease", &lease, "--input", "-",
            "--json",
        ],
        &json!({ "updates": [{ "requirement": "R1", "status": "passed", "evidence": [ev["id"]] }] }),
    );
    // T1 integrates → verifying; the run-scope verifier is next.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["round"]["scope"], json!("run"), "{d}");

    // Introduce a provenance leak into a product file after integration.
    h.write_file("src/leak.txt", &format!("// see {spec_ref} for context\n"));

    // This `run next` runs the run-scope scan over the integrated diff and
    // records a blocking, task-less provenance finding.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    let lease = d["lease"]["token"].as_str().expect("lease").to_string();
    let ev2 = h.ctl_in(
        &[
            "ctl", "evidence", "record", "--run", &run, "--input", "-", "--json",
        ],
        &json!({ "requirement": "R1", "kind": "review", "collected_by": "v" }),
    );
    h.ctl_in(
        &[
            "ctl", "requirement", "set-status", "--run", &run, "--lease", &lease, "--input", "-",
            "--json",
        ],
        &json!({ "updates": [{ "requirement": "R1", "status": "passed", "evidence": [ev2["id"]] }] }),
    );

    // With the provenance finding outstanding, the run does not verify.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_ne!(d["run_state"], json!("verified"), "{d}");

    // A task-less controller provenance finding was recorded.
    let log = h
        .read_home_file_containing(&format!("{run}/events.jsonl"))
        .expect("run events.jsonl present");
    let found = log
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .any(|v| {
            v["type"] == json!("finding_recorded")
                && v["finding"]["recorded_by"] == json!("controller:provenance-scan")
                && v["finding"]["task"].is_null()
        });
    assert!(
        found,
        "expected a task-less provenance-scan finding:\n{log}"
    );
}

/// Golden render for every managed file across both targets.
#[test]
fn golden_all_managed_files() {
    for target in [Target::Claude, Target::Codex] {
        let files = render_pack(target, &ProjectConfig::default()).expect("render_pack succeeds");
        for f in &files {
            let name = format!(
                "{}--{}",
                target_key(target),
                f.path.replace(['/', '.'], "_")
            );
            insta::assert_snapshot!(name, f.contents);
        }
    }
}

fn target_key(t: Target) -> &'static str {
    match t {
        Target::Claude => "claude",
        Target::Codex => "codex",
    }
}

/// The whole lifecycle through the real CLI: request → approve → verified →
/// ship → accept → landed.
#[test]
fn full_lifecycle_request_to_landed() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Lifecycle");
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

    let gate = drive_to_gate(&h, &run);
    assert_eq!(gate["run_state"], json!("verified"));

    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
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
        &json!({ "kind": "branch", "branch": "speccy/lifecycle" }),
    );
    assert_eq!(shipped["run_state"], json!("submitted"));

    let accepted = h.human(&["accept", &spec_ref]);
    assert!(accepted.contains("submitted -> landed"), "{accepted}");

    // The run is landed and the spec accepted; nothing remains active.
    let status = h.ctl(&["ctl", "run", "status", "--run", &run, "--json"]);
    assert_eq!(status["run_state"], json!("landed"));
    assert!(h.human(&["list"]).contains("No active specs"));
}

/// `speccy doctor` passes on a clean install and fails on pack drift.
#[test]
fn doctor_detects_pack_drift() {
    let h = Harness::new();
    h.mkdir(".claude");
    h.output(&["install", "--yes"]);
    let (_out, ok) = h.output(&["doctor"]);
    assert!(ok, "doctor should pass on a clean install");

    h.write_file(".claude/agents/speccy-worker.md", "tampered\n");
    let (report, ok) = h.output(&["doctor"]);
    assert!(!ok, "doctor should fail on drift: {report}");
    assert!(report.contains("DRIFT"), "{report}");
}

/// A1: a lease token that was cleared and reissued appends nothing. The lease
/// check happens inside the same store-lock hold as the append, so a stale
/// token can never slip a write in between check and commit.
#[test]
fn stale_lease_token_appends_nothing() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Stale lease");
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

    // Agent a acquires the lease.
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "a", "--json",
    ]);
    assert_eq!(d["action"], json!("claim_task"));
    let stale = d["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();

    // Expire it on disk (a crashed session), then let agent b clear + reissue.
    let lease_path = h
        .home_path_containing(&format!("{run}/lease.json"))
        .expect("lease file exists");
    fs_err::write(
        &lease_path,
        format!(r#"{{"token":"{stale}","agent":"a","expires_at":"2020-01-01T00:00:00Z"}}"#),
    )
    .expect("rewrite lease");
    let d = h.ctl(&[
        "ctl", "run", "next", "--run", &run, "--agent", "b", "--json",
    ]);
    assert_eq!(d["resume"]["cleared_lease"], json!("a"), "{d}");
    let fresh = d["lease"]["token"].as_str().expect("lease token present");
    assert_ne!(fresh, stale, "expired lease must be reissued");

    // The stale token must be refused and must append nothing.
    let refused = h.ctl_raw(&[
        "ctl", "task", "claim", "--run", &run, "--task", "T1", "--agent", "a", "--lease", &stale,
        "--json",
    ]);
    assert_eq!(refused["ok"], json!(false), "{refused}");
    assert_eq!(refused["error"]["code"], json!("lease_held"), "{refused}");
    let log = h
        .read_home_file_containing(&format!("{run}/events.jsonl"))
        .expect("run event log exists");
    assert!(
        !log.contains("task_claimed"),
        "stale token appended an event:\n{log}"
    );
}

/// A1: two processes racing the same live token and pre-state commit at most
/// one incompatible transition — the loser re-validates against the winner's
/// committed state inside the lock and is refused.
#[test]
fn concurrent_ship_with_one_token_commits_once() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Ship race");
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
    let gate = drive_to_gate(&h, &run);
    assert_eq!(gate["run_state"], json!("verified"), "{gate}");
    let lease = gate["lease"]["token"]
        .as_str()
        .expect("lease token present")
        .to_string();

    let envelopes: Vec<serde_json::Value> = thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let h = &h;
                let run = run.as_str();
                let lease = lease.as_str();
                scope.spawn(move || {
                    h.ctl_in_raw(
                        &[
                            "ctl",
                            "run",
                            "record-ship",
                            "--run",
                            run,
                            "--lease",
                            lease,
                            "--input",
                            "-",
                            "--json",
                        ],
                        &json!({ "kind": "none" }),
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("thread joins"))
            .collect()
    });

    let oks = envelopes.iter().filter(|e| e["ok"] == json!(true)).count();
    assert_eq!(oks, 1, "exactly one ship must commit: {envelopes:?}");
    let loser = envelopes
        .iter()
        .find(|e| e["ok"] == json!(false))
        .expect("one ship must lose the race");
    assert_eq!(
        loser["error"]["code"],
        json!("invalid_transition"),
        "{loser}"
    );

    let log = h
        .read_home_file_containing(&format!("{run}/events.jsonl"))
        .expect("run event log exists");
    assert_eq!(
        log.matches("ship_recorded").count(),
        1,
        "duplicate ship committed:\n{log}"
    );
    assert_eq!(
        log.matches("\"to\":\"submitted\"").count(),
        1,
        "duplicate submitted transition:\n{log}"
    );
}

/// A1: a stored run risk outside the closed vocabulary fails replay closed
/// (`corrupt event`) instead of silently falling back to `standard`.
#[test]
fn corrupt_stored_risk_tier_fails_replay_closed() {
    let h = Harness::new();
    let (spec_ref, rev) = approve_minimal(&h, "Corrupt risk");
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

    let log_path = h
        .home_path_containing(&format!("{run}/events.jsonl"))
        .expect("run event log exists");
    let log = fs_err::read_to_string(&log_path).expect("read run event log");
    assert!(log.contains("\"risk\":\"standard\""), "{log}");
    fs_err::write(
        &log_path,
        log.replace("\"risk\":\"standard\"", "\"risk\":\"experimental\""),
    )
    .expect("corrupt stored risk");

    let refused = h.ctl_raw(&["ctl", "run", "status", "--run", &run, "--json"]);
    assert_eq!(refused["ok"], json!(false), "{refused}");
    let message = refused["error"]["message"]
        .as_str()
        .expect("error message present");
    assert!(message.contains("corrupt event"), "{refused}");
}
