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

/// First child of `item` whose AST value is a `Paragraph`, or `None`
/// if none. Used by markdown walkers that need the textual lede of a
/// list item or heading.
#[must_use = "the returned reference is the paragraph node to walk"]
pub fn first_paragraph_child<'a>(item: &'a AstNode<'a>) -> Option<&'a AstNode<'a>> {
    item.children().find(|c| {
        let ast = c.data.borrow();
        matches!(ast.value, NodeValue::Paragraph)
    })
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
