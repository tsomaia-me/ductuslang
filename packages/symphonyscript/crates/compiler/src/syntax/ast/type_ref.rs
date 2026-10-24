use super::ast_node::AstNode;
use super::name::Name;
use super::support::{name_at, tokens};
use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TypeRef(SyntaxNode);

impl AstNode for TypeRef {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::TypeRef).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl TypeRef {
    pub fn base(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::I32Kw
                        | SyntaxKind::I64Kw
                        | SyntaxKind::F32Kw
                        | SyntaxKind::F64Kw
                        | SyntaxKind::BoolKw
                        | SyntaxKind::Ident
                )
            })
    }

    pub fn enum_name(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn lengths(&self) -> impl Iterator<Item = SyntaxToken> + use<> {
        tokens(&self.0, SyntaxKind::Number)
    }
}
