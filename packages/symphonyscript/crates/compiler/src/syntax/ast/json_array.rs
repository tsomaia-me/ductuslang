use super::json_value::JsonValue;
use super::support::children;

ast_node!(JsonArray);

impl<'p> JsonArray<'p> {
    pub fn values(&self) -> impl Iterator<Item = JsonValue<'p>> + use<'p> {
        children(&self.0)
    }
}
