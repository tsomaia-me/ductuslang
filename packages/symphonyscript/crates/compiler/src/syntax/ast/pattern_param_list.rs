use super::ast_node::AstNode;
use super::pattern_param::PatternParam;
use super::support::children;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PatternParamList<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for PatternParamList<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::PatternParamList).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> PatternParamList<'p> {
    pub fn params(&self) -> impl Iterator<Item = PatternParam<'p>> + use<'p> {
        children(&self.0)
    }
}
