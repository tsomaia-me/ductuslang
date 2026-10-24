use super::ids::NodeId;
use super::item_ref::ItemRef;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Destination {
    Fixed(ItemRef),
    Select {
        node: NodeId,
        candidates: Vec<ItemRef>,
    },
    Error,
}

impl Destination {
    pub fn candidates(&self) -> Vec<ItemRef> {
        match self {
            Destination::Fixed(item) => vec![*item],
            Destination::Select { candidates, .. } => candidates.clone(),
            Destination::Error => Vec::new(),
        }
    }
}
