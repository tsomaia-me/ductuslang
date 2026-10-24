use super::ast_node::AstNode;
use super::block::Block;
use super::name::Name;
use super::pattern_param::PatternParam;
use super::pattern_param_list::PatternParamList;
use super::support::{child, children, name_at};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PatternDecl<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for PatternDecl<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::PatternDecl).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> PatternDecl<'p> {
    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn params(&self) -> impl Iterator<Item = PatternParam<'p>> + use<'p> {
        child::<PatternParamList>(&self.0)
            .into_iter()
            .flat_map(|list| children::<PatternParam>(list.syntax()))
    }

    pub fn body(&self) -> Option<Block<'p>> {
        child(&self.0)
    }
}
