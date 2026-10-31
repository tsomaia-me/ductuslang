use super::map_entry::MapEntry;
use super::support::children;

ast_node!(MapLit);

impl<'p> MapLit<'p> {
    pub fn entries(&self) -> impl Iterator<Item = MapEntry<'p>> + use<'p> {
        children(&self.0)
    }
}
