//! Workspace-wide supersession index.
//!
//! Computes the inverse of every SPEC.md's `frontmatter.supersedes` so
//! consumers can answer "which specs replace SPEC-X?" without
//! re-scanning every file. See
//! `.speccy/specs/0001-artifact-parsers/SPEC.md` REQ-008.

use crate::parse::SpecMd;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Inverse `supersedes` relation across a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupersessionIndex {
    /// For each spec ID `Y`, the list of IDs `X` that declared
    /// `supersedes: [Y]`, in declared order across the input slice.
    by_target: BTreeMap<String, Vec<String>>,
    /// IDs referenced via `supersedes` that are absent from the input
    /// slice. Used by SPEC-0003 lint without re-scanning.
    dangling: Vec<String>,
}

impl SupersessionIndex {
    /// All specs that supersede `id`, in input order. Returns an empty
    /// slice for IDs nobody replaces.
    #[must_use = "the returned slice carries the inverse relation"]
    pub fn superseded_by(&self, id: &str) -> &[String] {
        self.by_target
            .get(id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// IDs referenced via `supersedes` that no parsed SPEC.md declares.
    #[must_use = "the returned slice lists dangling supersession references"]
    pub fn dangling_references(&self) -> &[String] {
        &self.dangling
    }
}

/// Build a supersession index from a slice of parsed SPEC.mds.
///
/// The function is pure and deterministic: calling twice on the same
/// input slice (in the same order) returns equal indices.
#[must_use = "the returned index answers `superseded_by` queries"]
pub fn supersession_index(specs: &[&SpecMd]) -> SupersessionIndex {
    let known: BTreeSet<String> = specs.iter().map(|s| s.frontmatter.id.clone()).collect();

    let mut by_target: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut dangling_set: BTreeSet<String> = BTreeSet::new();
    let mut dangling_ordered: Vec<String> = Vec::new();

    for spec in specs {
        for target in &spec.frontmatter.supersedes {
            by_target
                .entry(target.clone())
                .or_default()
                .push(spec.frontmatter.id.clone());

            if !known.contains(target) && dangling_set.insert(target.clone()) {
                dangling_ordered.push(target.clone());
            }
        }
    }

    SupersessionIndex {
        by_target,
        dangling: dangling_ordered,
    }
}

#[cfg(test)]
mod tests {
    use super::supersession_index;
    use crate::parse::SpecMd;
    use crate::parse::spec_md::SpecFrontmatter;
    use crate::parse::spec_md::SpecStatus;
    use jiff::civil::Date;

    fn fake_spec(id: &str, supersedes: &[&str]) -> SpecMd {
        SpecMd {
            frontmatter: SpecFrontmatter {
                id: id.to_owned(),
                slug: "x".to_owned(),
                title: "x".to_owned(),
                status: SpecStatus::InProgress,
                created: Date::new(2026, 5, 11).expect("valid date"),
                supersedes: supersedes.iter().map(|s| (*s).to_owned()).collect(),
            },
            requirements: Vec::new(),
            changelog: Vec::new(),
            raw: String::new(),
            sha256: [0u8; 32],
        }
    }

    #[test]
    fn inverts_supersedes_in_input_order() {
        let a = fake_spec("SPEC-0017", &[]);
        let b = fake_spec("SPEC-0042", &["SPEC-0017"]);
        let c = fake_spec("SPEC-0050", &["SPEC-0017", "SPEC-0030"]);
        let specs = vec![&a, &b, &c];

        let index = supersession_index(&specs);
        assert_eq!(
            index.superseded_by("SPEC-0017"),
            &["SPEC-0042".to_owned(), "SPEC-0050".to_owned()]
        );
        assert!(index.superseded_by("SPEC-0042").is_empty());
    }

    #[test]
    fn surfaces_dangling_references() {
        let a = fake_spec("SPEC-0017", &[]);
        let b = fake_spec("SPEC-0050", &["SPEC-0017", "SPEC-0030"]);
        let specs = vec![&a, &b];

        let index = supersession_index(&specs);
        assert_eq!(index.dangling_references(), &["SPEC-0030".to_owned()]);
    }

    #[test]
    fn empty_input_yields_empty_index() {
        let specs: Vec<&SpecMd> = Vec::new();
        let index = supersession_index(&specs);
        assert!(index.superseded_by("anything").is_empty());
        assert!(index.dangling_references().is_empty());
    }

    #[test]
    fn deterministic() {
        let a = fake_spec("SPEC-0001", &[]);
        let b = fake_spec("SPEC-0002", &["SPEC-0001"]);
        let specs = vec![&a, &b];

        let first = supersession_index(&specs);
        let second = supersession_index(&specs);
        assert_eq!(first, second);
    }
}
