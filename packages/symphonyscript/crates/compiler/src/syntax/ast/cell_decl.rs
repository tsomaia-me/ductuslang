use super::ast_node::AstNode;
use super::clock::Clock;
use super::expr::Expr;
use super::init::Init;
use super::match_block::MatchBlock;
use super::name::Name;
use super::support::{child, name_at, token};
use super::r#type::Type;
use crate::syntax::{NodeView, SyntaxKind, TokenView};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CellDecl<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for CellDecl<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::CellDecl).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> CellDecl<'p> {
    pub fn word(&self) -> Option<TokenView<'p>> {
        self.0.first_token().filter(|token| {
            matches!(
                token.kind(),
                SyntaxKind::StateKw | SyntaxKind::StreamKw | SyntaxKind::ChannelKw
            )
        })
    }

    pub fn has_capacity(&self) -> bool {
        token(&self.0, SyntaxKind::LBracket).is_some()
    }

    pub fn capacity(&self) -> Option<TokenView<'p>> {
        token(&self.0, SyntaxKind::Number)
    }

    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn init(&self) -> Option<Init<'p>> {
        child(&self.0)
    }

    pub fn ty(&self) -> Option<Type<'p>> {
        child(&self.0)
    }

    pub fn clock(&self) -> Option<Clock<'p>> {
        child(&self.0)
    }

    pub fn has_rule(&self) -> bool {
        token(&self.0, SyntaxKind::Eq).is_some()
    }

    pub fn value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }

    pub fn match_block(&self) -> Option<MatchBlock<'p>> {
        child(&self.0)
    }
}
