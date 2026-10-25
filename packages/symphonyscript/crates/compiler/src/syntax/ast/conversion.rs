use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{NodeView, SyntaxKind, TokenView};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Conversion<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Conversion<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Conversion).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Conversion<'p> {
    pub fn target(&self) -> Option<TokenView<'p>> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::U8Kw
                        | SyntaxKind::I32Kw
                        | SyntaxKind::I64Kw
                        | SyntaxKind::F32Kw
                        | SyntaxKind::F64Kw
                )
            })
    }

    pub fn operand(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
