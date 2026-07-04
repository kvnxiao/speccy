//! Identifier generation (TERMINOLOGY § ID scope summary).
//!
//! - `spec_ref`  — `SPEC-YYYYMMDD-XXXX`, the one human-addressable handle.
//! - `spec_id` / `run_id` — opaque lowercased ULIDs, prefixed.
//! - internal artifact IDs — short prefixed ULIDs.

use ulid::Ulid;

/// A fresh spec reference `SPEC-YYYYMMDD-XXXX`, dated today (UTC). The 4-char
/// suffix is uppercase hex; callers retry on the rare in-workspace collision.
pub fn spec_ref() -> String {
    let date = jiff::Timestamp::now()
        .to_zoned(jiff::tz::TimeZone::UTC)
        .strftime("%Y%m%d")
        .to_string();
    let suffix: u16 = (Ulid::new().0 & 0xFFFF) as u16;
    format!("SPEC-{date}-{suffix:04X}")
}

/// `spec_<ulid>` (lowercased).
pub fn spec_id() -> String {
    prefixed("spec")
}

/// `run_<ulid>` (lowercased).
pub fn run_id() -> String {
    prefixed("run")
}

fn prefixed(prefix: &str) -> String {
    format!("{prefix}_{}", Ulid::new().to_string().to_lowercase())
}

/// Short prefixed artifact ID, e.g. `ev_12a4`, `fd_77e1`, `ho_9bc2`.
pub fn short_id(prefix: &str) -> String {
    let ulid = Ulid::new().to_string().to_lowercase();
    let tail: String = ulid
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{prefix}_{tail}")
}

/// The run branch name `speccy/<spec-ref-lowercased>-<slug>`
/// (DESIGN § Run Branch and Snapshot Policy).
pub fn run_branch(spec_ref: &str, title: Option<&str>) -> String {
    let base = spec_ref.to_lowercase();
    match title.map(slugify).filter(|s| !s.is_empty()) {
        Some(slug) => format!("speccy/{base}-{slug}"),
        None => format!("speccy/{base}"),
    }
}

/// Lowercase, hyphen-separated slug from a human title (readability only; not
/// an identifier).
pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_ref_has_shape() {
        let r = spec_ref();
        assert!(r.starts_with("SPEC-"), "{r}");
        let parts: Vec<_> = r.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 8);
        assert_eq!(parts[2].len(), 4);
    }

    #[test]
    fn ids_are_prefixed_and_lowercase() {
        assert!(spec_id().starts_with("spec_"));
        assert!(run_id().starts_with("run_"));
        let id = spec_id();
        assert_eq!(id, id.to_lowercase());
    }

    #[test]
    fn branch_uses_slug() {
        let b = run_branch("SPEC-20260630-A7F4", Some("Passwordless Login!"));
        assert_eq!(b, "speccy/spec-20260630-a7f4-passwordless-login");
    }

    #[test]
    fn branch_without_title() {
        assert_eq!(
            run_branch("SPEC-20260630-A7F4", None),
            "speccy/spec-20260630-a7f4"
        );
    }
}
