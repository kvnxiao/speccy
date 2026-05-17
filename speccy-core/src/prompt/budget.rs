//! Context-budget trimmer.
//!
//! Drops low-priority sections in the order specified by
//! `.speccy/ARCHITECTURE.md` "Prompt context budget" (and SPEC-0005 REQ-006):
//!
//! 1. `## Notes` section content.
//! 2. Answered `## Open questions` entries (`- [x]` items).
//! 3. `## Changelog` rows older than the 5 most recent.
//! 4. Task review notes older than the 3 most recent per task.
//! 5. Other specs' summaries.
//!
//! Each step that fires records a label in [`TrimResult::dropped`]. If
//! the result still exceeds the budget after every applicable step, the
//! function emits a warning to its sink and returns `fits = false`.

use std::collections::HashSet;
use std::io::Write;

/// Default budget threshold for [`trim_to_budget`] in characters,
/// hardcoded at 80,000 per SPEC-0005 DEC-004. Safe across all currently
/// shipping host context windows.
pub const DEFAULT_BUDGET: usize = 80_000;

/// Signature shared by every drop step.
type DropStep = fn(&str) -> Option<String>;

/// Outcome of a single trim invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrimResult {
    /// Possibly-trimmed prompt text.
    pub output: String,
    /// Labels naming each drop step that fired, in the order they
    /// fired. Empty when nothing was dropped.
    pub dropped: Vec<String>,
    /// Whether the final `output` is within the budget.
    pub fits: bool,
}

/// Trim `rendered` to fit `budget` characters using the ARCHITECTURE.md drop
/// order. Warnings about overrun go to process stderr; use
/// [`trim_to_budget_with_warn`] to inject a custom sink for tests.
#[must_use = "the trim result carries the prompt text caller must emit"]
pub fn trim_to_budget(rendered: String, budget: usize) -> TrimResult {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    trim_to_budget_with_warn(rendered, budget, &mut lock)
}

/// Form of [`trim_to_budget`] that accepts an injected warning sink.
/// Used by tests; production code reaches for [`trim_to_budget`].
#[must_use = "the trim result carries the prompt text caller must emit"]
pub fn trim_to_budget_with_warn<W: Write>(
    rendered: String,
    budget: usize,
    warn_out: &mut W,
) -> TrimResult {
    if rendered.len() <= budget {
        return TrimResult {
            output: rendered,
            dropped: Vec::new(),
            fits: true,
        };
    }

    let mut current = rendered;
    let mut dropped: Vec<String> = Vec::new();

    let steps: [(&str, DropStep); 5] = [
        ("## Notes", |c| drop_section_by_heading(c, "Notes")),
        ("answered open questions", drop_answered_open_questions),
        (
            "Changelog rows older than 5 most recent",
            trim_changelog_to_recent_5,
        ),
        (
            "task review notes older than 3 most recent per task",
            trim_task_review_notes,
        ),
        ("other specs' summaries", |c| {
            drop_section_by_heading(c, "Other specs")
        }),
    ];

    for (label, step) in steps {
        if current.len() <= budget {
            break;
        }
        if let Some(next) = step(&current) {
            current = next;
            dropped.push(label.to_owned());
        }
    }

    let fits = current.len() <= budget;
    if !fits
        && writeln!(
            warn_out,
            "speccy prompt: rendered output ({chars} chars) exceeds budget ({budget} chars) after all drops",
            chars = current.len(),
        )
        .is_err()
    {
        // Warning sink is closed; nothing actionable.
    }

    TrimResult {
        output: current,
        dropped,
        fits,
    }
}

fn drop_section_by_heading(content: &str, heading_name: &str) -> Option<String> {
    let target = format!("## {heading_name}");
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_end().eq_ignore_ascii_case(&target))?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start.saturating_add(1))
        .find(|(_, l)| l.starts_with("## "))
        .map_or(lines.len(), |(i, _)| i);

    let kept: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < start || *i >= end)
        .map(|(_, l)| *l)
        .collect();

    Some(rejoin_lines(&kept, content))
}

fn drop_answered_open_questions(content: &str) -> Option<String> {
    let target = "## open questions";
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_end().eq_ignore_ascii_case(target))?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start.saturating_add(1))
        .find(|(_, l)| l.starts_with("## "))
        .map_or(lines.len(), |(i, _)| i);

    let is_answered = |line: &str| {
        let t = line.trim_start();
        t.starts_with("- [x]") || t.starts_with("- [X]")
    };
    let in_section = |i: usize| i > start && i < end;

    let any_answered = lines
        .iter()
        .enumerate()
        .any(|(i, l)| in_section(i) && is_answered(l));
    if !any_answered {
        return None;
    }

    let kept: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, l)| !(in_section(*i) && is_answered(l)))
        .map(|(_, l)| *l)
        .collect();

    Some(rejoin_lines(&kept, content))
}

fn trim_changelog_to_recent_5(content: &str) -> Option<String> {
    const KEEP: usize = 5;
    let target = "## Changelog";
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_end().eq_ignore_ascii_case(target))?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start.saturating_add(1))
        .find(|(_, l)| l.starts_with("## "))
        .map_or(lines.len(), |(i, _)| i);

    let table_line_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| *i > start && *i < end)
        .filter(|(_, l)| l.trim_start().starts_with('|'))
        .map(|(i, _)| i)
        .collect();

    let total = table_line_indices.len();
    if total <= KEEP.saturating_add(2) {
        return None;
    }

    let data_count = total.saturating_sub(2);
    let drop_count = data_count.saturating_sub(KEEP);
    let drop_indices: HashSet<usize> = table_line_indices
        .iter()
        .enumerate()
        .filter(|(pos, _)| *pos >= 2 && *pos < 2_usize.saturating_add(drop_count))
        .map(|(_, line_idx)| *line_idx)
        .collect();

    let kept: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !drop_indices.contains(i))
        .map(|(_, l)| *l)
        .collect();

    Some(rejoin_lines(&kept, content))
}

fn trim_task_review_notes(content: &str) -> Option<String> {
    const KEEP: usize = 3;
    let lines: Vec<&str> = content.lines().collect();

    let task_starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| is_task_line(l))
        .map(|(i, _)| i)
        .collect();

    if task_starts.is_empty() {
        return None;
    }

    let mut drop_indices: HashSet<usize> = HashSet::new();
    for (pos, start) in task_starts.iter().enumerate() {
        let next_task_start = task_starts
            .get(pos.saturating_add(1))
            .copied()
            .unwrap_or(lines.len());
        let review_indices: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| *i > *start && *i < next_task_start)
            .filter(|(_, l)| is_review_note_line(l))
            .map(|(i, _)| i)
            .collect();
        if review_indices.len() > KEEP {
            let drop_count = review_indices.len().saturating_sub(KEEP);
            for idx in review_indices.iter().take(drop_count) {
                drop_indices.insert(*idx);
            }
        }
    }

    if drop_indices.is_empty() {
        return None;
    }

    let kept: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !drop_indices.contains(i))
        .map(|(_, l)| *l)
        .collect();

    Some(rejoin_lines(&kept, content))
}

fn is_task_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed.strip_prefix("- [") else {
        return false;
    };
    let mut chars = rest.chars();
    let marker = chars.next();
    let close = chars.next();
    let space = chars.next();
    if !matches!(marker, Some(' ' | '~' | '?' | 'x')) {
        return false;
    }
    if close != Some(']') || space != Some(' ') {
        return false;
    }
    let after: String = chars.collect();
    after.contains("**T-")
}

fn is_review_note_line(line: &str) -> bool {
    line.trim_start().starts_with("- Review (")
}

fn rejoin_lines(lines: &[&str], original: &str) -> String {
    let mut out = lines.join("\n");
    if original.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::TrimResult;
    use super::trim_to_budget_with_warn;

    fn trim(content: &str, budget: usize) -> (TrimResult, String) {
        let mut warns = Vec::new();
        let result = trim_to_budget_with_warn(content.to_owned(), budget, &mut warns);
        (
            result,
            String::from_utf8(warns).expect("warning bytes UTF-8"),
        )
    }

    #[test]
    fn small_content_passes_through_unchanged() {
        let content = "tiny prompt";
        let (result, warns) = trim(content, 80_000);
        assert_eq!(result.output, content);
        assert!(result.dropped.is_empty());
        assert!(result.fits);
        assert!(warns.is_empty());
    }

    #[test]
    fn drops_notes_section_first() {
        let notes_body = "x".repeat(5_000);
        let content =
            format!("# Title\n\n## Goals\nbody\n\n## Notes\n{notes_body}\n\n## After\nmore\n");
        let budget = content.len().saturating_sub(100);
        let (result, _) = trim(&content, budget);
        assert!(result.fits, "dropping Notes should fit budget");
        assert!(
            !result.output.contains("## Notes"),
            "## Notes heading should be removed",
        );
        assert!(
            result.output.contains("## After"),
            "## After heading should survive",
        );
        assert_eq!(result.dropped, vec!["## Notes".to_owned()]);
    }

    #[test]
    fn drops_answered_open_questions() {
        let mut body = String::from("# Title\n\n## Open questions\n\n");
        for _ in 0..200 {
            body.push_str("- [x] answered with extra padding to inflate size\n");
        }
        body.push_str("- [ ] still open\n\n## End\nend body\n");
        let budget = body.len().saturating_sub(500);
        let (result, _) = trim(&body, budget);
        assert!(result.fits, "dropping answered questions should fit");
        assert!(
            !result.output.contains("- [x]"),
            "answered items should be removed",
        );
        assert!(
            result.output.contains("- [ ] still open"),
            "unanswered items must survive",
        );
        assert_eq!(result.dropped, vec!["answered open questions".to_owned()]);
    }

    #[test]
    fn trims_old_changelog_rows() {
        let mut body = String::from(
            "# Title\n\n## Changelog\n\n| Date | Author | Summary |\n|------|--------|---------|\n",
        );
        for i in 0..40 {
            use std::fmt::Write as _;
            writeln!(
                body,
                "| 2026-05-{i:02} | agent/x | row {i} with substantial padding text to inflate length |",
            )
            .expect("write to String should succeed");
        }
        body.push_str("\n## End\nend\n");
        let budget = body.len().saturating_sub(500);
        let (result, _) = trim(&body, budget);
        assert!(result.fits, "Changelog trim should bring under budget");
        assert!(
            result.output.contains("row 39"),
            "most-recent row should survive: output tail = {tail:?}",
            tail = result.output.lines().rev().take(15).collect::<Vec<_>>(),
        );
        assert!(
            !result.output.contains("row 0 "),
            "oldest row should be dropped",
        );
        assert_eq!(
            result.dropped,
            vec!["Changelog rows older than 5 most recent".to_owned()],
        );
    }

    #[test]
    fn trims_old_task_review_notes() {
        let mut body =
            String::from("# Tasks\n\n- [?] **T-001**: example task\n  - Covers: REQ-001\n");
        for i in 0..15 {
            use std::fmt::Write as _;
            writeln!(
                body,
                "  - Review (security, blocking): note {i} with padding to inflate size",
            )
            .expect("write to String should succeed");
        }
        let budget = body.len().saturating_sub(300);
        let (result, _) = trim(&body, budget);
        assert!(result.fits, "task review trim should bring under budget");
        assert!(
            result.output.contains("note 14"),
            "most-recent review note should survive",
        );
        assert!(
            !result.output.contains("note 0 "),
            "oldest review note should be dropped",
        );
        assert_eq!(
            result.dropped,
            vec!["task review notes older than 3 most recent per task".to_owned()],
        );
    }

    #[test]
    fn drops_other_specs_summaries() {
        let big = "x".repeat(5_000);
        let body = format!("# Title\n\n## Goals\nA\n\n## Other specs\n{big}\n\n## End\nend\n");
        let budget = body.len().saturating_sub(100);
        let (result, _) = trim(&body, budget);
        assert!(result.fits);
        assert!(
            !result.output.contains("## Other specs"),
            "Other specs section should be dropped",
        );
        assert_eq!(result.dropped, vec!["other specs' summaries".to_owned()]);
    }

    #[test]
    fn warns_and_returns_fits_false_when_no_step_helps() {
        let body = "x".repeat(2_000);
        let (result, warns) = trim(&body, 100);
        assert!(!result.fits);
        assert_eq!(result.output, body);
        assert!(result.dropped.is_empty());
        assert!(
            warns.contains("exceeds budget"),
            "expected overrun warning on stderr, got: {warns}",
        );
    }

    #[test]
    fn dropped_vec_preserves_step_order() {
        let notes = "n".repeat(2_000);
        let mut body = format!("# T\n\n## Goals\nA\n\n## Notes\n{notes}\n\n## Open questions\n\n");
        for _ in 0..200 {
            body.push_str("- [x] answered with padding to inflate size further\n");
        }
        body.push_str("\n## End\n");
        let budget = body.len().saturating_sub(body.len().saturating_sub(800));
        let (result, _) = trim(&body, budget);
        assert!(
            result.dropped.len() >= 2,
            "expected at least two drops, got {:?}",
            result.dropped,
        );
        let notes_pos = result
            .dropped
            .iter()
            .position(|s| s == "## Notes")
            .expect("Notes drop must be present");
        let answered_pos = result
            .dropped
            .iter()
            .position(|s| s == "answered open questions")
            .expect("answered drop must be present");
        assert!(
            notes_pos < answered_pos,
            "Notes must drop before answered questions per ARCHITECTURE.md order",
        );
    }
}
