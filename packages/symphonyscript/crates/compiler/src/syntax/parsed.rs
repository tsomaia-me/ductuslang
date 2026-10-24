use std::fmt;
use std::mem;

use rowan::{GreenNode, NodeOrToken};

use super::ast::{AstNode, File};
use super::node_view::NodeView;
use super::syntax_kind::SyntaxKind;
use super::syntax_node::SyntaxNode;
use crate::diagnostics::Diagnostic;

pub struct Parsed {
    green: GreenNode,
    pub diagnostics: Vec<Diagnostic>,
}

impl Parsed {
    pub(crate) fn new(green: GreenNode, diagnostics: Vec<Diagnostic>) -> Self {
        Self { green, diagnostics }
    }

    pub fn root(&self) -> NodeView<'_> {
        NodeView::new(SyntaxNode::new_root(self.green.clone()))
    }

    pub fn file(&self) -> Option<File<'_>> {
        File::cast(self.root())
    }
}

impl Clone for Parsed {
    fn clone(&self) -> Self {
        Self {
            green: self.green.clone(),
            diagnostics: self.diagnostics.clone(),
        }
    }
}

impl PartialEq for Parsed {
    fn eq(&self, other: &Self) -> bool {
        if self.diagnostics != other.diagnostics {
            return false;
        }
        let mut pending = vec![(self.green.clone(), other.green.clone())];
        while let Some((left, right)) = pending.pop() {
            if left.kind() != right.kind() || left.children().len() != right.children().len() {
                return false;
            }
            for pair in left.children().zip(right.children()) {
                match pair {
                    (NodeOrToken::Node(a), NodeOrToken::Node(b)) => {
                        pending.push((a.to_owned(), b.to_owned()));
                    }
                    (NodeOrToken::Token(a), NodeOrToken::Token(b))
                        if a.kind() == b.kind() && a.text() == b.text() => {}
                    _ => return false,
                }
            }
        }
        true
    }
}

impl Eq for Parsed {}

impl fmt::Debug for Parsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Parsed")
            .field("kind", &self.green.kind())
            .field("text_len", &self.green.text_len())
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

impl Drop for Parsed {
    fn drop(&mut self) {
        let placeholder = GreenNode::new(rowan::SyntaxKind(SyntaxKind::File as u16), []);
        let mut kept = vec![mem::replace(&mut self.green, placeholder)];
        let mut index = 0;
        while index < kept.len() {
            let children: Vec<GreenNode> = kept[index]
                .children()
                .filter_map(NodeOrToken::into_node)
                .map(ToOwned::to_owned)
                .collect();
            kept.extend(children);
            index += 1;
        }
    }
}
