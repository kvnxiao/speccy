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
    /// Present when `next_action` is `null`; `"completed"` or `"superseded"`.
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
    /// Kind string: `"decompose"`, `"review"`, `"implement"`, or `"ship"`.
    pub kind: &'static str,
    /// Task identifier; present for `review` and `implement`, absent for
    /// `decompose` and `ship`.
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

/// Build the JSON payload for the per-spec form.
#[must_use = "the JSON payload is the output of `speccy next SPEC-NNNN --json`"]
pub fn render_json_per_spec(
    spec_id: &str,
    action: Option<&NextAction>,
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
            reason: Some("completed".to_owned()),
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
        NextAction::Implement { task_id } => JsonNextAction {
            kind: "implement",
            task_id: Some(task_id.clone()),
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
    match action {
        None => format!("{spec_id}: completed\n"),
        Some(NextAction::Decompose) => format!("{spec_id}: decompose\n"),
        Some(NextAction::Review { task_id, .. }) => {
            format!("{spec_id}: review {task_id}\n")
        }
        Some(NextAction::Implement { task_id }) => {
            format!("{spec_id}: implement {task_id}\n")
        }
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
    fn text_per_spec_implement() {
        let action = NextAction::Implement {
            task_id: "T-003".to_owned(),
        };
        assert_eq!(
            render_text_per_spec("SPEC-0001", Some(&action)),
            "SPEC-0001: implement T-003\n",
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
                    action: NextAction::Implement {
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
        assert!(second.contains("implement"));
        assert!(second.contains("T-001"));
    }
}
