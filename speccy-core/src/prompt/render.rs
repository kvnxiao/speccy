//! Single-pass `{{NAME}}` placeholder substitution.
//!
//! Names match `[A-Za-z0-9_]+`. Substituted text is **not** rescanned
//! for further placeholders. Unrecognised placeholders are left in
//! place and emit one stderr warning per unique unmatched name (per
//! SPEC-0005 REQ-005).

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::io::Write;

/// Substitute `{{name}}` tokens in `template` using `vars`, writing
/// per-unknown warnings to `warn_out`.
///
/// Single-pass: substituted text is not re-scanned. Unknown
/// placeholders are emitted verbatim; one warning line per **unique**
/// unmatched name goes to `warn_out`.
///
/// Public callers should reach for [`render`], which wraps this with
/// `std::io::stderr()` as the warning sink. The injected sink lets
/// tests assert warning content without capturing process stderr.
#[must_use = "the rendered prompt is the function's output"]
pub fn render_with_warn<W: Write>(
    template: &str,
    vars: &BTreeMap<&str, String>,
    warn_out: &mut W,
) -> String {
    let mut out = String::with_capacity(template.len());
    let mut unknown_reported: BTreeSet<String> = BTreeSet::new();
    let bytes = template.as_bytes();
    let len = bytes.len();
    let mut cursor: usize = 0;

    while cursor < len {
        let Some(open) = find_open_brace_pair(bytes, cursor) else {
            out.push_str(&copy_through(template, cursor, len));
            cursor = len;
            continue;
        };
        out.push_str(&copy_through(template, cursor, open));
        let name_start = open.saturating_add(2);
        match find_close_brace_pair(bytes, name_start) {
            Some(close) if is_valid_name(template, name_start, close) => {
                let name = copy_through(template, name_start, close);
                if let Some(value) = vars.get(name.as_str()) {
                    out.push_str(value);
                } else {
                    out.push_str("{{");
                    out.push_str(&name);
                    out.push_str("}}");
                    if unknown_reported.insert(name.clone())
                        && writeln!(
                            warn_out,
                            "speccy prompt: unknown placeholder `{{{{{name}}}}}`",
                        )
                        .is_err()
                    {
                        // Warning sink is closed; nothing actionable.
                    }
                }
                cursor = close.saturating_add(2);
            }
            _ => {
                out.push_str("{{");
                cursor = name_start;
            }
        }
    }
    out
}

/// Convenience wrapper around [`render_with_warn`] that emits unknown
/// placeholder warnings to process stderr.
#[must_use = "the rendered prompt is the function's output"]
pub fn render(template: &str, vars: &BTreeMap<&str, String>) -> String {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    render_with_warn(template, vars, &mut lock)
}

fn copy_through(template: &str, start: usize, end: usize) -> String {
    let bytes = template.as_bytes();
    let slice = bytes.get(start..end).unwrap_or_default();
    String::from_utf8_lossy(slice).into_owned()
}

fn find_open_brace_pair(bytes: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i.saturating_add(1) < bytes.len() {
        let a = bytes.get(i).copied()?;
        let b = bytes.get(i.saturating_add(1)).copied()?;
        if a == b'{' && b == b'{' {
            return Some(i);
        }
        i = i.saturating_add(1);
    }
    None
}

fn find_close_brace_pair(bytes: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i.saturating_add(1) < bytes.len() {
        let a = bytes.get(i).copied()?;
        let b = bytes.get(i.saturating_add(1)).copied()?;
        if a == b'}' && b == b'}' {
            return Some(i);
        }
        i = i.saturating_add(1);
    }
    None
}

fn is_valid_name(template: &str, start: usize, end: usize) -> bool {
    if end <= start {
        return false;
    }
    let bytes = template.as_bytes();
    let Some(slice) = bytes.get(start..end) else {
        return false;
    };
    slice
        .iter()
        .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

#[cfg(test)]
mod tests {
    use super::render_with_warn;
    use std::collections::BTreeMap;

    fn vars(pairs: &[(&'static str, &str)]) -> BTreeMap<&'static str, String> {
        pairs.iter().map(|(k, v)| (*k, (*v).to_owned())).collect()
    }

    #[test]
    fn substitutes_single_placeholder() {
        let mut warns = Vec::new();
        let out = render_with_warn("hello {{name}}", &vars(&[("name", "world")]), &mut warns);
        assert_eq!(out, "hello world");
        assert!(warns.is_empty(), "no warnings expected, got: {warns:?}");
    }

    #[test]
    fn substitutes_multiple_placeholders() {
        let mut warns = Vec::new();
        let out = render_with_warn(
            "{{greeting}} {{name}}!",
            &vars(&[("greeting", "hi"), ("name", "kev")]),
            &mut warns,
        );
        assert_eq!(out, "hi kev!");
        assert!(warns.is_empty());
    }

    #[test]
    fn single_pass_does_not_rescan_substituted_text() {
        let mut warns = Vec::new();
        let out = render_with_warn(
            "{{a}} {{b}}",
            &vars(&[("a", "{{b}}"), ("b", "x")]),
            &mut warns,
        );
        assert_eq!(
            out, "{{b}} x",
            "substituted `{{{{b}}}}` from a must NOT be re-scanned"
        );
        assert!(warns.is_empty());
    }

    #[test]
    fn unknown_placeholder_left_in_place_and_warned_once() {
        let mut warns = Vec::new();
        let out = render_with_warn("{{unknown}}", &vars(&[]), &mut warns);
        assert_eq!(out, "{{unknown}}");
        let text = String::from_utf8(warns).expect("warning output is UTF-8");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "expected exactly one warning line for unknown placeholder, got: {text}",
        );
        assert!(
            text.contains("`{{unknown}}`"),
            "warning should name the unknown placeholder, got: {text}",
        );
    }

    #[test]
    fn duplicate_unknown_placeholders_warn_once_per_unique_name() {
        let mut warns = Vec::new();
        let out = render_with_warn("{{a}} {{a}} {{b}} {{a}}", &vars(&[]), &mut warns);
        assert_eq!(out, "{{a}} {{a}} {{b}} {{a}}");
        let text = String::from_utf8(warns).expect("warning output is UTF-8");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "expected one warning per unique unknown name (a, b), got: {text}",
        );
    }

    #[test]
    fn empty_template_and_vars_produce_empty_output() {
        let mut warns = Vec::new();
        let out = render_with_warn("", &vars(&[]), &mut warns);
        assert_eq!(out, "");
        assert!(warns.is_empty());
    }

    #[test]
    fn literal_braces_without_matching_pair_left_alone() {
        let mut warns = Vec::new();
        let out = render_with_warn(
            "rust code: fn x() {} and {{ no close",
            &vars(&[]),
            &mut warns,
        );
        assert_eq!(out, "rust code: fn x() {} and {{ no close");
        assert!(warns.is_empty());
    }

    #[test]
    fn invalid_placeholder_characters_left_alone() {
        let mut warns = Vec::new();
        let out = render_with_warn("{{has space}}", &vars(&[]), &mut warns);
        assert_eq!(
            out, "{{has space}}",
            "names with spaces are not valid placeholders; emit verbatim",
        );
        assert!(
            warns.is_empty(),
            "non-name brace contents should not produce unknown-placeholder warnings",
        );
    }
}
