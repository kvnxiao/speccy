//! QST-001 rule: surface unchecked open questions in SPEC.md.

use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;
use crate::parse::markdown::inline_text;
use crate::parse::markdown::parse_markdown;
use comrak::Arena;
use comrak::arena_tree::Node;
use comrak::nodes::Ast;
use comrak::nodes::AstNode;
use comrak::nodes::NodeValue;
use std::cell::RefCell;

const QST_001: &str = "QST-001";

/// Append one QST-001 diagnostic per unchecked `- [ ] question?` line
/// inside any `## Open questions` section (case-insensitive heading
/// match) of the spec's SPEC.md.
pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    let Some(spec_md) = spec.spec_md_ok() else {
        return;
    };

    let arena = Arena::new();
    let root = parse_markdown(&arena, &spec_md.raw);

    let mut in_open_questions = false;
    for node in root.children() {
        let ast = node.data.borrow();
        match &ast.value {
            NodeValue::Heading(h) if h.level == 2 => {
                let text = inline_text(node);
                in_open_questions = text.trim().eq_ignore_ascii_case("Open questions");
            }
            NodeValue::List(_) if in_open_questions => {
                drop(ast);
                collect_unchecked_items(node, spec, out);
            }
            _ => {}
        }
    }
}

fn collect_unchecked_items<'a>(
    list: &'a Node<'a, RefCell<Ast>>,
    spec: &ParsedSpec,
    out: &mut Vec<Diagnostic>,
) {
    for item in list.children() {
        let item_ast = item.data.borrow();
        if !matches!(item_ast.value, NodeValue::Item(_)) {
            continue;
        }
        let line = item_ast.sourcepos.start.line;
        drop(item_ast);

        let Some(paragraph) = first_paragraph(item) else {
            continue;
        };
        let text = inline_text(paragraph);
        let trimmed = text.trim_start();
        let Some(question) = trimmed.strip_prefix("[ ]") else {
            continue;
        };
        let question = question.trim();
        if question.is_empty() {
            continue;
        }
        out.push(Diagnostic::with_location(
            QST_001,
            Level::Info,
            spec.spec_id.clone(),
            spec.spec_md_path.clone(),
            u32::try_from(line).unwrap_or(0),
            format!("unchecked open question: {question}"),
        ));
    }
}

fn first_paragraph<'a>(item: &'a AstNode<'a>) -> Option<&'a AstNode<'a>> {
    item.children().find(|c| {
        let ast = c.data.borrow();
        matches!(ast.value, NodeValue::Paragraph)
    })
}
