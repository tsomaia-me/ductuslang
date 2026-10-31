use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::placement_type::PlacementType;
use super::support::{child, name_at, token};
use super::r#type::Type;
use crate::syntax::{NodeView, SyntaxKind, TokenView};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PatternParam<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for PatternParam<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::PatternParam).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> PatternParam<'p> {
    pub fn is_default(&self) -> bool {
        token(&self.0, SyntaxKind::DefaultKw).is_some()
    }

    pub fn is_children(&self) -> bool {
        token(&self.0, SyntaxKind::ChildrenKw).is_some()
    }

    pub fn markers(&self) -> impl Iterator<Item = TokenView<'p>> + use<'p> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| {
                token.kind().is_keyword()
                    && !matches!(token.kind(), SyntaxKind::DefaultKw | SyntaxKind::ChildrenKw)
            })
    }

    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn ty(&self) -> Option<Type<'p>> {
        child(&self.0)
    }

    pub fn placement_type(&self) -> Option<PlacementType<'p>> {
        child(&self.0)
    }

    pub fn default_value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
