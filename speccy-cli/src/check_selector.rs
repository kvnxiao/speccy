//! Polymorphic positional selector parser for `speccy check`.
//!
//! `speccy check` accepts a single optional positional argument with five
//! possible shapes (see `.speccy/specs/0017-check-selector/SPEC.md`):
//!
//! | Argument               | Selector variant                      |
//! |------------------------|---------------------------------------|
//! | absent                 | `CheckSelector::All`                  |
//! | `SPEC-NNNN`            | `CheckSelector::Spec`                 |
//! | `SPEC-NNNN/CHK-NNN`    | `CheckSelector::QualifiedCheck`       |
//! | `SPEC-NNNN/T-NNN`      | `CheckSelector::Task(Qualified)`      |
//! | `CHK-NNN`              | `CheckSelector::UnqualifiedCheck`     |
//! | `T-NNN`                | `CheckSelector::Task(Unqualified)`    |
//!
//! Task forms reuse [`speccy_core::task_lookup::parse_ref`] verbatim so
//! ambiguity and not-found semantics stay aligned with `speccy implement`
//! and `speccy review` (SPEC-0017 DEC-002).
//!
//! Dispatch order tests the most specific shapes first (qualified-task,
//! qualified-check, bare spec, unqualified task, unqualified check). The
//! five accepted shapes are mutually disjoint, so order is a safety net,
//! not a semantic dependency (see SPEC's Assumptions section).

use regex::Regex;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskRef;
use speccy_core::task_lookup::parse_ref;
use std::sync::OnceLock;
use thiserror::Error;

/// Parsed `speccy check` selector argument.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CheckSelector {
    /// No argument supplied; run every check across every spec.
    All,
    /// `SPEC-NNNN`: every check under the named spec.
    Spec {
        /// `SPEC-NNNN` identifier (4+ digits).
        spec_id: String,
    },
    /// `SPEC-NNNN/CHK-NNN`: one check, spec-qualified.
    QualifiedCheck {
        /// `SPEC-NNNN` identifier (4+ digits).
        spec_id: String,
        /// `CHK-NNN` identifier (3+ digits).
        check_id: String,
    },
    /// `CHK-NNN`: every spec's `CHK-NNN` (SPEC-0010 DEC-003 cross-spec
    /// match preserved).
    UnqualifiedCheck {
        /// `CHK-NNN` identifier (3+ digits).
        check_id: String,
    },
    /// `T-NNN` or `SPEC-NNNN/T-NNN`: checks proving the requirements the
    /// task covers. Resolution is delegated to `task_lookup::find`.
    Task(TaskRef),
}

/// Selector-layer parse and resolution failures.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SelectorError {
    /// Argument did not match any of the five accepted shapes.
    #[error(
        "invalid selector `{arg}`; expected one of: SPEC-NNNN, \
         SPEC-NNNN/CHK-NNN, SPEC-NNNN/T-NNN, CHK-NNN, T-NNN"
    )]
    InvalidFormat {
        /// Verbatim user input.
        arg: String,
    },
    /// Named spec was not found in the workspace.
    #[error("no spec `{spec_id}` found in workspace")]
    NoSpecMatching {
        /// Spec ID that produced no match.
        spec_id: String,
    },
    /// Named check was not present in the named spec.
    #[error("no `{check_id}` in `{spec_id}`")]
    NoQualifiedCheckMatching {
        /// Spec the user qualified the check against.
        spec_id: String,
        /// Check ID absent from that spec.
        check_id: String,
    },
}

/// Parse the optional positional argument to `speccy check` into a
/// [`CheckSelector`].
///
/// Dispatch tests the most specific shapes first:
///
/// 1. `SPEC-NNNN/T-NNN` (qualified task)
/// 2. `SPEC-NNNN/CHK-NNN` (qualified check)
/// 3. `SPEC-NNNN` (bare spec)
/// 4. `T-NNN` (unqualified task)
/// 5. `CHK-NNN` (unqualified check)
///
/// Task shapes (1 and 4) reuse [`speccy_core::task_lookup::parse_ref`] so
/// ambiguity and not-found errors stay byte-for-byte aligned with
/// `speccy implement` / `speccy review`.
///
/// # Errors
///
/// Returns [`SelectorError::InvalidFormat`] when `arg` is `Some` but does
/// not match any of the five accepted shapes. The offending input is
/// preserved verbatim (no case normalisation, no whitespace trimming) so
/// CLI surfaces can name it back to the user.
pub fn parse_selector(arg: Option<&str>) -> Result<CheckSelector, SelectorError> {
    let Some(raw) = arg else {
        return Ok(CheckSelector::All);
    };

    if let Some(caps) = qualified_task_regex().captures(raw) {
        let spec_id = caps
            .get(1)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        let task_id = caps
            .get(2)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        // Re-parse the fragment through task_lookup so the canonical
        // TaskRef constructor is the only place a Qualified is built.
        let composed = format!("{spec_id}/{task_id}");
        let task_ref = parse_ref(&composed).map_err(|e| match e {
            LookupError::InvalidFormat { arg } => SelectorError::InvalidFormat { arg },
            // Should not occur: the regex already constrained the input.
            _ => SelectorError::InvalidFormat {
                arg: raw.to_owned(),
            },
        })?;
        return Ok(CheckSelector::Task(task_ref));
    }

    if let Some(caps) = qualified_check_regex().captures(raw) {
        let spec_id = caps
            .get(1)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        let check_id = caps
            .get(2)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        return Ok(CheckSelector::QualifiedCheck { spec_id, check_id });
    }

    if bare_spec_regex().is_match(raw) {
        return Ok(CheckSelector::Spec {
            spec_id: raw.to_owned(),
        });
    }

    if unqualified_task_regex().is_match(raw) {
        let task_ref = parse_ref(raw).map_err(|e| match e {
            LookupError::InvalidFormat { arg } => SelectorError::InvalidFormat { arg },
            _ => SelectorError::InvalidFormat {
                arg: raw.to_owned(),
            },
        })?;
        return Ok(CheckSelector::Task(task_ref));
    }

    if unqualified_check_regex().is_match(raw) {
        return Ok(CheckSelector::UnqualifiedCheck {
            check_id: raw.to_owned(),
        });
    }

    Err(SelectorError::InvalidFormat {
        arg: raw.to_owned(),
    })
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn qualified_task_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^(SPEC-\d{4,})/(T-\d{3,})$").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn qualified_check_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^(SPEC-\d{4,})/(CHK-\d{3,})$").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn bare_spec_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^SPEC-\d{4,}$").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn unqualified_task_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^T-\d{3,}$").unwrap())
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn unqualified_check_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^CHK-\d{3,}$").unwrap())
}

#[cfg(test)]
mod tests {
    use super::CheckSelector;
    use super::SelectorError;
    use super::parse_selector;
    use speccy_core::task_lookup::TaskRef;

    #[test]
    fn none_returns_all() {
        let got = parse_selector(None).expect("None must parse to CheckSelector::All");
        assert_eq!(got, CheckSelector::All);
    }

    #[test]
    fn qualified_task_delegates_to_task_lookup() {
        // Building the expected value through task_lookup confirms the
        // task fragment is produced via parse_ref and not a second copy.
        let expected = CheckSelector::Task(
            speccy_core::task_lookup::parse_ref("SPEC-0010/T-002")
                .expect("fixture: SPEC-0010/T-002 must parse via task_lookup"),
        );
        let got = parse_selector(Some("SPEC-0010/T-002"))
            .expect("SPEC-0010/T-002 must parse to a qualified task");
        assert_eq!(got, expected);
        assert!(
            matches!(
                &got,
                CheckSelector::Task(TaskRef::Qualified { spec_id, task_id })
                    if spec_id == "SPEC-0010" && task_id == "T-002",
            ),
            "expected Qualified{{SPEC-0010/T-002}}, got {got:?}",
        );
    }

    #[test]
    fn qualified_check_parses() {
        let got = parse_selector(Some("SPEC-0010/CHK-001")).expect("SPEC-0010/CHK-001 must parse");
        assert_eq!(
            got,
            CheckSelector::QualifiedCheck {
                spec_id: "SPEC-0010".to_owned(),
                check_id: "CHK-001".to_owned(),
            },
        );
    }

    #[test]
    fn bare_spec_parses() {
        let got = parse_selector(Some("SPEC-0010")).expect("SPEC-0010 must parse");
        assert_eq!(
            got,
            CheckSelector::Spec {
                spec_id: "SPEC-0010".to_owned(),
            },
        );
    }

    #[test]
    fn unqualified_task_parses() {
        let got = parse_selector(Some("T-002")).expect("T-002 must parse");
        assert_eq!(
            got,
            CheckSelector::Task(TaskRef::Unqualified {
                id: "T-002".to_owned(),
            }),
        );
    }

    #[test]
    fn unqualified_check_parses() {
        let got = parse_selector(Some("CHK-001")).expect("CHK-001 must parse");
        assert_eq!(
            got,
            CheckSelector::UnqualifiedCheck {
                check_id: "CHK-001".to_owned(),
            },
        );
    }

    #[test]
    fn foo_invalid_and_display_lists_five_shapes() {
        let err = parse_selector(Some("FOO")).expect_err("FOO must be rejected");
        assert!(
            matches!(&err, SelectorError::InvalidFormat { arg } if arg == "FOO"),
            "expected InvalidFormat{{FOO}}, got {err:?}",
        );
        let display = format!("{err}");
        assert!(display.contains("FOO"), "Display must name FOO: {display}");
        for shape in &[
            "SPEC-NNNN",
            "SPEC-NNNN/CHK-NNN",
            "SPEC-NNNN/T-NNN",
            "CHK-NNN",
            "T-NNN",
        ] {
            assert!(
                display.contains(shape),
                "Display must list `{shape}`: {display}",
            );
        }
    }

    #[test]
    fn lowercase_chk_not_normalised() {
        let err = parse_selector(Some("chk-001")).expect_err("chk-001 must be rejected");
        assert!(
            matches!(&err, SelectorError::InvalidFormat { arg } if arg == "chk-001"),
            "expected InvalidFormat{{chk-001}}, got {err:?}",
        );
    }

    #[test]
    fn malformed_inputs_carry_exact_arg() {
        for bad in &["", "SPEC-", "SPEC-001", "SPEC-0001/", "/T-001", "T- 001"] {
            let err = parse_selector(Some(bad)).expect_err("malformed input must fail");
            assert!(
                matches!(&err, SelectorError::InvalidFormat { arg } if arg == bad),
                "expected InvalidFormat{{{bad}}}, got {err:?}",
            );
        }
    }

    #[test]
    fn dispatch_order_walks_each_accepted_form() {
        let cases: &[(&str, CheckSelector)] = &[
            (
                "SPEC-0010/T-002",
                CheckSelector::Task(TaskRef::Qualified {
                    spec_id: "SPEC-0010".to_owned(),
                    task_id: "T-002".to_owned(),
                }),
            ),
            (
                "SPEC-0010/CHK-001",
                CheckSelector::QualifiedCheck {
                    spec_id: "SPEC-0010".to_owned(),
                    check_id: "CHK-001".to_owned(),
                },
            ),
            (
                "SPEC-0010",
                CheckSelector::Spec {
                    spec_id: "SPEC-0010".to_owned(),
                },
            ),
            (
                "T-002",
                CheckSelector::Task(TaskRef::Unqualified {
                    id: "T-002".to_owned(),
                }),
            ),
            (
                "CHK-001",
                CheckSelector::UnqualifiedCheck {
                    check_id: "CHK-001".to_owned(),
                },
            ),
        ];
        for (input, expected) in cases {
            let got = parse_selector(Some(input)).expect("accepted form must parse");
            assert_eq!(
                &got, expected,
                "dispatch regression for `{input}`: got {got:?}, want {expected:?}",
            );
        }
    }
}
