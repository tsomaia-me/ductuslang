use super::ast_node::AstNode;
use super::block::Block;
use super::name::Name;
use super::pattern_param::PatternParam;
use super::pattern_param_list::PatternParamList;
use super::support::{child, children, name_at};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PatternDecl(SyntaxNode);

impl AstNode for PatternDecl {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::PatternDecl).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PatternDecl {
    pub fn name(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn params(&self) -> impl Iterator<Item = PatternParam> + use<> {
        child::<PatternParamList>(&self.0)
            .into_iter()
            .flat_map(|list| children::<PatternParam>(list.syntax()))
    }

    pub fn body(&self) -> Option<Block> {
        child(&self.0)
    }
}
