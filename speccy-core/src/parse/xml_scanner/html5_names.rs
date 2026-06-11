//! Checked-in copy of the HTML5 element name set.
//!
//! Used by the Speccy element whitelist disjointness invariant: every
//! recognised Speccy structure element name (`spec`, `overview`,
//! `requirement`, `scenario`, `decision`, `open-question`, `changelog`)
//! must be absent from this set. The constant lives in code (not just in
//! docs) so the unit test in `super` can prove the invariant at build
//! time — accidental future collisions surface as a test failure rather
//! than as silent ambiguity in agent-facing prompts.
//!
//! The list is the WHATWG HTML Living Standard element index, sorted
//! alphabetically. Element names use the lowercase form HTML5 mandates.

/// Every element name reserved by the HTML5 (WHATWG HTML Living Standard)
/// element index.
///
/// Sorted alphabetically so future edits can stay diff-stable; the order
/// is otherwise irrelevant to the disjointness invariant.
pub const HTML5_ELEMENT_NAMES: &[&str] = &[
    "a",
    "abbr",
    "address",
    "area",
    "article",
    "aside",
    "audio",
    "b",
    "base",
    "bdi",
    "bdo",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "cite",
    "code",
    "col",
    "colgroup",
    "data",
    "datalist",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "map",
    "mark",
    "math",
    "menu",
    "meta",
    "meter",
    "nav",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "param",
    "picture",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "script",
    "search",
    "section",
    "select",
    "slot",
    "small",
    "source",
    "span",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "svg",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "title",
    "tr",
    "track",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

/// Return true when `name` is in the HTML5 element name set.
#[must_use = "the caller uses this to enforce the disjointness invariant"]
pub fn is_html5_element_name(name: &str) -> bool {
    HTML5_ELEMENT_NAMES.contains(&name)
}

/// The HTML5 void elements — elements that have no end tag by definition
/// (WHATWG HTML Living Standard §13.1.2 "void elements").
///
/// A foreign open tag whose name is in this set is never a dangling open:
/// it cannot have a matching close, so balance checks must exempt it. The
/// set lives in code (not just in docs) so the unit test in `super` can
/// prove it is a real subset of [`HTML5_ELEMENT_NAMES`] at build time
/// rather than re-asserting a hand-copied literal.
///
/// Sorted alphabetically so future edits stay diff-stable.
pub const VOID_ELEMENT_NAMES: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Return true when `name` is an HTML5 void element name.
#[must_use = "the caller uses this to exempt void elements from balance checks"]
pub fn is_void_element_name(name: &str) -> bool {
    VOID_ELEMENT_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The void set is a genuine subset of the HTML5 element index: a
    /// "void" name that is not even a recognised HTML5 element would be a
    /// typo or an invented name. This proves the structural relationship
    /// between the two constants rather than re-asserting a hand-copied
    /// literal, so it fails if either list drifts out from under the other.
    #[test]
    fn void_set_is_subset_of_html5_element_set() {
        for name in VOID_ELEMENT_NAMES {
            assert!(
                is_html5_element_name(name),
                "void element `{name}` is not in HTML5_ELEMENT_NAMES",
            );
        }
    }
}
