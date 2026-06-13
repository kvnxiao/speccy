//! Workspace-wide supersession index.
//!
//! Computes the inverse of every SPEC.md's `frontmatter.supersedes` so
//! consumers can answer "which specs replace SPEC-X?" without
//! re-scanning every file.

use crate::parse::SpecMd;
use crate::parse::spec_md::SpecStatus;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Detect supersession-chain orphan candidates triggered by archiving a
/// spec.
///
/// Given the active spec set (every parsed SPEC.md still under
/// `.speccy/specs/`, i.e. not yet relocated to `.speccy/archive/`) and
/// the canonical ID of the spec about to be archived, return the sorted
/// list of active specs `X` that would become orphaned by the archive
/// move. `X` is orphaned when:
///
/// 1. `X` is in the active set,
/// 2. `X`'s status is [`SpecStatus::Superseded`], and
/// 3. `archiving` is the *only* active spec declaring `supersedes: [X]` (after
///    the move, no active spec would explain why `X` is marked superseded, so
///    the SPC-006 lint will fire on `X`).
///
/// Returns an empty `Vec` when `archiving` has an empty `supersedes`
/// list, is absent from the active set, or no candidate matches.
#[must_use = "the returned list drives the archive warning output"]
pub fn orphan_candidates_on_archive(active: &[&SpecMd], archiving: &str) -> Vec<String> {
    // Locate the archiving spec in the active set; if absent, no
    // orphans can be inferred from this scan.
    let Some(src) = active
        .iter()
        .find(|s| s.frontmatter.id == archiving)
        .copied()
    else {
        return Vec::new();
    };

    if src.frontmatter.supersedes.is_empty() {
        return Vec::new();
    }

    let mut out: BTreeSet<String> = BTreeSet::new();
    for target in &src.frontmatter.supersedes {
        // (a) target must be in the active set.
        let Some(target_spec) = active.iter().find(|s| &s.frontmatter.id == target) else {
            continue;
        };
        // (b) target status must be `superseded`.
        if !matches!(target_spec.frontmatter.status, SpecStatus::Superseded) {
            continue;
        }
        // (c) no other active spec besides `archiving` declares
        //     `supersedes: [target]`.
        let other_declarers = active.iter().any(|s| {
            s.frontmatter.id != archiving && s.frontmatter.supersedes.iter().any(|t| t == target)
        });
        if other_declarers {
            continue;
        }
        out.insert(target.clone());
    }

    out.into_iter().collect()
}

/// Inverse `supersedes` relation across a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupersessionIndex {
    /// For each spec ID `Y`, the list of IDs `X` that declared
    /// `supersedes: [Y]`, in declared order across the input slice.
    by_target: BTreeMap<String, Vec<String>>,
    /// IDs referenced via `supersedes` that are absent from the input
    /// slice. Used by the dangling-reference lint without re-scanning.
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
    use super::orphan_candidates_on_archive;
    use super::supersession_index;
    use crate::parse::SpecMd;
    use crate::parse::spec_md::SpecFrontmatter;
    use crate::parse::spec_md::SpecStatus;
    use jiff::civil::Date;

    fn fake_spec(id: &str, supersedes: &[&str]) -> SpecMd {
        fake_spec_with_status(id, supersedes, SpecStatus::InProgress)
    }

    fn fake_spec_with_status(id: &str, supersedes: &[&str], status: SpecStatus) -> SpecMd {
        SpecMd {
            frontmatter: SpecFrontmatter {
                id: id.to_owned(),
                slug: "x".to_owned(),
                title: "x".to_owned(),
                status,
                created: Date::new(2026, 5, 11).expect("valid date"),
                supersedes: supersedes.iter().map(|s| (*s).to_owned()).collect(),
                archived_at: None,
                archived_reason: None,
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

    #[test]
    fn orphan_warn_sole_declarer() {
        // SPEC-0019 active, status: superseded; SPEC-0021 sole declarer.
        let old = fake_spec_with_status("SPEC-0019", &[], SpecStatus::Superseded);
        let new = fake_spec_with_status("SPEC-0021", &["SPEC-0019"], SpecStatus::Implemented);
        let active = vec![&old, &new];
        assert_eq!(
            orphan_candidates_on_archive(&active, "SPEC-0021"),
            vec!["SPEC-0019".to_owned()]
        );
    }

    #[test]
    fn orphan_natural_archive_older_returns_empty() {
        let old = fake_spec_with_status("SPEC-0019", &[], SpecStatus::Superseded);
        let new = fake_spec_with_status("SPEC-0021", &["SPEC-0019"], SpecStatus::Implemented);
        let active = vec![&old, &new];
        // Archiving SPEC-0019 (the older, superseded one): SPEC-0019
        // has empty `supersedes`, so no orphan candidates surface.
        assert!(orphan_candidates_on_archive(&active, "SPEC-0019").is_empty());
    }

    #[test]
    fn orphan_multi_declarer_returns_empty() {
        let old = fake_spec_with_status("SPEC-0019", &[], SpecStatus::Superseded);
        let new_a = fake_spec_with_status("SPEC-0021", &["SPEC-0019"], SpecStatus::Implemented);
        let new_b = fake_spec_with_status("SPEC-0022", &["SPEC-0019"], SpecStatus::Implemented);
        let active = vec![&old, &new_a, &new_b];
        // SPEC-0022 still declares supersedes: [SPEC-0019] after SPEC-0021
        // is archived, so SPEC-0019 is not orphaned.
        assert!(orphan_candidates_on_archive(&active, "SPEC-0021").is_empty());
    }

    #[test]
    fn orphan_target_not_in_active_set_returns_empty() {
        // SPEC-0019 absent from active set (already archived); SPEC-0021
        // still declares supersedes: [SPEC-0019].
        let new = fake_spec_with_status("SPEC-0021", &["SPEC-0019"], SpecStatus::Implemented);
        let active = vec![&new];
        assert!(orphan_candidates_on_archive(&active, "SPEC-0021").is_empty());
    }

    #[test]
    fn orphan_target_not_superseded_returns_empty() {
        // SPEC-0019 active but status: implemented (not superseded).
        let old = fake_spec_with_status("SPEC-0019", &[], SpecStatus::Implemented);
        let new = fake_spec_with_status("SPEC-0021", &["SPEC-0019"], SpecStatus::Implemented);
        let active = vec![&old, &new];
        assert!(orphan_candidates_on_archive(&active, "SPEC-0021").is_empty());
    }

    #[test]
    fn orphan_empty_supersedes_returns_empty() {
        let solo = fake_spec_with_status("SPEC-0030", &[], SpecStatus::Implemented);
        let active = vec![&solo];
        assert!(orphan_candidates_on_archive(&active, "SPEC-0030").is_empty());
    }
}
