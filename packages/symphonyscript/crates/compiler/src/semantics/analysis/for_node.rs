use super::for_iterable::ForIterable;
use super::ids::{NodeId, SlotId};

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct ForNode {
    pub iterable: ForIterable,
    pub start: NodeId,
    pub body: NodeId,
    pub index: Option<SlotId>,
    pub element: Option<SlotId>,
    pub accumulator: SlotId,
}

impl ForNode {
    pub fn operands(&self) -> Vec<NodeId> {
        let mut operands = self.iterable.nodes();
        operands.push(self.start);
        operands.push(self.body);
        operands
    }
}
