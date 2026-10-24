use super::ast_node::AstNode;
use super::expr::Expr;
use super::named_arg::NamedArg;
use super::support::children;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ArgList<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for ArgList<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::ArgList).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> ArgList<'p> {
    pub fn positional(&self) -> impl Iterator<Item = Expr<'p>> + use<'p> {
        children(&self.0)
    }

    pub fn named(&self) -> impl Iterator<Item = NamedArg<'p>> + use<'p> {
        children(&self.0)
    }
}
