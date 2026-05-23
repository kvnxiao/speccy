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
}

/// JSON envelope for the workspace form (`speccy next --json`).
#[derive(Debug, Clone, Serialize)]
pub struct JsonWorkspace {
    /// Schema version. Pinned at `1` pre-v1.
    pub schema_version: u32,
    /// Active specs with their derived next actions.
    pub specs: Vec<JsonWorkspaceEntry>,
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
}

/// The `next_action` object inside JSON envelopes.
#[derive(Debug, Clone, Serialize)]
pub struct JsonNextAction {
    /// Kind string: `"decompose"`, `"review"`, `"work"`, `"vet"`, or
    /// `"ship"`.
    pub kind: &'static str,
    /// Task identifier; present for `review` and `work`, absent for
    /// `decompose`, `vet`, and `ship`.
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
/// `null` (SPEC-0043 REQ-003). Maps to the `reason` field in JSON.
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

/// Build the JSON payload for the per-spec form.
#[must_use = "the JSON payload is the output of `speccy next SPEC-NNNN --json`"]
pub fn render_json_per_spec(
    spec_id: &str,
    action: Option<&NextAction>,
    paths: SpecPaths,
) -> JsonPerSpec {
    render_json_per_spec_with_reason(spec_id, action, TerminalReason::Completed, paths)
}

/// Build the JSON payload for the per-spec form, allowing the caller to
/// name the terminal reason emitted when `action` is `None`.
#[must_use = "the JSON payload is the output of `speccy next SPEC-NNNN --json`"]
pub fn render_json_per_spec_with_reason(
    spec_id: &str,
    action: Option<&NextAction>,
    reason: TerminalReason,
    paths: SpecPaths,
) -> JsonPerSpec {
    match action {
        Some(a) => JsonPerSpec {
            schema_version: 1,
            spec_id: spec_id.to_owned(),
            next_action: Some(to_json_action(a)),
            reason: None,
            spec_md_path: paths.spec_md_path,
            tasks_md_path: paths.tasks_md_path,
            mission_md_path: paths.mission_md_path,
        },
        None => JsonPerSpec {
            schema_version: 1,
            spec_id: spec_id.to_owned(),
            next_action: None,
            reason: Some(reason.as_str().to_owned()),
            spec_md_path: paths.spec_md_path,
            tasks_md_path: paths.tasks_md_path,
            mission_md_path: paths.mission_md_path,
        },
    }
}

/// Build the JSON payload for the workspace form.
#[must_use = "the JSON payload is the output of `speccy next --json`"]
pub fn render_json_workspace(entries: &[(SpecNextEntry, SpecPaths)]) -> JsonWorkspace {
    JsonWorkspace {
        schema_version: 1,
        specs: entries
            .iter()
            .map(|(e, paths)| JsonWorkspaceEntry {
                spec_id: e.spec_id.clone(),
                next_action: to_json_action(&e.action),
                spec_md_path: paths.spec_md_path.clone(),
                tasks_md_path: paths.tasks_md_path.clone(),
                mission_md_path: paths.mission_md_path.clone(),
            })
            .collect(),
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

// ---------------------------------------------------------------------------
// Text renderers
// ---------------------------------------------------------------------------

/// Render the per-spec text output.
///
/// Format: `SPEC-NNNN: <kind> [T-NNN]\n` or `SPEC-NNNN: completed\n`.
#[must_use = "the rendered line goes to stdout"]
pub fn render_text_per_spec(spec_id: &str, action: Option<&NextAction>) -> String {
    render_text_per_spec_with_reason(spec_id, action, TerminalReason::Completed)
}

/// Render the per-spec text output with an explicit terminal reason.
///
/// Used by the dispatcher when the spec is in a dropped or superseded
/// terminal state (SPEC-0043 REQ-003) so the text line reflects the
/// actual reason rather than the default `completed`.
#[must_use = "the rendered line goes to stdout"]
pub fn render_text_per_spec_with_reason(
    spec_id: &str,
    action: Option<&NextAction>,
    reason: TerminalReason,
) -> String {
    match action {
        None => format!("{spec_id}: {reason}\n", reason = reason.as_str()),
        Some(NextAction::Decompose) => format!("{spec_id}: decompose\n"),
        Some(NextAction::Review { task_id, .. }) => {
            format!("{spec_id}: review {task_id}\n")
        }
        Some(NextAction::Work { task_id }) => {
            format!("{spec_id}: work {task_id}\n")
        }
        Some(NextAction::Vet) => format!("{spec_id}: vet\n"),
        Some(NextAction::Ship) => format!("{spec_id}: ship\n"),
    }
}

/// Render the workspace text output.
///
/// One line per active spec. Completed specs (where [`compute_workspace`]
/// returns no entry) are already filtered out by the caller.
/// The `paths` component is ignored for text rendering (paths are JSON-only).
#[must_use = "the rendered lines go to stdout"]
pub fn render_text_workspace(entries: &[(SpecNextEntry, SpecPaths)]) -> String {
    let mut out = String::new();
    for (e, _paths) in entries {
        let line = render_text_per_spec(&e.spec_id, Some(&e.action));
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
            render_text_per_spec("SPEC-0001", Some(&NextAction::Decompose)),
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
            render_text_per_spec("SPEC-0001", Some(&action)),
            "SPEC-0001: review T-002\n",
        );
    }

    #[test]
    fn text_per_spec_work() {
        let action = NextAction::Work {
            task_id: "T-003".to_owned(),
        };
        assert_eq!(
            render_text_per_spec("SPEC-0001", Some(&action)),
            "SPEC-0001: work T-003\n",
        );
    }

    #[test]
    fn text_per_spec_vet() {
        assert_eq!(
            render_text_per_spec("SPEC-0001", Some(&NextAction::Vet)),
            "SPEC-0001: vet\n",
        );
    }

    #[test]
    fn text_per_spec_ship() {
        assert_eq!(
            render_text_per_spec("SPEC-0001", Some(&NextAction::Ship)),
            "SPEC-0001: ship\n",
        );
    }

    #[test]
    fn text_per_spec_completed() {
        assert_eq!(
            render_text_per_spec("SPEC-0001", None),
            "SPEC-0001: completed\n",
        );
    }

    #[test]
    fn text_workspace_one_line_per_active_spec() {
        let stub_paths = super::SpecPaths {
            spec_md_path: ".speccy/specs/0001-x/SPEC.md".to_owned(),
            tasks_md_path: None,
            mission_md_path: None,
        };
        let entries = vec![
            (
                SpecNextEntry {
                    spec_id: "SPEC-0001".to_owned(),
                    action: NextAction::Decompose,
                },
                stub_paths.clone(),
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
}
