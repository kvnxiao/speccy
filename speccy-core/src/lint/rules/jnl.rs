//! JNL-* rules: per-task journal file validation gated by task state.
//!
//! The three rules are:
//!
//! - `JNL-001` (error): fires on any task at `state="pending"` whose
//!   `journal/T-NNN.md` exists.
//! - `JNL-002` (error): fires on any task at `state="completed"` whose
//!   `journal/T-NNN.md` is missing.
//! - `JNL-003` (error): fires on any task at `state="completed"` whose journal
//!   file has a shape or binding violation (filename ↔ frontmatter mismatch,
//!   parent spec mismatch, parser errors, round-sequence violations).
//!
//! Tasks at `state="in-progress"` or `state="in-review"` skip all
//! `JNL-*` lints — the lint never runs mid-loop.

use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;
use crate::parse::TaskState;
use crate::parse::journal_xml::parse as parse_journal;
use camino::Utf8Path;

const JNL_001: &str = "JNL-001";
const JNL_002: &str = "JNL-002";
const JNL_003: &str = "JNL-003";

/// Append every JNL-* diagnostic for one spec.
pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    let Some(tasks_md) = spec.tasks_md_ok() else {
        return;
    };
    let journal_dir = spec.dir.join("journal");

    for task in &tasks_md.tasks {
        let journal_path = journal_dir.join(format!("{}.md", task.id));
        match task.state {
            TaskState::Pending => check_pending(spec, task, &journal_path, out),
            TaskState::Completed => check_completed(spec, task, &journal_path, out),
            TaskState::InProgress | TaskState::InReview => {
                // In-progress/in-review tasks skip
                // all JNL-* lints entirely.
            }
        }
    }
}

fn check_pending(
    spec: &ParsedSpec,
    task: &crate::parse::Task,
    journal_path: &Utf8Path,
    out: &mut Vec<Diagnostic>,
) {
    if journal_path.exists() {
        out.push(Diagnostic::with_file(
            JNL_001,
            Level::Error,
            spec.spec_id.clone(),
            journal_path.to_path_buf(),
            format!(
                "task `{}` is `state=\"pending\"` but `{journal_path}` exists; pending tasks must have a clean slate (delete the journal file or transition the task)",
                task.id
            ),
        ));
    }
}

fn check_completed(
    spec: &ParsedSpec,
    task: &crate::parse::Task,
    journal_path: &Utf8Path,
    out: &mut Vec<Diagnostic>,
) {
    if !journal_path.exists() {
        out.push(Diagnostic::with_file(
            JNL_002,
            Level::Error,
            spec.spec_id.clone(),
            journal_path.to_path_buf(),
            format!(
                "task `{}` is `state=\"completed\"` but `{journal_path}` is missing; completed tasks must have a well-formed journal file",
                task.id
            ),
        ));
        return;
    }
    let raw = match fs_err::read_to_string(journal_path.as_std_path()) {
        Ok(s) => s,
        Err(e) => {
            out.push(Diagnostic::with_file(
                JNL_003,
                Level::Error,
                spec.spec_id.clone(),
                journal_path.to_path_buf(),
                format!("could not read journal file: {e}"),
            ));
            return;
        }
    };
    let doc = match parse_journal(&raw, journal_path) {
        Ok(d) => d,
        Err(e) => {
            out.push(Diagnostic::with_file(
                JNL_003,
                Level::Error,
                spec.spec_id.clone(),
                journal_path.to_path_buf(),
                format!("journal parse error: {e}"),
            ));
            return;
        }
    };
    if doc.task != task.id {
        out.push(Diagnostic::with_file(
            JNL_003,
            Level::Error,
            spec.spec_id.clone(),
            journal_path.to_path_buf(),
            format!(
                "journal frontmatter `task: {}` does not match filename binding `{}` (filename ↔ frontmatter `task:` mismatch)",
                doc.task, task.id
            ),
        ));
    }
    if let Some(expected_spec) = spec.spec_id.as_deref()
        && doc.spec != expected_spec
    {
        out.push(Diagnostic::with_file(
            JNL_003,
            Level::Error,
            spec.spec_id.clone(),
            journal_path.to_path_buf(),
            format!(
                "journal frontmatter `spec: {}` does not match parent spec directory binding `{expected_spec}` (frontmatter `spec:` ↔ parent dir mismatch)",
                doc.spec
            ),
        ));
    }
}
