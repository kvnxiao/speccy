//! Structural lint of a spec draft (SCHEMAS § spec record-draft). Approval is
//! refused while the draft is lint-dirty (DESIGN § Planning Phase).
//!
//! This is deterministic structural checking only — never a semantic judgment
//! of the English (that belongs to the harness planner).

use std::collections::HashSet;

use crate::config::CommandPolicy;
use crate::error::Finding;
use crate::model::{EvidenceKind, RiskTier, SpecDraft};

/// Lint a draft, returning findings (empty ⇒ clean). The command-allow-policy
/// check runs only when `policy` has patterns configured (M5).
pub fn lint_draft(draft: &SpecDraft, policy: &CommandPolicy) -> Vec<Finding> {
    let mut findings = Vec::new();

    // 1. Required sections present.
    if draft
        .goal
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        findings.push(Finding::at("missing_goal", "goal", "goal is required"));
    }
    if draft.scope.is_none() {
        findings.push(Finding::at("missing_scope", "scope", "scope is required"));
    }
    match &draft.risk {
        None => findings.push(Finding::at("missing_risk", "risk", "risk is required")),
        Some(r) if RiskTier::parse(r).is_none() => findings.push(Finding::at(
            "invalid_risk_tier",
            "risk",
            format!("\"{r}\" is not one of minimal|standard|high|critical"),
        )),
        _ => {}
    }
    if draft.requirements().is_empty() {
        findings.push(Finding::at(
            "missing_requirements",
            "requirements",
            "at least one requirement is required",
        ));
    }
    if draft.tasks().is_empty() {
        findings.push(Finding::at(
            "missing_tasks",
            "tasks",
            "at least one task is required",
        ));
    }

    // 2. Requirement-level checks.
    let mut seen_reqs: HashSet<&str> = HashSet::new();
    for req in draft.requirements() {
        let base = format!("requirements[{}]", req.id);
        if !seen_reqs.insert(&req.id) {
            findings.push(Finding::at(
                "duplicate_requirement_id",
                &base,
                format!("requirement id {} is not unique", req.id),
            ));
        }
        if req
            .statement
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            findings.push(Finding::at(
                "missing_statement",
                &base,
                format!("{} has no statement", req.id),
            ));
        }
        if req.evidence.is_empty() {
            findings.push(Finding::at(
                "missing_evidence_request",
                &base,
                format!("{} has no evidence request", req.id),
            ));
        }
        let mut seen_ev: HashSet<&str> = HashSet::new();
        for ev in &req.evidence {
            let ev_path = format!("{base}.evidence[{}]", ev.id);
            if !seen_ev.insert(&ev.id) {
                findings.push(Finding::at(
                    "duplicate_evidence_request_id",
                    &ev_path,
                    format!("evidence id {} is not unique within {}", ev.id, req.id),
                ));
            }
            match ev.kind_enum() {
                None => findings.push(Finding::at(
                    "invalid_evidence_kind",
                    &ev_path,
                    format!(
                        "\"{}\" is not one of command|review|browser|api|manual",
                        ev.kind.clone().unwrap_or_default()
                    ),
                )),
                Some(EvidenceKind::Command) => {
                    let cmd = ev.command.as_deref().map(str::trim).unwrap_or("");
                    if cmd.is_empty() {
                        findings.push(Finding::at(
                            "missing_command",
                            &ev_path,
                            "kind: command requires a command string",
                        ));
                    } else if !policy.allow.is_empty() && !command_allowed(cmd, &policy.allow) {
                        findings.push(Finding::at(
                            "command_not_allowed",
                            &ev_path,
                            format!("command \"{cmd}\" matches no allow pattern"),
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    // 3. Task-level checks and coverage.
    let req_ids: HashSet<&str> = draft.requirements().iter().map(|r| r.id.as_str()).collect();
    let mut covered: HashSet<&str> = HashSet::new();
    let mut seen_tasks: HashSet<&str> = HashSet::new();
    for task in draft.tasks() {
        let base = format!("tasks[{}]", task.id);
        if !seen_tasks.insert(&task.id) {
            findings.push(Finding::at(
                "duplicate_task_id",
                &base,
                format!("task id {} is not unique", task.id),
            ));
        }
        for r in &task.requirements {
            if req_ids.contains(r.as_str()) {
                covered.insert(r.as_str());
            } else {
                findings.push(Finding::at(
                    "unknown_requirement",
                    format!("{base}.requirements"),
                    format!("task {} references unknown requirement {}", task.id, r),
                ));
            }
        }
    }
    for req in draft.requirements() {
        if !covered.contains(req.id.as_str()) {
            findings.push(Finding::at(
                "requirement_not_covered",
                format!("requirements[{}]", req.id),
                format!("{} is not covered by any task", req.id),
            ));
        }
    }

    findings
}

/// Whole-command glob match (never a prefix): `*` matches any run of chars,
/// `?` any single char (DESIGN § Acceptance Ledger command allow policy).
pub fn command_allowed(command: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| glob_match(p, command))
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    // Classic two-pointer wildcard match with backtracking on `*`.
    let (mut pi, mut ti, mut star, mut mark) = (0usize, 0usize, None::<usize>, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EvidenceRequest, Requirement, Scope, TaskDef};

    fn clean_draft() -> SpecDraft {
        SpecDraft {
            goal: Some("do the thing".into()),
            scope: Some(Scope {
                in_: vec!["a".into()],
                out: vec![],
            }),
            risk: Some("standard".into()),
            requirements: Some(vec![Requirement {
                id: "R1".into(),
                statement: Some("it works".into()),
                scenario: None,
                evidence: vec![EvidenceRequest {
                    id: "E1".into(),
                    kind: Some("review".into()),
                    command: None,
                    note: None,
                }],
            }]),
            tasks: Some(vec![TaskDef {
                id: "T1".into(),
                title: Some("do".into()),
                requirements: vec!["R1".into()],
                constraints: vec![],
            }]),
            ..Default::default()
        }
    }

    #[test]
    fn clean_draft_has_no_findings() {
        assert!(lint_draft(&clean_draft(), &CommandPolicy::default()).is_empty());
    }

    #[test]
    fn flags_missing_evidence_and_bad_risk() {
        let mut d = clean_draft();
        d.risk = Some("medium".into());
        d.requirements.as_mut().unwrap()[0].evidence.clear();
        let findings = lint_draft(&d, &CommandPolicy::default());
        let codes: Vec<&str> = findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"invalid_risk_tier"));
        assert!(codes.contains(&"missing_evidence_request"));
    }

    #[test]
    fn flags_uncovered_requirement_and_unknown_ref() {
        let mut d = clean_draft();
        d.tasks.as_mut().unwrap()[0].requirements = vec!["R9".into()];
        let findings = lint_draft(&d, &CommandPolicy::default());
        let codes: Vec<&str> = findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"unknown_requirement"));
        assert!(codes.contains(&"requirement_not_covered"));
    }

    #[test]
    fn command_policy_flags_unlisted_when_configured() {
        let mut d = clean_draft();
        d.requirements.as_mut().unwrap()[0].evidence[0] = EvidenceRequest {
            id: "E1".into(),
            kind: Some("command".into()),
            command: Some("npm test && curl evil".into()),
            note: None,
        };
        let policy = CommandPolicy {
            allow: vec!["npm test*".into()],
        };
        // Whole-command glob: "npm test*" does match "npm test && curl evil".
        assert!(lint_draft(&d, &policy)
            .iter()
            .all(|f| f.code != "command_not_allowed"));
        let policy2 = CommandPolicy {
            allow: vec!["npm test".into()],
        };
        assert!(lint_draft(&d, &policy2)
            .iter()
            .any(|f| f.code == "command_not_allowed"));
    }

    #[test]
    fn glob_is_whole_string() {
        assert!(glob_match("npm test*", "npm test -- x"));
        assert!(!glob_match("npm test", "npm test && curl"));
        assert!(glob_match("*", "anything"));
    }
}
