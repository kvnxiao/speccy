//! Symmetric REQ-ID diff between SPEC.md and spec.toml.
//!
//! Pure, deterministic, idempotent. See
//! `.speccy/specs/0001-artifact-parsers/SPEC.md` REQ-006.

use crate::parse::SpecMd;
use crate::parse::SpecToml;
use std::collections::HashSet;

/// Symmetric diff between SPEC.md REQ headings and `spec.toml`
/// `[[requirements]]` rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossRef {
    /// IDs that appear in SPEC.md but not in `spec.toml`, in SPEC.md
    /// declared order.
    pub only_in_spec_md: Vec<String>,
    /// IDs that appear in `spec.toml` but not in SPEC.md, in
    /// `spec.toml` declared order.
    pub only_in_toml: Vec<String>,
    /// IDs present on both sides, in SPEC.md declared order.
    pub in_both: Vec<String>,
}

/// Compute the symmetric REQ-ID diff between SPEC.md and `spec.toml`.
#[must_use = "the diff is the entire purpose of this call"]
pub fn cross_ref(spec: &SpecMd, toml: &SpecToml) -> CrossRef {
    let md_ids: Vec<&str> = spec.requirements.iter().map(|r| r.id.as_str()).collect();
    let toml_ids: Vec<&str> = toml.requirements.iter().map(|r| r.id.as_str()).collect();

    let md_set: HashSet<&str> = md_ids.iter().copied().collect();
    let toml_set: HashSet<&str> = toml_ids.iter().copied().collect();

    let only_in_spec_md: Vec<String> = md_ids
        .iter()
        .filter(|id| !toml_set.contains(*id))
        .map(|id| (*id).to_owned())
        .collect();

    let only_in_toml: Vec<String> = toml_ids
        .iter()
        .filter(|id| !md_set.contains(*id))
        .map(|id| (*id).to_owned())
        .collect();

    let in_both: Vec<String> = md_ids
        .iter()
        .filter(|id| toml_set.contains(*id))
        .map(|id| (*id).to_owned())
        .collect();

    CrossRef {
        only_in_spec_md,
        only_in_toml,
        in_both,
    }
}

#[cfg(test)]
mod tests {
    use super::cross_ref;
    use crate::parse::CheckEntry;
    use crate::parse::CheckPayload;
    use crate::parse::RequirementEntry;
    use crate::parse::SpecMd;
    use crate::parse::SpecToml;
    use crate::parse::spec_md::ReqHeading;
    use crate::parse::spec_md::SpecFrontmatter;
    use crate::parse::spec_md::SpecStatus;
    use jiff::civil::Date;

    fn fake_spec(md_ids: &[&str]) -> SpecMd {
        SpecMd {
            frontmatter: SpecFrontmatter {
                id: "SPEC-0000".into(),
                slug: "x".into(),
                title: "x".into(),
                status: SpecStatus::InProgress,
                created: Date::new(2026, 5, 11).expect("valid date"),
                supersedes: Vec::new(),
            },
            requirements: md_ids
                .iter()
                .enumerate()
                .map(|(i, id)| ReqHeading {
                    id: (*id).to_owned(),
                    title: format!("title {i}"),
                    line: i.saturating_add(1),
                })
                .collect(),
            changelog: Vec::new(),
            raw: String::new(),
            sha256: [0u8; 32],
        }
    }

    fn fake_toml(toml_ids: &[&str]) -> SpecToml {
        SpecToml {
            requirements: toml_ids
                .iter()
                .map(|id| RequirementEntry {
                    id: (*id).to_owned(),
                    checks: vec!["CHK-000".to_owned()],
                })
                .collect(),
            checks: vec![CheckEntry {
                id: "CHK-000".to_owned(),
                kind: "test".to_owned(),
                proves: "x".to_owned(),
                payload: CheckPayload::Command("cargo test".to_owned()),
            }],
        }
    }

    #[test]
    fn partitions_ids_symmetrically() {
        let spec = fake_spec(&["REQ-001", "REQ-002", "REQ-003"]);
        let toml = fake_toml(&["REQ-001", "REQ-002", "REQ-004"]);
        let diff = cross_ref(&spec, &toml);
        assert_eq!(diff.only_in_spec_md, vec!["REQ-003".to_owned()]);
        assert_eq!(diff.only_in_toml, vec!["REQ-004".to_owned()]);
        assert_eq!(
            diff.in_both,
            vec!["REQ-001".to_owned(), "REQ-002".to_owned()]
        );
    }

    #[test]
    fn preserves_declared_order() {
        let spec = fake_spec(&["REQ-003", "REQ-001", "REQ-002"]);
        let toml = fake_toml(&["REQ-002", "REQ-001"]);
        let diff = cross_ref(&spec, &toml);
        // `in_both` order should match SPEC.md order, not toml order.
        assert_eq!(
            diff.in_both,
            vec!["REQ-001".to_owned(), "REQ-002".to_owned()]
        );
    }

    #[test]
    fn idempotent() {
        let spec = fake_spec(&["REQ-001", "REQ-002"]);
        let toml = fake_toml(&["REQ-001", "REQ-003"]);
        let first = cross_ref(&spec, &toml);
        let second = cross_ref(&spec, &toml);
        assert_eq!(first, second);
    }

    #[test]
    fn empty_inputs_yield_empty_diff() {
        let spec = fake_spec(&[]);
        let toml = fake_toml(&[]);
        let diff = cross_ref(&spec, &toml);
        assert!(diff.only_in_spec_md.is_empty());
        assert!(diff.only_in_toml.is_empty());
        assert!(diff.in_both.is_empty());
    }
}
