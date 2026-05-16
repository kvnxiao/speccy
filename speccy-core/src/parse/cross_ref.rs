//! Symmetric REQ-ID diff between SPEC.md headings and SPEC.md marker tree.
//!
//! Before SPEC-0019 this compared SPEC.md against per-spec `spec.toml`.
//! After SPEC-0019 the requirement graph lives in the SPEC.md marker
//! tree (see [`crate::parse::spec_markers`]); the heading view from
//! [`SpecMd`] and the marker view from [`SpecDoc`] should agree on the
//! same REQ-ID set.
//!
//! Pure, deterministic, idempotent. See
//! `.speccy/specs/0001-artifact-parsers/SPEC.md` REQ-006.

use crate::parse::SpecDoc;
use crate::parse::SpecMd;
use std::collections::HashSet;

/// Symmetric diff between SPEC.md REQ headings and `speccy:requirement`
/// markers in the same SPEC.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossRef {
    /// IDs that appear in SPEC.md headings but not in the marker tree,
    /// in heading-declared order.
    pub only_in_spec_md: Vec<String>,
    /// IDs that appear in the marker tree but not in SPEC.md headings,
    /// in marker-declared order.
    pub only_in_markers: Vec<String>,
    /// IDs present on both sides, in SPEC.md heading-declared order.
    pub in_both: Vec<String>,
}

/// Compute the symmetric REQ-ID diff between SPEC.md headings and the
/// SPEC.md marker tree.
#[must_use = "the diff is the entire purpose of this call"]
pub fn cross_ref(spec: &SpecMd, doc: &SpecDoc) -> CrossRef {
    let md_ids: Vec<&str> = spec.requirements.iter().map(|r| r.id.as_str()).collect();
    let marker_ids: Vec<&str> = doc.requirements.iter().map(|r| r.id.as_str()).collect();

    let md_set: HashSet<&str> = md_ids.iter().copied().collect();
    let marker_set: HashSet<&str> = marker_ids.iter().copied().collect();

    let only_in_spec_md: Vec<String> = md_ids
        .iter()
        .filter(|id| !marker_set.contains(*id))
        .map(|id| (*id).to_owned())
        .collect();

    let only_in_markers: Vec<String> = marker_ids
        .iter()
        .filter(|id| !md_set.contains(*id))
        .map(|id| (*id).to_owned())
        .collect();

    let in_both: Vec<String> = md_ids
        .iter()
        .filter(|id| marker_set.contains(*id))
        .map(|id| (*id).to_owned())
        .collect();

    CrossRef {
        only_in_spec_md,
        only_in_markers,
        in_both,
    }
}
