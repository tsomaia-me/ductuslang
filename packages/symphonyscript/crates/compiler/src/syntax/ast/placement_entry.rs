use super::count::Count;
use super::support::child;

ast_node!(PlacementEntry);

impl<'p> PlacementEntry<'p> {
    pub fn bundle(&self) -> Option<PlacementEntry<'p>> {
        child(&self.0)
    }

    pub fn count(&self) -> Option<Count<'p>> {
        child(&self.0)
    }
}
