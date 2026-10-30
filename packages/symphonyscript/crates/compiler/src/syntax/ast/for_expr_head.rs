use super::ast_node::AstNode;
use super::expr::Expr;
use super::loop_head::LoopHead;
use super::name::Name;
use super::support::expr_after;
use super::type_ref::TypeRef;
use crate::syntax::{ElementView, NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ForExprHead<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for ForExprHead<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::ForExprHead).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> ForExprHead<'p> {
    pub fn loop_head(&self) -> LoopHead<'p> {
        LoopHead::of(&self.0)
    }

    pub fn accumulator(&self) -> Option<Name<'p>> {
        self.after_into()
            .find_map(ElementView::into_token)
            .filter(|token| token.kind() == SyntaxKind::Ident)
            .map(Name::new)
    }

    pub fn accumulator_type(&self) -> Option<TypeRef<'p>> {
        self.after_into()
            .take_while(|element| element.kind() != SyntaxKind::FromKw)
            .filter_map(ElementView::into_node)
            .find_map(TypeRef::cast)
    }

    pub fn start(&self) -> Option<Expr<'p>> {
        expr_after(&self.0, |kind| kind == SyntaxKind::FromKw, |_| false)
    }

    fn after_into(&self) -> impl Iterator<Item = ElementView<'p>> + use<'p> {
        self.0
            .children_with_tokens()
            .skip_while(|element| element.kind() != SyntaxKind::IntoKw)
            .skip(1)
            .filter(|element| !element.kind().is_trivia())
    }
}
