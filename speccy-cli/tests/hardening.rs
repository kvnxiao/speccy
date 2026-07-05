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
