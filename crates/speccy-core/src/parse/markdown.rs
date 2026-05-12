//! Shared comrak helpers used by the SPEC.md and TASKS.md parsers.

use comrak::Arena;
use comrak::Options;
use comrak::arena_tree::Node;
use comrak::nodes::Ast;
use comrak::nodes::AstNode;
use comrak::nodes::NodeValue;
use comrak::parse_document;
use std::cell::RefCell;

// `Arena` and `AstNode` are lifetime-parameterised type aliases in
// comrak. The `'a` parameter on each function ties the returned AST node
// borrow to the arena's lifetime.

/// Configure comrak options once with the extensions Speccy depends on:
/// GFM tables, task lists, and frontmatter recognition (so source line
/// positions stay accurate when a document begins with `---`).
#[must_use = "the returned options block must be passed to parse_document"]
pub fn speccy_options() -> Options<'static> {
    let mut opts = Options::default();
    opts.extension.table = true;
    opts.extension.tasklist = false; // we parse checkbox glyphs manually
    opts.extension.front_matter_delimiter = Some("---".to_owned());
    opts
}

/// Parse a markdown document into a comrak AST using Speccy's standard
/// option set. The returned root borrows from the arena.
#[must_use = "the parsed AST borrows from the arena"]
pub fn parse_markdown<'a>(arena: &'a Arena<'a>, source: &str) -> &'a AstNode<'a> {
    let options = speccy_options();
    parse_document(arena, source, &options)
}

/// Concatenate the inline text content of a node and its descendants.
///
/// Code spans contribute their literal payload; soft and hard line breaks
/// become single spaces. Block-level descendants (paragraphs, lists, etc.)
/// are walked but only their inline leaves contribute.
#[must_use = "the produced string is the flat inline text"]
pub fn inline_text<'a>(node: &'a Node<'a, RefCell<Ast>>) -> String {
    let mut out = String::new();
    for descendant in node.descendants() {
        let ast = descendant.data.borrow();
        match &ast.value {
            NodeValue::Text(t) => out.push_str(t),
            NodeValue::Code(c) => out.push_str(&c.literal),
            NodeValue::LineBreak | NodeValue::SoftBreak => out.push(' '),
            _ => {}
        }
    }
    out
}

/// Like [`inline_text`], but preserves code spans separately so callers
/// can recover the original backtick-quoted segments. Each returned span
/// is either a [`TextSpan::Plain`] (Text or whitespace) or
/// [`TextSpan::Code`] (the literal between backticks).
#[must_use = "the returned spans are needed to recover code-span content"]
pub fn inline_spans<'a>(node: &'a Node<'a, RefCell<Ast>>) -> Vec<TextSpan> {
    let mut out = Vec::new();
    for descendant in node.descendants() {
        let ast = descendant.data.borrow();
        match &ast.value {
            NodeValue::Text(t) => out.push(TextSpan::Plain(t.clone().into_owned())),
            NodeValue::Code(c) => out.push(TextSpan::Code(c.literal.clone())),
            NodeValue::LineBreak | NodeValue::SoftBreak => {
                out.push(TextSpan::Plain(" ".to_owned()));
            }
            _ => {}
        }
    }
    out
}

/// One span of inline content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSpan {
    /// Plain text (or a single space substituting for a line break).
    Plain(String),
    /// Literal content of a code span between backticks.
    Code(String),
}
