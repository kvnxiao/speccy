//! Markdown frontmatter splitter.
//!
//! Splits a markdown source into `(yaml_frontmatter, body)` pairs. The
//! implementation is intentionally narrow — four string slicing steps — so
//! frontmatter parsing is not coupled to any particular YAML crate. See
//! `.speccy/specs/0001-artifact-parsers/SPEC.md` REQ-002.

use crate::error::ParseError;
use camino::Utf8Path;

/// Outcome of attempting to split a markdown source into frontmatter and
/// body.
#[derive(Debug, PartialEq, Eq)]
pub enum Split<'a> {
    /// File starts with a `---` fence and the closing fence was found.
    Some {
        /// YAML payload between the opening and closing fences (exclusive).
        yaml: &'a str,
        /// Body content immediately after the closing fence.
        body: &'a str,
    },
    /// File does not start with a `---` fence; no frontmatter is present.
    None,
}

impl<'a> Split<'a> {
    /// Convert into an `Option<(yaml, body)>` for ergonomic test
    /// assertions.
    #[must_use = "the returned Option carries the parsed frontmatter pair"]
    pub fn into_option(self) -> Option<(&'a str, &'a str)> {
        match self {
            Split::Some { yaml, body } => Some((yaml, body)),
            Split::None => None,
        }
    }
}

/// Split a markdown source into frontmatter and body.
///
/// # Behavior
///
/// - A file beginning with `---\n<yaml>\n---\n<body>` returns [`Split::Some`].
/// - A file that does not start with a `---` fence (after optional UTF-8 BOM
///   stripping) returns [`Split::None`].
/// - A file that begins with an opening `---` but no matching closing `---`
///   returns [`ParseError::UnterminatedFrontmatter`].
/// - `\r\n` line endings are treated identically to `\n`.
///
/// # Errors
///
/// Returns [`ParseError::UnterminatedFrontmatter`] if an opening fence is
/// present without a matching closing fence.
pub fn split<'a>(source: &'a str, path: &Utf8Path) -> Result<Split<'a>, ParseError> {
    let source = strip_utf8_bom(source);

    let Some(after_open) = strip_opening_fence(source) else {
        return Ok(Split::None);
    };

    let Some((yaml, body)) = find_closing_fence(after_open) else {
        return Err(ParseError::UnterminatedFrontmatter {
            path: path.to_path_buf(),
        });
    };

    Ok(Split::Some { yaml, body })
}

fn strip_utf8_bom(source: &str) -> &str {
    source.strip_prefix('\u{feff}').unwrap_or(source)
}

fn strip_opening_fence(source: &str) -> Option<&str> {
    let rest = source.strip_prefix("---")?;
    skip_eol_or_eof(rest)
}

/// Return the YAML payload up to (but not including) the closing fence, and
/// the body immediately after the closing fence's terminating newline (or
/// `""` if the file ends with the fence). Returns `None` if no closing
/// fence is found.
fn find_closing_fence(after_open: &str) -> Option<(&str, &str)> {
    let mut cursor = 0usize;
    while cursor <= after_open.len() {
        let remainder = after_open.get(cursor..)?;
        if remainder.starts_with("---") {
            let after_dashes = remainder.get(3..).unwrap_or("");
            if let Some(post_line) = skip_eol_or_eof(after_dashes) {
                let yaml = after_open.get(..cursor)?;
                return Some((yaml, post_line));
            }
        }
        let line_break = remainder.find('\n')?;
        cursor = cursor.checked_add(line_break)?.checked_add(1)?;
    }
    None
}

/// If `s` begins with a line terminator (LF or CRLF) or is empty, consume
/// any leading spaces/tabs and the terminator, returning the rest. Returns
/// `None` if `s` has non-whitespace content before the terminator.
fn skip_eol_or_eof(s: &str) -> Option<&str> {
    let trimmed = s.trim_start_matches([' ', '\t']);
    if trimmed.is_empty() {
        return Some("");
    }
    if let Some(rest) = trimmed.strip_prefix("\r\n") {
        return Some(rest);
    }
    trimmed.strip_prefix('\n')
}

#[cfg(test)]
mod tests {
    use super::Split;
    use super::split;
    use camino::Utf8Path;

    fn fake_path() -> &'static Utf8Path {
        Utf8Path::new("fake/file.md")
    }

    #[test]
    fn splits_valid_lf_frontmatter() {
        let src = "---\nkey: value\n---\nbody text\n";
        let (yaml, body) = split(src, fake_path())
            .expect("split should succeed")
            .into_option()
            .expect("expected Some");
        assert_eq!(yaml, "key: value\n");
        assert_eq!(body, "body text\n");
    }

    #[test]
    fn splits_valid_crlf_frontmatter() {
        let src = "---\r\nkey: value\r\n---\r\nbody text\r\n";
        let (yaml, body) = split(src, fake_path())
            .expect("split should succeed")
            .into_option()
            .expect("expected Some");
        assert_eq!(yaml, "key: value\r\n");
        assert_eq!(body, "body text\r\n");
    }

    #[test]
    fn returns_none_without_leading_fence() {
        let src = "no frontmatter here\n";
        let outcome = split(src, fake_path()).expect("split should succeed");
        assert_eq!(outcome, Split::None);
    }

    #[test]
    fn errors_on_unterminated_fence() {
        let src = "---\nkey: value\nno closing fence here\n";
        let err = split(src, fake_path()).expect_err("unterminated fence");
        assert!(matches!(
            err,
            crate::error::ParseError::UnterminatedFrontmatter { .. }
        ));
    }

    #[test]
    fn body_horizontal_rule_does_not_close_fence() {
        let src = "---\nkey: value\n---\nfirst\n\n---\nsecond\n";
        let (yaml, body) = split(src, fake_path())
            .expect("split should succeed")
            .into_option()
            .expect("expected Some");
        assert_eq!(yaml, "key: value\n");
        assert_eq!(body, "first\n\n---\nsecond\n");
    }

    #[test]
    fn empty_frontmatter_and_body() {
        let src = "---\n---\n";
        let (yaml, body) = split(src, fake_path())
            .expect("split should succeed")
            .into_option()
            .expect("expected Some");
        assert_eq!(yaml, "");
        assert_eq!(body, "");
    }

    #[test]
    fn empty_frontmatter_no_trailing_newline() {
        let src = "---\n---";
        let (yaml, body) = split(src, fake_path())
            .expect("split should succeed")
            .into_option()
            .expect("expected Some");
        assert_eq!(yaml, "");
        assert_eq!(body, "");
    }

    #[test]
    fn utf8_bom_is_stripped_before_fence_check() {
        let src = "\u{feff}---\nkey: value\n---\nbody\n";
        let outcome = split(src, fake_path()).expect("split should succeed");
        assert!(matches!(outcome, Split::Some { .. }));
    }

    #[test]
    fn opening_fence_with_trailing_whitespace_is_tolerated() {
        let src = "---   \nkey: value\n---\nbody\n";
        let (yaml, body) = split(src, fake_path())
            .expect("split should succeed")
            .into_option()
            .expect("expected Some");
        assert_eq!(yaml, "key: value\n");
        assert_eq!(body, "body\n");
    }
}
