use super::json_entry::JsonEntry;
use super::support::children;

ast_node!(JsonObject);

impl<'p> JsonObject<'p> {
    pub fn entries(&self) -> impl Iterator<Item = JsonEntry<'p>> + use<'p> {
        children(&self.0)
    }
}
