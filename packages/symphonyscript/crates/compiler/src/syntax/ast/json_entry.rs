use super::json_value::JsonValue;
use super::support::{child, token};
use crate::syntax::{SyntaxKind, TokenView};

ast_node!(JsonEntry);

impl<'p> JsonEntry<'p> {
    pub fn key(&self) -> Option<TokenView<'p>> {
        token(&self.0, SyntaxKind::JsonString)
    }

    pub fn value(&self) -> Option<JsonValue<'p>> {
        child(&self.0)
    }
}
