use super::ast_node::AstNode;
use super::fn_type::FnType;
use super::sum_type::SumType;
use super::type_ref::TypeRef;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Type<'p> {
    Ref(TypeRef<'p>),
    Sum(SumType<'p>),
    Fn(FnType<'p>),
}

impl<'p> AstNode<'p> for Type<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        match node.kind() {
            SyntaxKind::TypeRef => TypeRef::cast(node).map(Type::Ref),
            SyntaxKind::SumType => SumType::cast(node).map(Type::Sum),
            SyntaxKind::FnType => FnType::cast(node).map(Type::Fn),
            _ => None,
        }
    }

    fn syntax(&self) -> &NodeView<'p> {
        match self {
            Type::Ref(node) => node.syntax(),
            Type::Sum(node) => node.syntax(),
            Type::Fn(node) => node.syntax(),
        }
    }
}
