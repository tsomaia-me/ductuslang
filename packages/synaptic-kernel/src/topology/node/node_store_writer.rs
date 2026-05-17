use crate::errors::slot_allocator_error::SlotAllocatorError;
use crate::primitives::entry_store_writer::EntryStoreWriter;
use crate::primitives::slot::SlotId;
use crate::primitives::triple_buffer_writer::TripleBufferWriter;
use crate::primitives::types::AtomicBuffer;
use crate::topology::node::node_handle::NodeHandle;
use crate::topology::node::node_store_config::NodeStoreConfig;
use crate::topology::node::node_store_reader::NodeStoreReader;
use crate::topology::node::node_writer::NodeWriter;

/// Producer-side doubly-linked list of nodes.
///
/// Owns an internal `EntryStoreWriter<...>` for slot allocation and per-slot storage
/// (core + meta zones in TB, attributes on the MEM).
///
/// # Threading
/// Producer thread only.
///
/// # Memory Layout (MEM plane)
/// ```text
/// Order       Segment             Size
/// -------------------------------------
/// 1           Slot Allocator      SlotAllocator::calculate_size_on_mem()
/// 2           Node Attributes     capacity * ATTR_STRIDE
/// ```
///
/// # Memory Layout (TB plane)
/// ```text
/// Offset      Size            Field
/// -------------------------------------
/// 0           C * (N + M)     nodes (core + meta pers slot)
///
/// C = capacity
/// N = NODE_STRIDE
/// M = META_STRIDE
/// ```
///
/// Each node's core and meta zones are adjacent per slot - see `NodeWriter`
/// for the exact core field layout.
///
/// # Scope
/// This type manages only the node store. `remove_node()` unlinks the node
/// from the sub-chain and defers its slot removal, but does NOT touch any synapses
/// that reference the removed node. If the graph has active synapses,
/// use `NetworkWriter::remove_node()` instead - it cascades synapse cleanup before
/// invoking this.
///
/// # Constraints
/// - Slots are 1-based. 0 denotes "no slot" / "undefined".
/// - Lifecycle safety: `remove_node()` marks the slot for deferred freeing, preventing
///   reallocation until the consumer has advanced past the pending `publish()`.
/// - Use `to_reader()` to create the paired `NodeStoreReader`.
#[derive(Clone)]
pub struct NodeStoreWriter {
    pub(crate) nodes: EntryStoreWriter,
}

impl NodeStoreWriter {
    pub fn new(
        mem: AtomicBuffer,
        tb: TripleBufferWriter,
        config: NodeStoreConfig,
        mem_start_offset: usize,
        tb_start_offset: usize,
    ) -> Self {
        Self::create(mem, tb, config, mem_start_offset, tb_start_offset, false)
    }

    pub fn bind(
        mem: AtomicBuffer,
        tb: TripleBufferWriter,
        config: NodeStoreConfig,
        mem_start_offset: usize,
        tb_start_offset: usize,
    ) -> Self {
        Self::create(mem, tb, config, mem_start_offset, tb_start_offset, true)
    }

    pub fn create(
        mem: AtomicBuffer,
        tb: TripleBufferWriter,
        config: NodeStoreConfig,
        mem_start_offset: usize,
        tb_start_offset: usize,
        bind: bool,
    ) -> Self {
        NodeStoreWriter {
            nodes: EntryStoreWriter::create(
                mem,
                tb,
                config.to_entry_store_config(),
                mem_start_offset,
                tb_start_offset,
                bind,
            ),
        }
    }

    #[inline]
    pub fn calculate_size_on_mem(config: &NodeStoreConfig) -> usize {
        EntryStoreWriter::calculate_size_on_mem(&config.to_entry_store_config())
    }

    #[inline]
    pub fn calculate_size_on_tb(config: &NodeStoreConfig) -> usize {
        EntryStoreWriter::calculate_size_on_tb(&config.to_entry_store_config())
    }

    pub fn to_reader(&self) -> NodeStoreReader {
        NodeStoreReader::bind(self.nodes.to_reader())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    pub fn mem_start_offset(&self) -> usize {
        self.nodes.mem_start_offset()
    }

    #[inline]
    pub fn mem_end_offset(&self) -> usize {
        self.nodes.mem_end_offset()
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
    pub fn utilization(&self) -> f32 {
        self.nodes.utilization()
    }

    #[inline]
    pub fn get_node(&'_ self, slot: SlotId) -> NodeWriter<'_> {
        NodeWriter::new(self.nodes.get(slot))
    }

    #[inline]
    pub fn get_node_handle(&'_ self, slot: SlotId) -> NodeHandle<'_> {
        NodeHandle::new(self.nodes.get_handle(slot))
    }

    pub fn insert_node(&self, kind: i32) -> Option<SlotId> {
        self.insert_orphaned_node(kind, None, None)
    }

    pub fn insert_node_after(&self, prev_slot: SlotId, kind: i32) -> Option<SlotId> {
        let prev = self.get_node(prev_slot);
        let prev_next_slot = prev.get_next_ptr();
        let result = self.insert_orphaned_node(kind, prev_next_slot, Some(prev_slot));

        if result.is_none() {
            return None;
        }

        let new_slot = result.unwrap();

        prev.set_next_ptr(Some(new_slot));

        if prev_next_slot.is_some() {
            self.get_node(prev_next_slot.unwrap())
                .set_prev_ptr(Some(new_slot));
        }

        Some(new_slot)
    }

    pub fn insert_node_before(&self, next_slot: SlotId, kind: i32) -> Option<SlotId> {
        let next = self.get_node(next_slot);
        let next_prev_slot = next.get_prev_ptr();
        let result = self.insert_orphaned_node(kind, Some(next_slot), next_prev_slot);

        if result.is_none() {
            return None;
        }

        let new_slot = result.unwrap();

        next.set_prev_ptr(Some(new_slot));

        if next_prev_slot.is_some() {
            self.get_node(next_prev_slot.unwrap())
                .set_next_ptr(Some(new_slot));
        }

        Some(new_slot)
    }

    pub fn remove_node(&self, slot: SlotId) -> Result<(), SlotAllocatorError> {
        let node = self.get_node(slot);
        let prev_slot = node.get_prev_ptr();
        let next_slot = node.get_next_ptr();

        self.nodes.remove(slot)?;

        if prev_slot.is_some() {
            self.get_node(prev_slot.unwrap()).set_next_ptr(next_slot);
        }

        if next_slot.is_some() {
            self.get_node(next_slot.unwrap()).set_prev_ptr(prev_slot);
        }

        Ok(())
    }

    pub fn publish(&self) {
        self.nodes.publish()
    }

    pub fn copy_from(&self, source: &Self) {
        debug_assert!(
            source.capacity() <= self.capacity(),
            "NodeStoreWriter.copy_from | source.capacity {} cannot be greater than destination.capacity {}",
            source.capacity(),
            self.capacity(),
        );

        self.nodes.copy_from(&source.nodes);
    }

    fn insert_orphaned_node(
        &self,
        kind: i32,
        next_ptr: Option<SlotId>,
        prev_ptr: Option<SlotId>,
    ) -> Option<SlotId> {
        let result = self.nodes.insert();

        if result.is_none() {
            return None;
        }

        let new_slot = result.unwrap();
        let node = self.get_node(new_slot);

        node.set_kind(kind);
        node.set_next_ptr(next_ptr);
        node.set_prev_ptr(prev_ptr);
        node.set_outgoing_synapse_head(None);
        node.set_outgoing_synapse_tail(None);
        node.set_incoming_synapse_head(None);
        node.set_incoming_synapse_tail(None);

        Some(new_slot)
    }
}
