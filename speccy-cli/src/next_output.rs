//! Text and JSON renderers for `speccy next`.
//!
//! The text renderer prints one line per kind variant; the JSON
//! renderer emits a tagged union by `kind` following the
//! `schema_version: 1` envelope conventions established by SPEC-0004.
//! See `.speccy/specs/0007-next-command/SPEC.md` REQ-004 / REQ-005.

use serde::Serialize;
use speccy_core::next::NextResult;

/// Tagged-union JSON envelope. `schema_version` is the first field so
/// downstream consumers can sniff it cheaply.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum JsonOutput {
    /// `kind: implement` variant.
    Implement(JsonImplement),
    /// `kind: review` variant.
    Review(JsonReview),
    /// `kind: report` variant.
    Report(JsonReport),
    /// `kind: blocked` variant.
    Blocked(JsonBlocked),
}

/// `kind: implement` payload.
#[derive(Debug, Clone, Serialize)]
pub struct JsonImplement {
    /// Stable schema version. Always 1 for now.
    pub schema_version: u32,
    /// `SPEC-NNNN` containing the task.
    pub spec: String,
    /// `T-NNN` identifier.
    pub task: String,
    /// Task title (no checkbox or bold ID prefix).
    pub task_line: String,
    /// Requirement IDs the task covers.
    pub covers: Vec<String>,
    /// Suggested file references.
    pub suggested_files: Vec<String>,
    /// Verbatim command harnesses should invoke next.
    pub prompt_command: String,
}

/// `kind: review` payload.
#[derive(Debug, Clone, Serialize)]
pub struct JsonReview {
    /// Stable schema version. Always 1 for now.
    pub schema_version: u32,
    /// `SPEC-NNNN` containing the task.
    pub spec: String,
    /// `T-NNN` identifier.
    pub task: String,
    /// Task title.
    pub task_line: String,
    /// Hardcoded persona fan-out list.
    pub personas: Vec<String>,
    /// Template harnesses iterate over `personas` to materialise.
    pub prompt_command_template: String,
}

/// `kind: report` payload.
#[derive(Debug, Clone, Serialize)]
pub struct JsonReport {
    /// Stable schema version.
    pub schema_version: u32,
    /// `SPEC-NNNN` needing a REPORT.md.
    pub spec: String,
    /// Verbatim command harnesses should invoke next.
    pub prompt_command: String,
}

/// `kind: blocked` payload.
#[derive(Debug, Clone, Serialize)]
pub struct JsonBlocked {
    /// Stable schema version.
    pub schema_version: u32,
    /// Canonical reason string.
    pub reason: String,
}

/// Render `result` as one line of human text, terminated with `\n`.
#[must_use = "the rendered line goes to stdout"]
pub fn render_text(result: &NextResult) -> String {
    match result {
        NextResult::Implement {
            spec,
            task,
            task_line,
            ..
        } => {
            format!("next: implement {task} ({spec}) -- {task_line}\n")
        }
        NextResult::Review {
            spec,
            task,
            personas,
            ..
        } => {
            format!(
                "next: review {task} ({spec}) -- personas: {personas}\n",
                personas = personas.join(", "),
            )
        }
        NextResult::Report { spec } => {
            format!("next: report {spec} -- all tasks complete\n")
        }
        NextResult::Blocked { reason } => format!("next: blocked -- {reason}\n"),
    }
}

/// Build the JSON payload for `result`.
#[must_use = "the JSON payload is the output of speccy next --json"]
pub fn render_json(result: &NextResult) -> JsonOutput {
    match result {
        NextResult::Implement {
            spec,
            task,
            task_line,
            covers,
            suggested_files,
        } => JsonOutput::Implement(JsonImplement {
            schema_version: 1,
            spec: spec.clone(),
            task: task.clone(),
            task_line: task_line.clone(),
            covers: covers.clone(),
            suggested_files: suggested_files.clone(),
            prompt_command: format!("speccy implement {spec}/{task}"),
        }),
        NextResult::Review {
            spec,
            task,
            task_line,
            personas,
        } => JsonOutput::Review(JsonReview {
            schema_version: 1,
            spec: spec.clone(),
            task: task.clone(),
            task_line: task_line.clone(),
            personas: personas.iter().map(|p| (*p).to_owned()).collect(),
            prompt_command_template: format!("speccy review {spec}/{task} --persona {{persona}}"),
        }),
        NextResult::Report { spec } => JsonOutput::Report(JsonReport {
            schema_version: 1,
            spec: spec.clone(),
            prompt_command: format!("speccy report {spec}"),
        }),
        NextResult::Blocked { reason } => JsonOutput::Blocked(JsonBlocked {
            schema_version: 1,
            reason: reason.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::render_text;
    use speccy_core::next::NextResult;

    #[test]
    fn text_implement_line_shape() {
        let r = NextResult::Implement {
            spec: "SPEC-0001".to_owned(),
            task: "T-001".to_owned(),
            task_line: "Implement signup".to_owned(),
            covers: vec!["REQ-001".to_owned()],
            suggested_files: vec![],
        };
        assert_eq!(
            render_text(&r),
            "next: implement T-001 (SPEC-0001) -- Implement signup\n",
        );
    }

    #[test]
    fn text_review_line_lists_personas_csv() {
        let r = NextResult::Review {
            spec: "SPEC-0002".to_owned(),
            task: "T-004".to_owned(),
            task_line: "Review me".to_owned(),
            personas: &["business", "tests", "security", "style"],
        };
        assert_eq!(
            render_text(&r),
            "next: review T-004 (SPEC-0002) -- personas: business, tests, security, style\n",
        );
    }

    #[test]
    fn text_report_line_shape() {
        let r = NextResult::Report {
            spec: "SPEC-0003".to_owned(),
        };
        assert_eq!(
            render_text(&r),
            "next: report SPEC-0003 -- all tasks complete\n"
        );
    }

    #[test]
    fn text_blocked_line_shape() {
        let r = NextResult::Blocked {
            reason: "no specs in workspace".to_owned(),
        };
        assert_eq!(render_text(&r), "next: blocked -- no specs in workspace\n");
    }
}
