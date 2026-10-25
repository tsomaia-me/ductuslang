use super::ast_node::AstNode;
use super::bound::Bound;
use super::expr::Expr;
use super::support::{expr_before, token};
use crate::syntax::{ElementView, NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct IndexExpr<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for IndexExpr<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::IndexExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> IndexExpr<'p> {
    pub fn base(&self) -> Option<Expr<'p>> {
        expr_before(&self.0, |kind| kind == SyntaxKind::LBracket)
    }

    pub fn is_slice(&self) -> bool {
        token(&self.0, SyntaxKind::Dot2).is_some()
    }

    pub fn index(&self) -> Option<Bound<'p>> {
        if self.is_slice() {
            return None;
        }
        self.bound(|_| true)
    }

    pub fn lower(&self) -> Option<Bound<'p>> {
        self.bound(|before_range| before_range)
    }

    pub fn upper(&self) -> Option<Bound<'p>> {
        self.bound(|before_range| !before_range)
    }

    fn bound(&self, wanted: impl Fn(bool) -> bool) -> Option<Bound<'p>> {
        let mut before_range = true;
        for element in self.0.children_with_tokens() {
            match element {
                ElementView::Token(token) if token.kind() == SyntaxKind::Dot2 => {
                    before_range = false;
                }
                ElementView::Node(node) => {
                    if let Some(bound) = Bound::cast(node)
                        && wanted(before_range)
                    {
                        return Some(bound);
                    }
                }
                ElementView::Token(_) => {}
            }
        }
        None
    }
}
