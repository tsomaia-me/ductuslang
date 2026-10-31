use super::ast_node::AstNode;
use super::expr::Expr;
use super::json_array::JsonArray;
use super::json_atom::JsonAtom;
use super::json_object::JsonObject;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum JsonValue<'p> {
    Object(JsonObject<'p>),
    Array(JsonArray<'p>),
    Atom(JsonAtom<'p>),
    Expr(Expr<'p>),
}

impl<'p> AstNode<'p> for JsonValue<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        match node.kind() {
            SyntaxKind::JsonObject => JsonObject::cast(node).map(JsonValue::Object),
            SyntaxKind::JsonArray => JsonArray::cast(node).map(JsonValue::Array),
            SyntaxKind::JsonAtom => JsonAtom::cast(node).map(JsonValue::Atom),
            _ => Expr::cast(node).map(JsonValue::Expr),
        }
    }

    fn syntax(&self) -> &NodeView<'p> {
        match self {
            JsonValue::Object(node) => node.syntax(),
            JsonValue::Array(node) => node.syntax(),
            JsonValue::Atom(node) => node.syntax(),
            JsonValue::Expr(node) => node.syntax(),
        }
    }
}
