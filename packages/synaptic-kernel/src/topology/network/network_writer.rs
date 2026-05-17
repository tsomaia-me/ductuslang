use crate::constants::SYNAPSE_STRIDE;
use crate::errors::slot_allocator_error::SlotAllocatorError;
use crate::primitives::entry_store_writer::EntryStoreWriter;
use crate::primitives::slot::SlotId;
use crate::primitives::triple_buffer_writer::TripleBufferWriter;
use crate::primitives::types::AtomicBuffer;
use crate::topology::network::network_config::NetworkConfig;
use crate::topology::network::network_reader::NetworkReader;
use crate::topology::network::synapse_handle::SynapseView;
use crate::topology::network::synapse_writer::SynapseWriter;
use crate::topology::node::node_handle::NodeHandle;
use crate::topology::node::node_store_writer::NodeStoreWriter;
use crate::topology::node::node_writer::NodeWriter;
use std::sync::Arc;

/// Producer-side orchestrator for node and synapse topology.
///
/// Owns two entity stores: a node store of doubly-linked sub-chains of nodes
/// and a flat synapse store. Synapse lifecycle is threaded through node state -
/// every active synapse participates in two concurrent doubly-linked lists:
/// one through its source node's `outgoing` slots, another through its target
/// node's `incoming` slots.
///
/// Node removal cascades to synapses: `remove_node()` first disconnects every
/// outgoing and incoming synapse of the target node, then frees the node's
/// slot. This invariant lives here - not in the node store - because only
/// the combined power of nodes and synapses can enforce it.
///
/// # Threading
/// Producer thread only.
///
/// # Memory Layout (MEM Plane)
/// ```text
/// Order       Segment         Size
/// -------------------------------------
/// 1           Node Store      NodeStoreWriter::<...>::calculate_size_on_mem()
/// 2           Synapse Store   EntryStoreWriter::<...>::calculate_size_on_mem()
/// ```
///
/// # Memory Layout (Triple Buffer Plane)
/// ```text
/// Order       Segment         Size
/// -------------------------------------
/// 1           Node Store      NodeStoreWriter::<...>::calculate_size_on_mem()
/// 2           Synapse Store   EntryStoreWriter::<...>::calculate_size_on_mem()
/// ```
///
/// Synapses have no global head on the TB plane. A synapse is always reached
/// by traversing a node's `outgoing_synapse_head` or `incoming_synapse_head` and
/// following the per-node doubly-linked lists.
///
/// # Deployment
/// 1. Structural edits (node insertion/removal, connect/disconnect) stage changes
///    on the triple-buffer's current writer buffer and on the mem-plane slot allocators.
///    Such changes must be `publish()`-ed.
/// 2. Attribute writes (node and synapse) go directly to the mem plane. The consumer
///    sees them immediately, without a publishing requirement.
/// 3. `publish()` publishes both node and synapse allocator state; the
///    surrounding `Epoch` publishes the triple buffer.
///
/// # Constraints
/// - Slots are 1-based. 0 denotes "no slot" / "undefined".
/// - Built-in lifecycle safety: `remove_node()`, `disconnect()` and `disconnect_synapse()` marks
///   their slots for deferred freeing, preventing reallocation until the consumer has
///   advanced pas the pending `publish()`.
/// - Use `to_reader()` to create the paired `NetworkReader`.
#[derive(Clone)]
pub struct NetworkWriter {
    node_chain: NodeStoreWriter,
    pub(crate) synapses: EntryStoreWriter,
}

impl NetworkWriter {
    pub fn new(
        mem: AtomicBuffer,
        tb: TripleBufferWriter,
        config: NetworkConfig,
        mem_start_offset: usize,
        tb_start_offset: usize,
    ) -> Self {
        Self::create(mem, tb, config, mem_start_offset, tb_start_offset, false)
    }

    pub fn bind(
        mem: AtomicBuffer,
        tb: TripleBufferWriter,
        config: NetworkConfig,
        mem_start_offset: usize,
        tb_start_offset: usize,
    ) -> Self {
        Self::create(mem, tb, config, mem_start_offset, tb_start_offset, true)
    }

    pub fn create(
        mem: AtomicBuffer,
        tb: TripleBufferWriter,
        config: NetworkConfig,
        mem_start_offset: usize,
        tb_start_offset: usize,
        bind: bool,
    ) -> Self {
        let node_chain = NodeStoreWriter::create(
            Arc::clone(&mem),
            tb.clone(),
            config.to_node_store_config(),
            mem_start_offset,
            tb_start_offset,
            bind,
        );
        let synapses = EntryStoreWriter::create(
            mem,
            tb,
            config.to_synapse_entry_store_config(),
            node_chain.mem_end_offset(),
            node_chain.tb_end_offset(),
            bind,
        );

        NetworkWriter {
            node_chain,
            synapses,
        }
    }

    pub fn calculate_size_on_mem(config: &NetworkConfig) -> usize {
        NodeStoreWriter::calculate_size_on_mem(&config.to_node_store_config())
            + EntryStoreWriter::calculate_size_on_mem(&config.to_synapse_entry_store_config())
    }

    pub fn calculate_size_on_tb(config: &NetworkConfig) -> usize {
        NodeStoreWriter::calculate_size_on_tb(&config.to_node_store_config())
            + config.synapse_capacity as usize * (SYNAPSE_STRIDE + config.synapse_meta_stride)
    }

    pub fn to_reader(&self) -> NetworkReader {
        NetworkReader::bind(self.node_chain.to_reader(), self.synapses.to_reader())
    }

    pub fn mem_start_offset(&self) -> usize {
        self.node_chain.mem_start_offset()
    }

    pub fn mem_end_offset(&self) -> usize {
        self.synapses.mem_end_offset()
    }

    pub fn tb_start_offset(&self) -> usize {
        self.node_chain.tb_start_offset()
    }

    pub fn tb_end_offset(&self) -> usize {
        self.synapses.tb_end_offset()
    }

    pub fn node_capacity(&self) -> usize {
        self.node_chain.capacity()
    }

    pub fn node_count(&self) -> usize {
        self.node_chain.len()
    }

    pub fn node_utilization(&self) -> f32 {
        self.node_chain.utilization()
    }

    pub fn synapse_capacity(&self) -> usize {
        self.synapses.capacity()
    }

    pub fn synapse_count(&self) -> usize {
        self.synapses.len()
    }

    pub fn synapse_utilization(&self) -> f32 {
        self.synapses.utilization()
    }

    pub fn peek_utilization(&self) -> f32 {
        self.node_utilization().max(self.synapse_utilization())
    }

    #[inline]
    pub fn get_node(&'_ self, slot: SlotId) -> NodeWriter<'_> {
        self.node_chain.get_node(slot)
    }

    pub fn get_synapse(&'_ self, slot: SlotId) -> SynapseWriter<'_> {
        SynapseWriter::new(self.synapses.get(slot))
    }

    #[inline]
    pub fn get_node_handle(&'_ self, slot: SlotId) -> NodeHandle<'_> {
        self.node_chain.get_node_handle(slot)
    }

    pub fn get_synapse_handle(&'_ self, slot: SlotId) -> SynapseView<'_> {
        SynapseView::new(self.synapses.get_handle(slot))
    }

    pub fn insert_node(&self, kind: i32) -> Option<SlotId> {
        self.node_chain.insert_node(kind)
    }

    pub fn insert_node_after(&self, prev_slot: SlotId, kind: i32) -> Option<SlotId> {
        self.node_chain.insert_node_after(prev_slot, kind)
    }

    pub fn insert_node_before(&self, next_slot: SlotId, kind: i32) -> Option<SlotId> {
        self.node_chain.insert_node_before(next_slot, kind)
    }

    pub fn remove_node(&self, slot: SlotId) -> Result<(), SlotAllocatorError> {
        loop {
            let head = self.node_chain.get_node(slot).get_outgoing_synapse_head();

            if head.is_none() {
                break;
            }

            self.disconnect_synapse(head.unwrap())?;
        }

        loop {
            let head = self.node_chain.get_node(slot).get_incoming_synapse_head();

            if head.is_none() {
                break;
            }

            self.disconnect_synapse(head.unwrap())?;
        }

        self.node_chain.remove_node(slot)
    }

    pub fn remove_chain(&self, head_slot: SlotId) -> Result<(), SlotAllocatorError> {
        let mut current_slot = Some(head_slot);

        while current_slot.is_some() {
            let next_slot = self
                .node_chain
                .get_node(current_slot.unwrap())
                .get_next_ptr();
            self.remove_node(current_slot.unwrap())?;
            current_slot = next_slot;
        }

        Ok(())
    }

    pub fn connect(&self, source_slot: SlotId, target_slot: SlotId, kind: i32) -> Option<SlotId> {
        let source = self.node_chain.get_node(source_slot);
        let target = self.node_chain.get_node(target_slot);
        let source_current_tail_ptr = source.get_outgoing_synapse_tail();
        let target_current_tail_ptr = target.get_incoming_synapse_tail();
        let result = self.synapses.insert();

        if result.is_none() {
            return None;
        }

        let new_slot = result.unwrap();
        let synapse = self.get_synapse(new_slot);

        synapse.set_kind(kind);
        synapse.set_source_ptr(source_slot);
        synapse.set_target_ptr(target_slot);
        synapse.set_outgoing_next_ptr(None);
        synapse.set_outgoing_prev_ptr(source_current_tail_ptr);
        synapse.set_incoming_next_ptr(None);
        synapse.set_incoming_prev_ptr(target_current_tail_ptr);

        if source.get_outgoing_synapse_head().is_none() {
            source.set_outgoing_synapse_head(Some(new_slot));
        }

        if target.get_incoming_synapse_head().is_none() {
            target.set_incoming_synapse_head(Some(new_slot));
        }

        if source_current_tail_ptr.is_some() {
            self.get_synapse(source_current_tail_ptr.unwrap())
                .set_outgoing_next_ptr(Some(new_slot));
        }

        if target_current_tail_ptr.is_some() {
            self.get_synapse(target_current_tail_ptr.unwrap())
                .set_incoming_next_ptr(Some(new_slot));
        }

        source.set_outgoing_synapse_tail(Some(new_slot));
        target.set_incoming_synapse_tail(Some(new_slot));

        Some(new_slot)
    }

    pub fn disconnect(
        &self,
        source_slot: SlotId,
        target_slot: SlotId,
    ) -> Result<(), SlotAllocatorError> {
        let source = self.node_chain.get_node(source_slot);
        let mut synapse = source.get_outgoing_synapse_head();

        while synapse.is_some() {
            let synapse_handle = self.get_synapse(synapse.unwrap());
            let next_synapse = synapse_handle.get_outgoing_next_ptr();

            if synapse_handle.get_target_ptr() == target_slot {
                self.disconnect_synapse(synapse.unwrap())?;
            }

            synapse = next_synapse;
        }

        Ok(())
    }

    pub fn disconnect_synapse(&self, synapse_slot: SlotId) -> Result<(), SlotAllocatorError> {
        let synapse = self.get_synapse(synapse_slot);
        let source = self.node_chain.get_node(synapse.get_source_ptr());
        let target = self.node_chain.get_node(synapse.get_target_ptr());
        let synapse_outgoing_next_ptr = synapse.get_outgoing_next_ptr();
        let synapse_outgoing_prev_ptr = synapse.get_outgoing_prev_ptr();
        let synapse_incoming_next_ptr = synapse.get_incoming_next_ptr();
        let synapse_incoming_prev_ptr = synapse.get_incoming_prev_ptr();

        self.synapses.remove(synapse_slot)?;

        if synapse_outgoing_prev_ptr.is_some() {
            self.get_synapse(synapse_outgoing_prev_ptr.unwrap())
                .set_outgoing_next_ptr(synapse_outgoing_next_ptr);
        } else {
            source.set_outgoing_synapse_head(synapse_outgoing_next_ptr);
        }

        if synapse_outgoing_next_ptr.is_some() {
            self.get_synapse(synapse_outgoing_next_ptr.unwrap())
                .set_outgoing_prev_ptr(synapse_outgoing_prev_ptr);
        } else {
            source.set_outgoing_synapse_tail(synapse_outgoing_prev_ptr);
        }

        if synapse_incoming_prev_ptr.is_some() {
            self.get_synapse(synapse_incoming_prev_ptr.unwrap())
                .set_incoming_next_ptr(synapse_incoming_next_ptr);
        } else {
            target.set_incoming_synapse_head(synapse_incoming_next_ptr);
        }

        if synapse_incoming_next_ptr.is_some() {
            self.get_synapse(synapse_incoming_next_ptr.unwrap())
                .set_incoming_prev_ptr(synapse_incoming_prev_ptr);
        } else {
            target.set_incoming_synapse_tail(synapse_incoming_prev_ptr);
        }

        Ok(())
    }

    pub fn publish(&self) {
        self.node_chain.publish();
        self.synapses.publish()
    }

    pub fn copy_from(&self, source: &Self) {
        debug_assert!(
            source.node_capacity() <= self.node_capacity(),
            "NetworkWriter.copy_from | source.node_capacity() {} cannot be greater than destination.capacity {}",
            source.node_capacity(),
            self.node_capacity(),
        );

        debug_assert!(
            source.synapse_capacity() <= self.synapse_capacity(),
            "NetworkWriter.copy_from | source.synapse_capacity() {} cannot be greater than destination.capacity {}",
            source.synapse_capacity(),
            self.synapse_capacity(),
        );

        self.node_chain.copy_from(&source.node_chain);
        self.synapses.copy_from(&source.synapses);
    }
}
