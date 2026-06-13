//! Text and JSON renderers for `speccy next`.
//!
//! The text renderers emit human-readable output:
//! - Workspace form: one line per active spec.
//! - Per-spec form: one line for the spec, or `SPEC-NNNN: completed`.
//!
//! The JSON renderers emit structured envelopes:
//! - Workspace form: `{"schema_version":1,"specs":[{…},…]}`.
//! - Per-spec form:
//!   `{"schema_version":1,"spec_id":"…","next_action":{…}|null}`.
//!
//! `schema_version` is pinned at `1` pre-v1. The envelope carries
//! `next_action`, `spec_md_path`, `tasks_md_path`, and
//! `mission_md_path` so skills do not need to glob `.speccy/specs/`.

use serde::Serialize;
use speccy_core::consistency::ConsistencyBlock;
use speccy_core::next::NextAction;
use speccy_core::next::SpecNextEntry;

/// JSON envelope for the per-spec form (`speccy next SPEC-NNNN --json`).
#[derive(Debug, Clone, Serialize)]
pub struct JsonPerSpec {
    /// Schema version. Pinned at `1` pre-v1.
    pub schema_version: u32,
    /// The spec identifier.
    pub spec_id: String,
    /// Derived next action, or `null` when the spec is completed.
    pub next_action: Option<JsonNextAction>,
    /// Present when `next_action` is `null`; `"completed"`, `"dropped"`, or
    /// `"superseded"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Repo-relative forward-slash path to `SPEC.md`.
    pub spec_md_path: String,
    /// Repo-relative forward-slash path to `TASKS.md`, or `null` when absent.
    pub tasks_md_path: Option<String>,
    /// Repo-relative forward-slash path to the mission folder's `MISSION.md`,
    /// or `null` for flat specs.
    pub mission_md_path: Option<String>,
    /// Consistency block. Always present; carries
    /// `status: "ok"` with an empty `drifts` array when no drift was
    /// detected.
    pub consistency: ConsistencyBlock,
}

/// JSON envelope for the workspace form (`speccy next --json`).
#[derive(Debug, Clone, Serialize)]
pub struct JsonWorkspace {
    /// Schema version. Pinned at `1` pre-v1.
    pub schema_version: u32,
    /// Active specs with their derived next actions.
    pub specs: Vec<JsonWorkspaceEntry>,
    /// Present when `specs` is empty and the workspace itself is in a
    /// terminal state for `speccy next`; absent otherwise. The only
    /// slug emitted today is `"no_active_specs"`. Consumers that read
    /// `reason` should treat it as the loop-stop signal in parallel
    /// to the per-spec form's `reason` field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A single per-spec entry inside the workspace JSON envelope.
#[derive(Debug, Clone, Serialize)]
pub struct JsonWorkspaceEntry {
    /// The spec identifier.
    pub spec_id: String,
    /// Derived next action.
    pub next_action: JsonNextAction,
    /// Repo-relative forward-slash path to `SPEC.md`.
    pub spec_md_path: String,
    /// Repo-relative forward-slash path to `TASKS.md`, or `null` when absent.
    pub tasks_md_path: Option<String>,
    /// Repo-relative forward-slash path to the mission folder's `MISSION.md`,
    /// or `null` for flat specs.
    pub mission_md_path: Option<String>,
    /// Consistency block.
    pub consistency: ConsistencyBlock,
}

/// The `next_action` object inside JSON envelopes.
#[derive(Debug, Clone, Serialize)]
pub struct JsonNextAction {
    /// Kind string: `"decompose"`, `"review"`, `"work"`, `"vet"`,
    /// `"ship"`, or `"reconcile"` (override when
    /// consistency drift is detected).
    pub kind: &'static str,
    /// Task identifier; present for `review` and `work`. Absent for
    /// `decompose`, `vet`, and `ship`. For the
    /// `reconcile` override, the `task_id` from the underlying
    /// dispatch (when there was one) is preserved through the
    /// override so downstream skills can still see which task the
    /// unreconciled state pertains to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

// ---------------------------------------------------------------------------
// JSON renderers
// ---------------------------------------------------------------------------

/// Resolved per-spec filesystem paths for JSON output.
///
/// All fields are repo-relative forward-slash strings (or `None`).
#[derive(Debug, Clone)]
pub struct SpecPaths {
    /// Repo-relative path to `SPEC.md`.
    pub spec_md_path: String,
    /// Repo-relative path to `TASKS.md`, or `None` when absent.
    pub tasks_md_path: Option<String>,
    /// Repo-relative path to the mission folder's `MISSION.md`, or `None`
    /// for flat specs.
    pub mission_md_path: Option<String>,
}

/// Terminal-state reason for the per-spec form when `next_action` is
/// `null`. Maps to the `reason` field in JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalReason {
    /// REPORT.md present — the SPEC has shipped.
    Completed,
    /// SPEC frontmatter `status: dropped`.
    Dropped,
    /// SPEC frontmatter `status: superseded`.
    Superseded,
}

impl TerminalReason {
    /// Stable string used in JSON `reason` and stderr messages.
    #[must_use = "the slug is part of the public JSON contract"]
    pub fn as_str(self) -> &'static str {
        match self {
            TerminalReason::Completed => "completed",
            TerminalReason::Dropped => "dropped",
            TerminalReason::Superseded => "superseded",
        }
    }
}

/// Stable slug used in the workspace-form JSON `reason` field and the
/// stderr advisory when `speccy next` resolves to a terminal state at
/// the workspace level. Mirrors [`TerminalReason::as_str`] in spirit
/// but covers the workspace-level loop-stop signal (no active specs
/// at all) rather than a per-spec terminal status.
pub const WORKSPACE_TERMINAL_REASON: &str = "no_active_specs";

/// Build the JSON payload for the per-spec form.
#[must_use = "the JSON payload is the output of `speccy next SPEC-NNNN --json`"]
pub fn render_json_per_spec(
    spec_id: &str,
    action: Option<&NextAction>,
    paths: SpecPaths,
    consistency: ConsistencyBlock,
) -> JsonPerSpec {
    render_json_per_spec_with_reason(
        spec_id,
        action,
        TerminalReason::Completed,
        paths,
        consistency,
    )
}

/// Build the JSON payload for the per-spec form, allowing the caller to
/// name the terminal reason emitted when `action` is `None`.
#[must_use = "the JSON payload is the output of `speccy next SPEC-NNNN --json`"]
pub fn render_json_per_spec_with_reason(
    spec_id: &str,
    action: Option<&NextAction>,
    reason: TerminalReason,
    paths: SpecPaths,
    consistency: ConsistencyBlock,
) -> JsonPerSpec {
    match action {
        Some(a) => JsonPerSpec {
            schema_version: 1,
            spec_id: spec_id.to_owned(),
            next_action: Some(apply_reconcile_override(to_json_action(a), &consistency)),
            reason: None,
            spec_md_path: paths.spec_md_path,
            tasks_md_path: paths.tasks_md_path,
            mission_md_path: paths.mission_md_path,
            consistency,
        },
        None => JsonPerSpec {
            schema_version: 1,
            spec_id: spec_id.to_owned(),
            next_action: None,
            reason: Some(reason.as_str().to_owned()),
            spec_md_path: paths.spec_md_path,
            tasks_md_path: paths.tasks_md_path,
            mission_md_path: paths.mission_md_path,
            consistency,
        },
    }
}

/// Build the JSON payload for the workspace form.
///
/// When `entries` is empty the envelope carries
/// `reason: "no_active_specs"` (the workspace-level loop-stop signal);
/// otherwise `reason` is omitted.
#[must_use = "the JSON payload is the output of `speccy next --json`"]
pub fn render_json_workspace(
    entries: &[(SpecNextEntry, SpecPaths, ConsistencyBlock)],
) -> JsonWorkspace {
    let specs: Vec<JsonWorkspaceEntry> = entries
        .iter()
        .map(|(e, paths, consistency)| JsonWorkspaceEntry {
            spec_id: e.spec_id.clone(),
            next_action: apply_reconcile_override(to_json_action(&e.action), consistency),
            spec_md_path: paths.spec_md_path.clone(),
            tasks_md_path: paths.tasks_md_path.clone(),
            mission_md_path: paths.mission_md_path.clone(),
            consistency: consistency.clone(),
        })
        .collect();
    let reason = specs
        .is_empty()
        .then(|| WORKSPACE_TERMINAL_REASON.to_owned());
    JsonWorkspace {
        schema_version: 1,
        specs,
        reason,
    }
}

fn to_json_action(action: &NextAction) -> JsonNextAction {
    match action {
        NextAction::Decompose => JsonNextAction {
            kind: "decompose",
            task_id: None,
        },
        NextAction::Review { task_id, .. } => JsonNextAction {
            kind: "review",
            task_id: Some(task_id.clone()),
        },
        NextAction::Work { task_id } => JsonNextAction {
            kind: "work",
            task_id: Some(task_id.clone()),
        },
        NextAction::Vet => JsonNextAction {
            kind: "vet",
            task_id: None,
        },
        NextAction::Ship => JsonNextAction {
            kind: "ship",
            task_id: None,
        },
    }
}

/// Apply the reconcile override: when `consistency.status` is
/// anything other than `Ok`, force `next_action.kind` to `"reconcile"`.
/// The `task_id` is preserved when present so downstream skills can
/// still see which task the unreconciled state pertains to.
fn apply_reconcile_override(
    action: JsonNextAction,
    consistency: &ConsistencyBlock,
) -> JsonNextAction {
    if matches!(
        consistency.status,
        speccy_core::consistency::ConsistencyStatus::Ok
    ) {
        action
    } else {
        JsonNextAction {
            kind: "reconcile",
            task_id: action.task_id,
        }
    }
}

// ---------------------------------------------------------------------------
// Text renderers
// ---------------------------------------------------------------------------

/// Render the per-spec text output.
///
/// Format: `SPEC-NNNN: <kind> [T-NNN]\n` or `SPEC-NNNN: completed\n`.
/// The reconcile override applies exactly as in the
/// JSON form: a non-`ok` consistency status renders `reconcile` (with
/// the underlying dispatch's task id preserved when there is one).
#[must_use = "the rendered line goes to stdout"]
pub fn render_text_per_spec(
    spec_id: &str,
    action: Option<&NextAction>,
    consistency: &ConsistencyBlock,
) -> String {
    render_text_per_spec_with_reason(spec_id, action, TerminalReason::Completed, consistency)
}

/// Render the per-spec text output with an explicit terminal reason.
///
/// Used by the dispatcher when the spec is in a dropped or superseded
/// terminal state so the text line reflects the
/// actual reason rather than the default `completed`.
#[must_use = "the rendered line goes to stdout"]
pub fn render_text_per_spec_with_reason(
    spec_id: &str,
    action: Option<&NextAction>,
    reason: TerminalReason,
    consistency: &ConsistencyBlock,
) -> String {
    let Some(action) = action else {
        return format!("{spec_id}: {reason}\n", reason = reason.as_str());
    };
    let json_action = apply_reconcile_override(to_json_action(action), consistency);
    match json_action.task_id {
        Some(task_id) => format!("{spec_id}: {kind} {task_id}\n", kind = json_action.kind),
        None => format!("{spec_id}: {kind}\n", kind = json_action.kind),
    }
}

/// Render the workspace text output.
///
/// One line per active spec. Completed specs (where [`compute_workspace`]
/// returns no entry) are already filtered out by the caller.
/// The `paths` component is ignored for text rendering (paths are JSON-only).
#[must_use = "the rendered lines go to stdout"]
pub fn render_text_workspace(entries: &[(SpecNextEntry, SpecPaths, ConsistencyBlock)]) -> String {
    let mut out = String::new();
    for (e, _paths, consistency) in entries {
        let line = render_text_per_spec(&e.spec_id, Some(&e.action), consistency);
        out.push_str(&line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::render_text_per_spec;
    use super::render_text_workspace;
    use speccy_core::next::NextAction;
    use speccy_core::next::SpecNextEntry;
    use speccy_core::next::default_personas;

    #[test]
    fn text_per_spec_decompose() {
        assert_eq!(
            render_text_per_spec(
                "SPEC-0001",
                Some(&NextAction::Decompose),
                &ConsistencyBlock::ok()
            ),
            "SPEC-0001: decompose\n",
        );
    }

    #[test]
    fn text_per_spec_review() {
        let action = NextAction::Review {
            task_id: "T-002".to_owned(),
            personas: default_personas(),
        };
        assert_eq!(
            render_text_per_spec("SPEC-0001", Some(&action), &ConsistencyBlock::ok()),
            "SPEC-0001: review T-002\n",
        );
    }

    #[test]
    fn text_per_spec_work() {
        let action = NextAction::Work {
            task_id: "T-003".to_owned(),
        };
        assert_eq!(
            render_text_per_spec("SPEC-0001", Some(&action), &ConsistencyBlock::ok()),
            "SPEC-0001: work T-003\n",
        );
    }

    #[test]
    fn text_per_spec_vet() {
        assert_eq!(
            render_text_per_spec("SPEC-0001", Some(&NextAction::Vet), &ConsistencyBlock::ok()),
            "SPEC-0001: vet\n",
        );
    }

    #[test]
    fn text_per_spec_ship() {
        assert_eq!(
            render_text_per_spec(
                "SPEC-0001",
                Some(&NextAction::Ship),
                &ConsistencyBlock::ok()
            ),
            "SPEC-0001: ship\n",
        );
    }

    #[test]
    fn text_per_spec_completed() {
        assert_eq!(
            render_text_per_spec("SPEC-0001", None, &ConsistencyBlock::ok()),
            "SPEC-0001: completed\n",
        );
    }

    #[test]
    fn text_per_spec_reconcile_override_matches_json_form() {
        let action = NextAction::Work {
            task_id: "T-001".to_owned(),
        };
        assert_eq!(
            render_text_per_spec("SPEC-0099", Some(&action), &blocked_block()),
            "SPEC-0099: reconcile T-001\n",
        );
    }

    #[test]
    fn text_workspace_one_line_per_active_spec() {
        let stub_paths = super::SpecPaths {
            spec_md_path: ".speccy/specs/0001-x/SPEC.md".to_owned(),
            tasks_md_path: None,
            mission_md_path: None,
        };
        let ok = speccy_core::consistency::ConsistencyBlock::ok();
        let entries = vec![
            (
                SpecNextEntry {
                    spec_id: "SPEC-0001".to_owned(),
                    action: NextAction::Decompose,
                },
                stub_paths.clone(),
                ok.clone(),
            ),
            (
                SpecNextEntry {
                    spec_id: "SPEC-0002".to_owned(),
                    action: NextAction::Work {
                        task_id: "T-001".to_owned(),
                    },
                },
                super::SpecPaths {
                    spec_md_path: ".speccy/specs/0002-y/SPEC.md".to_owned(),
                    tasks_md_path: None,
                    mission_md_path: None,
                },
                ok.clone(),
            ),
        ];
        let text = render_text_workspace(&entries);
        let output_lines: Vec<&str> = text.lines().collect();
        assert_eq!(output_lines.len(), 2);
        let first = output_lines.first().copied().unwrap_or_default();
        let second = output_lines.get(1).copied().unwrap_or_default();
        assert!(first.contains("SPEC-0001"));
        assert!(first.contains("decompose"));
        assert!(second.contains("SPEC-0002"));
        assert!(second.contains("work"));
        assert!(second.contains("T-001"));
    }

    // ---- reconcile-override behaviour ----

    use speccy_core::consistency::ConsistencyBlock;
    use speccy_core::consistency::ConsistencyStatus;
    use speccy_core::consistency::DriftDetails;
    use speccy_core::consistency::DriftEntry;
    use speccy_core::consistency::DriftKind;
    use speccy_core::consistency::DriftSeverity;

    fn stub_paths() -> super::SpecPaths {
        super::SpecPaths {
            spec_md_path: ".speccy/specs/0099-x/SPEC.md".to_owned(),
            tasks_md_path: None,
            mission_md_path: None,
        }
    }

    fn blocked_block() -> ConsistencyBlock {
        ConsistencyBlock {
            status: ConsistencyStatus::Blocked,
            drifts: vec![DriftEntry {
                task_id: "T-001".to_owned(),
                kind: DriftKind::StateCompletedNoCommit,
                severity: DriftSeverity::Blocking,
                tasks_state: "completed".to_owned(),
                details: DriftDetails::StateCompletedNoCommit {
                    expected_trailer: "[SPEC-0099/T-001]:".to_owned(),
                    working_tree_dirty: false,
                },
            }],
        }
    }

    fn drift_block() -> ConsistencyBlock {
        ConsistencyBlock {
            status: ConsistencyStatus::Drift,
            drifts: vec![DriftEntry {
                task_id: "T-001".to_owned(),
                kind: DriftKind::CommitWithoutState,
                severity: DriftSeverity::AutoFixable,
                tasks_state: "in-review".to_owned(),
                details: DriftDetails::CommitWithoutState {
                    commit_sha: "a".repeat(40),
                    commit_short_sha: "a".repeat(8),
                },
            }],
        }
    }

    #[test]
    fn reconcile_override_fires_on_blocked_and_preserves_task_id() {
        let action = NextAction::Work {
            task_id: "T-001".to_owned(),
        };
        let json =
            super::render_json_per_spec("SPEC-0099", Some(&action), stub_paths(), blocked_block());
        let next = json.next_action.expect("non-terminal");
        assert_eq!(next.kind, "reconcile");
        assert_eq!(next.task_id.as_deref(), Some("T-001"));
    }

    #[test]
    fn reconcile_override_fires_on_drift_status() {
        let action = NextAction::Review {
            task_id: "T-001".to_owned(),
            personas: default_personas(),
        };
        let json =
            super::render_json_per_spec("SPEC-0099", Some(&action), stub_paths(), drift_block());
        let next = json.next_action.expect("non-terminal");
        assert_eq!(next.kind, "reconcile");
        assert_eq!(next.task_id.as_deref(), Some("T-001"));
    }

    #[test]
    fn reconcile_override_is_no_op_on_ok_status() {
        let action = NextAction::Work {
            task_id: "T-002".to_owned(),
        };
        let json = super::render_json_per_spec(
            "SPEC-0099",
            Some(&action),
            stub_paths(),
            ConsistencyBlock::ok(),
        );
        let next = json.next_action.expect("non-terminal");
        assert_eq!(next.kind, "work");
        assert_eq!(next.task_id.as_deref(), Some("T-002"));
    }

    #[test]
    fn reconcile_override_preserves_decompose_kind_on_ok_status() {
        let json = super::render_json_per_spec(
            "SPEC-0099",
            Some(&NextAction::Decompose),
            stub_paths(),
            ConsistencyBlock::ok(),
        );
        let next = json.next_action.expect("non-terminal");
        assert_eq!(next.kind, "decompose");
        assert!(next.task_id.is_none());
    }
}
