use super::json_value::JsonValue;
use super::support::child;

ast_node!(JsonLit);

impl<'p> JsonLit<'p> {
    pub fn value(&self) -> Option<JsonValue<'p>> {
        child(&self.0)
    }
}
