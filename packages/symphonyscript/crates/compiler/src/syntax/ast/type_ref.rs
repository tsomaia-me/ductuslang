use super::ast_node::AstNode;
use super::name::Name;
use super::support::{name_at, tokens};
use crate::syntax::{NodeView, SyntaxKind, TokenView};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TypeRef<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for TypeRef<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::TypeRef).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> TypeRef<'p> {
    pub fn base(&self) -> Option<TokenView<'p>> {
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
                        | SyntaxKind::BoolKw
                        | SyntaxKind::Ident
                )
            })
    }

    pub fn enum_name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn lengths(&self) -> impl Iterator<Item = TokenView<'p>> + use<'p> {
        let capacity = self.capacity();
        tokens(&self.0, SyntaxKind::Number).filter(move |token| Some(token) != capacity.as_ref())
    }

    pub fn capacity(&self) -> Option<TokenView<'p>> {
        let mut after_range = false;
        for token in self
            .0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
        {
            match token.kind() {
                SyntaxKind::Dot2 => after_range = true,
                SyntaxKind::Number if after_range => return Some(token),
                _ => {}
            }
        }
        None
    }
}
