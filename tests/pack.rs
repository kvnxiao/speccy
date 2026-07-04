//! M4 done-when: golden render tests pass for both targets, install-twice is
//! idempotent, --check catches hash drift, and a roster change adds/removes
//! persona files on re-install.

mod common;

use common::Harness;
use speccy::config::{Persona, ProjectConfig};
use speccy::render::{render_pack, Harness as Target};

fn find<'a>(files: &'a [speccy::render::ManagedFile], suffix: &str) -> &'a str {
    files
        .iter()
        .find(|f| f.path.ends_with(suffix))
        .map(|f| f.contents.as_str())
        .unwrap()
}

#[test]
fn golden_claude_pack() {
    let files = render_pack(Target::Claude, &ProjectConfig::default()).unwrap();
    insta::assert_snapshot!("claude-skill-plan", find(&files, "speccy-plan/SKILL.md"));
    insta::assert_snapshot!("claude-agent-worker", find(&files, "speccy-worker.md"));
    insta::assert_snapshot!("claude-agent-verifier", find(&files, "speccy-verifier.md"));
    insta::assert_snapshot!(
        "claude-reviewer-security",
        find(&files, "speccy-reviewer-security.md")
    );
}

#[test]
fn golden_codex_pack() {
    let files = render_pack(Target::Codex, &ProjectConfig::default()).unwrap();
    insta::assert_snapshot!(
        "codex-skill-implement",
        find(&files, "speccy-implement/SKILL.md")
    );
    insta::assert_snapshot!("codex-agent-worker", find(&files, "speccy-worker.toml"));
}

#[test]
fn golden_reviewer_with_model() {
    let mut config = ProjectConfig::default();
    config.review.personas = vec![Persona {
        name: "defects".into(),
        model: Some("opus".into()),
        min_risk: None,
    }];
    let claude = render_pack(Target::Claude, &config).unwrap();
    insta::assert_snapshot!(
        "claude-reviewer-model",
        find(&claude, "speccy-reviewer-defects.md")
    );
    let codex = render_pack(Target::Codex, &config).unwrap();
    insta::assert_snapshot!(
        "codex-reviewer-model",
        find(&codex, "speccy-reviewer-defects.toml")
    );
}

#[test]
fn install_is_idempotent_and_check_passes() {
    let h = Harness::new();
    h.mkdir(".claude");
    let (out1, ok1) = h.output(&["install", "--yes"]);
    assert!(ok1, "{out1}");
    assert!(h.exists(".claude/skills/speccy-plan/SKILL.md"));
    assert!(h.exists(".speccy/project.yaml"));
    assert!(h.exists(".speccy/pack-lock.yaml"));

    // Second install writes no managed prose and reports everything up to date.
    let (out2, ok2) = h.output(&["install", "--yes"]);
    assert!(ok2, "{out2}");
    assert!(out2.contains("ok    "), "{out2}");

    // --check passes on a clean install.
    let (_c, check_ok) = h.output(&["install", "--check"]);
    assert!(check_ok);

    // The defensive .gitignore block is present.
    assert!(h.read(".gitignore").contains("!.speccy/project.yaml"));
}

#[test]
fn check_catches_drift() {
    let h = Harness::new();
    h.mkdir(".claude");
    h.output(&["install", "--yes"]);
    // Locally edit a managed file.
    h.write_file(".claude/agents/speccy-worker.md", "hand-edited\n");
    let (_out, check_ok) = h.output(&["install", "--check"]);
    assert!(!check_ok, "--check should fail on drift");
}

#[test]
fn roster_change_adds_and_removes_persona_files() {
    let h = Harness::new();
    h.mkdir(".claude");
    h.output(&["install", "--yes"]);
    assert!(h.exists(".claude/agents/speccy-reviewer-style.md"));
    assert!(!h.exists(".claude/agents/speccy-reviewer-perf.md"));

    // Swap `style` for a new `perf` persona.
    h.write_file(
        ".speccy/project.yaml",
        "risk_default: standard\nreview:\n  personas:\n    - name: spec-fidelity\n    - name: defects\n    - name: security\n    - name: perf\n",
    );
    let (out, ok) = h.output(&["install", "--yes"]);
    assert!(ok, "{out}");
    assert!(
        h.exists(".claude/agents/speccy-reviewer-perf.md"),
        "perf added"
    );
    assert!(
        !h.exists(".claude/agents/speccy-reviewer-style.md"),
        "style removed"
    );
}

#[test]
fn update_merges_local_edits_when_upstream_unchanged() {
    let h = Harness::new();
    h.mkdir(".claude");
    h.output(&["install", "--yes"]);
    // Local edit, then --update at the same pack version. The three-way merge
    // sees no upstream change (base == new render), so it preserves the local
    // file and stages nothing.
    h.write_file(".claude/agents/speccy-worker.md", "my local version\n");
    let (out, ok) = h.output(&["install", "--update", "--yes"]);
    assert!(ok, "{out}");
    assert_eq!(
        h.read(".claude/agents/speccy-worker.md"),
        "my local version\n"
    );
    assert!(
        !h.exists(".speccy/pack-updates"),
        "no conflict staged when upstream is unchanged"
    );
}
