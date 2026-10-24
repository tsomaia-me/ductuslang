use super::ast_node::AstNode;
use super::expr::Expr;
use super::named_arg::NamedArg;
use super::support::children;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ArgList(SyntaxNode);

impl AstNode for ArgList {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::ArgList).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ArgList {
    pub fn positional(&self) -> impl Iterator<Item = Expr> + use<> {
        children(&self.0)
    }

    pub fn named(&self) -> impl Iterator<Item = NamedArg> + use<> {
        children(&self.0)
    }
}
