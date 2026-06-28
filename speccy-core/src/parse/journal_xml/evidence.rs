//! Pure detection helpers for the demonstrated-evidence append-time gate.
//!
//! The gate refuses an `<implementer>` block whose Evidence roll call labels a
//! CHK `demonstrated` while no backing evidence scenario exists. Detection is a
//! line-scoped heuristic, not a parsed grammar: a `CHK-NNN` id counts as
//! demonstrated only when its own line also carries the token `demonstrated`.
//! This recognizes both documented roll-call forms — the bullet form
//! (`- CHK-001 (...): demonstrated`) and the prose form
//! (`CHK-001 demonstrated by some_passing_test`) — without committing to a
//! brittle grammar over free-form prose. The same-line scoping stops an
//! incidental `demonstrated` token on a CHK-less line from over-triggering.

use regex::Regex;
use std::sync::OnceLock;

fn chk_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    #[expect(
        clippy::unwrap_used,
        reason = "compile-time literal regex; covered by unit tests"
    )]
    CELL.get_or_init(|| Regex::new(r"CHK-[0-9]+").unwrap())
}

/// The CHK ids an implementer block's Evidence roll call labels
/// `demonstrated`, deduplicated and sorted.
///
/// Line-scoped: a `CHK-NNN` id is treated as demonstrated only when its own
/// line also carries the token `demonstrated`. A line carrying the token but
/// no CHK id, or a CHK id whose line carries no `demonstrated` token,
/// contributes nothing.
#[must_use = "the demonstrated CHK ids drive the evidence gate"]
pub fn demonstrated_chk_ids(implementer_body: &str) -> Vec<String> {
    let mut ids: Vec<String> = implementer_body
        .lines()
        .filter(|line| line.contains("demonstrated"))
        .flat_map(|line| {
            chk_id_regex()
                .find_iter(line)
                .map(|m| m.as_str().to_owned())
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// The number of `### Scenario` headings in an evidence-file body.
///
/// A heading is a line whose first non-whitespace content is the literal
/// `### Scenario` marker; trailing text on the same line (e.g.
/// `### Scenario 1 — ...`) is part of the heading.
#[must_use = "the scenario count decides whether demonstrated claims are backed"]
pub fn scenario_heading_count(evidence_body: &str) -> usize {
    evidence_body
        .lines()
        .filter(|line| line.trim_start().starts_with("### Scenario"))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bullet_form_roll_call_is_detected() {
        let body = "- CHK-001 (range-parser rejection): demonstrated → evidence";
        assert_eq!(demonstrated_chk_ids(body), vec!["CHK-001".to_owned()]);
    }

    #[test]
    fn prose_form_roll_call_is_detected() {
        let body = "CHK-002 demonstrated by some_passing_test in the suite";
        assert_eq!(demonstrated_chk_ids(body), vec!["CHK-002".to_owned()]);
    }

    #[test]
    fn hygiene_and_judgment_only_lines_yield_nothing() {
        let body = "\
- CHK-001 (default when omitted): hygiene → existing test
- CHK-002 (persona-only contract): judgment-only";
        assert!(demonstrated_chk_ids(body).is_empty());
    }

    #[test]
    fn mixed_multiline_body_collects_only_demonstrated() {
        let body = "\
- CHK-001 (range-parser rejection): demonstrated → evidence
- CHK-002 (default when omitted): hygiene → existing test
- CHK-003 (cycle fixture aborts): demonstrated → evidence
- CHK-004 (persona-only contract): judgment-only";
        assert_eq!(
            demonstrated_chk_ids(body),
            vec!["CHK-001".to_owned(), "CHK-003".to_owned()]
        );
    }

    #[test]
    fn token_alone_on_chk_less_line_is_not_a_claim() {
        // The token `demonstrated` appears, but on a line carrying no CHK id;
        // the CHK lines are all hygiene. Nothing is claimed.
        let body = "\
The off-by-one was demonstrated to be fixed under the suite.
- CHK-001 (range-parser rejection): hygiene → existing test";
        assert!(demonstrated_chk_ids(body).is_empty());
    }

    #[test]
    fn duplicate_ids_are_deduplicated_and_sorted() {
        let body = "\
- CHK-003 demonstrated
- CHK-001 demonstrated
- CHK-003 demonstrated again";
        assert_eq!(
            demonstrated_chk_ids(body),
            vec!["CHK-001".to_owned(), "CHK-003".to_owned()]
        );
    }

    #[test]
    fn scenario_heading_count_counts_headings() {
        let body = "\
## Session 2026-05-21
### Scenario 1 — range parser (CHK-001)
some prose
### Scenario 2 — cycle abort (CHK-003)
### not a scenario heading
#### Scenario nested-deeper does not count";
        assert_eq!(scenario_heading_count(body), 2);
    }

    #[test]
    fn scenario_heading_count_zero_when_absent() {
        let body = "## Session\njust prose, no scenario heading at all";
        assert_eq!(scenario_heading_count(body), 0);
    }
}
