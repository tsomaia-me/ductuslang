use crate::primitives::entry_store_reader::EntryStoreReader;
use crate::primitives::slot::SlotId;
use crate::topology::network::synapse_reader::SynapseReader;
use crate::topology::node::node_reader::NodeReader;
use crate::topology::node::node_store_reader::NodeStoreReader;

/// Consumer-side mirror of the node and synapse topology.
///
/// Provides read-only traversal across both chains. Callers walk the graph
/// starting from the head pointers for desired nodes that they know are alive,
/// follow `get_next_ptr()` through the node store,
/// and dereference `get_outgoing_synapse_head()` / `get_incoming_synapse_head()`
/// on each node to walk its synapse lists.
///
/// # Threading
/// Consumer thread only.
///
/// # Memory Layout
/// Shares the backing MEM and TB regions with `NetworkWriter`. See its layout.
///
/// # Constraints
/// - Read-only: structural mutation is strictly prohibited.
/// - Slots are 1-based. 0 denotes "no slot" / "undefined".
/// - No liveness check on random access: the reader does not carry the slot allocators,
///   so `get_note(slot)` and `get_synapse(slot)` return raw memory for whatever entity last
///   occupied that slot. Consumers MUST reach slots by traversing head pointers and next/prev
///   pointers of already-acquired entries. Random-access arbitrary slot is undefined.
/// - `ack_generation()` acknowledges both node and synapse deferred-deletion generations.
///   Invoked by `EpochMirror::swap()` - consumers do not call it directly.
/// - Created exclusively via `NetworkWriter::to_reader()`.
#[derive(Clone)]
pub struct NetworkReader {
    node_chain: NodeStoreReader,
    pub(crate) synapses: EntryStoreReader,
}

impl NetworkReader {
    pub(crate) fn bind(node_chain: NodeStoreReader, synapses: EntryStoreReader) -> Self {
        NetworkReader {
            node_chain,
            synapses,
        }
    }

    #[inline]
    pub fn mem_end_offset(&self) -> usize {
        self.synapses.mem_end_offset()
    }

    #[inline]
    pub fn tb_start_offset(&self) -> usize {
        self.node_chain.tb_start_offset()
    }

    #[inline]
    pub fn tb_end_offset(&self) -> usize {
        self.synapses.tb_end_offset()
    }

    #[inline]
    pub fn node_capacity(&self) -> usize {
        self.node_chain.capacity()
    }

    #[inline]
    pub fn synapse_capacity(&self) -> usize {
        self.synapses.capacity()
    }

    #[inline]
    pub fn get_node(&'_ self, slot: SlotId) -> NodeReader<'_> {
        self.node_chain.get_node(slot)
    }

    #[inline]
    pub fn get_synapse(&'_ self, slot: SlotId) -> SynapseReader<'_> {
        SynapseReader::new(self.synapses.get(slot))
    }

    #[inline]
    pub fn ack_generation(&'_ self) {
        self.node_chain.ack_generation();
        self.synapses.ack_generation();
    }
}
