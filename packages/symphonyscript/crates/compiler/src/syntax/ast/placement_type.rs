use super::placement_entry::PlacementEntry;
use super::support::children;

ast_node!(PlacementType);

impl<'p> PlacementType<'p> {
    pub fn entries(&self) -> impl Iterator<Item = PlacementEntry<'p>> + use<'p> {
        children(&self.0)
    }
}
