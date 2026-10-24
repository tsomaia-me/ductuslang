use rowan::GreenNode;

use super::ast::{AstNode, File};
use super::syntax_node::SyntaxNode;
use crate::diagnostics::Diagnostic;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Parsed {
    pub green: GreenNode,
    pub diagnostics: Vec<Diagnostic>,
}

impl Parsed {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    pub fn file(&self) -> Option<File> {
        File::cast(self.syntax())
    }
}
