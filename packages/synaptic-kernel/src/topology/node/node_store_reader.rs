use crate::primitives::entry_store_reader::EntryStoreReader;
use crate::primitives::slot::SlotId;
use crate::topology::node::node_reader::NodeReader;

/// Consumer-side doubly-linked list of nodes.
///
/// Provides read-only structural traversal of the node topology.
///
/// # Threading
/// Consumer thread only.
///
/// # Memory Layout
/// Shares the backing MEM and TB regions with `NodeStoreWriter`. See its layout.
///
/// # Constraints
/// - Read-only: structural mutation is strictly prohibited on the reading plane.
/// - Slots are 1-based. 0 indicates an undefined state.
/// - Created exclusively via `NodeStoreWriter::to_reader()`.
#[derive(Clone)]
pub struct NodeStoreReader {
    pub(crate) nodes: EntryStoreReader,
}

impl NodeStoreReader {
    pub(crate) fn bind(es: EntryStoreReader) -> Self {
        NodeStoreReader { nodes: es }
    }

    #[inline]
    pub fn tb_start_offset(&self) -> usize {
        self.nodes.tb_start_offset()
    }

    #[inline]
    pub fn tb_end_offset(&self) -> usize {
        self.nodes.tb_end_offset()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.nodes.capacity()
    }

    #[inline]
    pub fn get_node(&'_ self, slot: SlotId) -> NodeReader<'_> {
        NodeReader::new(self.nodes.get(slot))
    }

    #[inline]
    pub fn ack_generation(&'_ self) {
        self.nodes.ack_generation()
    }
}
