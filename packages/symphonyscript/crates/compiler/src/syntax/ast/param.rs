use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::support::{child, name_at};
use super::r#type::Type;
use crate::syntax::{NodeView, SyntaxKind, TokenView};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Param<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Param<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Param).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Param<'p> {
    pub fn marker(&self) -> Option<TokenView<'p>> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| token.kind().is_keyword())
    }

    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn ty(&self) -> Option<Type<'p>> {
        child(&self.0)
    }

    pub fn default_value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
