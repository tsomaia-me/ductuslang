use crate::constants::{KERNEL_MAGIC, KERNEL_VERSION};
use crate::control_plane::ControlPlane;
use crate::epoch::Epoch;
use crate::epoch_mirror::EpochMirror;
use crate::errors::kernel_error::KernelError;
use crate::errors::slot_allocator_error::SlotAllocatorError;
use crate::kernel_config::KernelConfig;
use crate::primitives::entry_store_def::{EntryStoreDef, EntryStoreId};
use crate::primitives::entry_store_writer::EntryStoreWriter;
use crate::primitives::lut_def::{LutDef, LutId};
use crate::primitives::lut_writer::LutWriter;
use crate::primitives::slot::SlotId;
use crate::primitives::tb_writer::TbWriter;
use crate::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use crate::primitives::types::AtomicBuffer;
use crate::serialized_kernel::SerializedKernel;
use crate::topology::network::synapse_handle::SynapseView;
use crate::topology::node::node_handle::NodeHandle;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

/// Producer-side entry point to the synaptic graph.
///
/// Owns the graph memory, the active graph writer, the control plane, and the
/// deferred-deletion queue.
/// Provides a unified API for building and mutating a lock-free, wait-free SPSC
/// graph topology.
///
/// # Threading
/// Producer thread only. The consumer accesses the graph exclusively through
/// a [`EpochConsumer`] obtained via [`get_control_plane()`].
///
/// # Lifecycle
/// 1. Create via [`new()`] or restore via [`load_serialized()`].
/// 2. Mutate: insert/remove nodes, connect/disconnect synapses, write attributes.
/// 3. Call [`publish()`] to deploy structural changes to the consumer and reclaim
///    generation-acknowledged deferred deletions.
/// 4. Call [`publish_tb(id)`] to independently deploy a user TB updates.
/// 4. Call [`grow()`] when utilization exceeds the target threshold.
///    This allocates a new, larger backing buffer, migrates all state, and
///    hot-swaps the consumer's graph via the [`ControlPlane`].
/// 5. Call [`serialize()`] to a snapshot for persistence.
///
/// # Memory Model
/// - **Structural Changes** (nodes, synapses, topology metadata) are written to the
///   default tripl buffer and become visible to the consumer after [`publish()`].
/// - **User TB Changes** are written to user-allocated triple buffers and become
///   visible to the consumer after [`publish_tb(id)`].
/// - **Attributes** (node and synapse) and **mem metadata** are written to the direct
///   plane and are visible to the consumer immediately.
/// - **Deferred deletions** (removed nodes, disconnected synapses) are staged in a
///   generation-gated buffer and reclaimed during [`publish()`] once the consumer has
///   acknowledged the relevant generation.
///
/// # Safety Contract
/// The consumer thread **must** be fully quiesced before the `Kernel` is dropped.
/// Dropping the kernel unconditionally frees the deferred-deletion queue and the backing
/// memory. If the consumer is still traversing a hot-swapped graph, the result
/// is undefined behavior.
pub struct Kernel<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize> {
    config: KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>,
    mem: AtomicBuffer,
    control_plane: Arc<ControlPlane<TB_COUNT, STORE_COUNT, LUT_COUNT>>,
    active_epoch: Epoch<TB_COUNT, STORE_COUNT, LUT_COUNT>,
    readers_pending_deletion: VecDeque<(Box<EpochMirror<TB_COUNT, STORE_COUNT, LUT_COUNT>>, i32)>,
}

impl<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize>
    Kernel<TB_COUNT, STORE_COUNT, LUT_COUNT>
{
    pub const HEADERS_SIZE: usize = 2;

    pub fn new(config: KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>) -> Self {
        let mem = Self::create_mem(Self::calculate_size_on_mem(&config));
        Self::new_from_mem(mem, config)
    }

    pub fn new_from_mem(
        mem: AtomicBuffer,
        config: KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>,
    ) -> Self {
        assert!(
            mem[0].load(Ordering::Acquire) == 0 && mem[1].load(Ordering::Acquire) == 0,
            "Attempted to initialize Kernel on already allocated memory"
        );

        assert!(
            mem.len() >= Self::calculate_size_on_mem(&config),
            "Provided AtomicBuffer is too small for this configuration"
        );

        let epoch = Epoch::new(Arc::clone(&mem), config.clone(), Self::HEADERS_SIZE);
        let mirror = Box::new(epoch.to_mirror());
        let control_plane = Arc::new(ControlPlane::new(mirror));

        Self::stamp_mem(&mem);

        Kernel {
            config,
            mem,
            control_plane,
            active_epoch: epoch,
            readers_pending_deletion: VecDeque::new(),
        }
    }

    pub fn load_serialized(
        serialized_kernel: SerializedKernel<TB_COUNT, STORE_COUNT, LUT_COUNT>,
    ) -> Self {
        let config = serialized_kernel.config;
        let mem: AtomicBuffer = serialized_kernel
            .mem
            .into_iter()
            .map(AtomicI32::new)
            .collect();

        assert_eq!(
            mem[0].load(Ordering::Acquire),
            KERNEL_MAGIC,
            "Attempted to initialize Kernel on foreign memory"
        );
        assert_eq!(
            mem[1].load(Ordering::Acquire),
            KERNEL_VERSION,
            "Attempted to initialize Kernel on mismatched AtomicBuffer version"
        );

        assert!(
            mem.len() >= Self::calculate_size_on_mem(&config),
            "Provided AtomicBuffer is too small for this configuration"
        );

        let writer = Epoch::bind(Arc::clone(&mem), config.clone(), Self::HEADERS_SIZE);
        let reader = Box::new(writer.to_mirror());
        let control_plane = Arc::new(ControlPlane::new(reader));

        Kernel {
            config,
            mem,
            control_plane,
            active_epoch: writer,
            readers_pending_deletion: VecDeque::new(),
        }
    }

    pub fn calculate_size_on_mem(config: &KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>) -> usize {
        Self::HEADERS_SIZE
            + Epoch::<TB_COUNT, STORE_COUNT, LUT_COUNT>::calculate_size_on_mem(&config)
    }

    /// Snapshots the current kernel state for persistence.
    ///
    /// # Safety Contract
    /// The consumer thread **must** be fully quiesced before calling `serialize`.
    /// If a consumer thread is actively traversing the topology or acking generations,
    /// the snapshot may capture a torn SPSC state (e.g., a triple buffer mid-swap).
    /// This is the same quiescence requirement that applies to dropping the Kernel.
    ///
    /// Note: publish() is called internally before snapshotting. Callers do not need
    /// to call publish() beforehand. Calling it explicitly before serialize() is
    /// harmless (double-publish is idempotent) but unnecessary.
    pub fn serialize(&mut self) -> SerializedKernel<TB_COUNT, STORE_COUNT, LUT_COUNT> {
        self.publish();

        let mem = self.mem.iter().map(|a| a.load(Ordering::Relaxed)).collect();

        SerializedKernel {
            config: self.config.clone(),
            mem,
        }
    }

    /// Returns a shared handle to the `ControlPlane` for constructing a `EpochConsumer` on
    /// the consumer thread.
    ///
    /// The `Arc` is a cross-thread transport mechanism, not a lifetime extension.
    /// The `ControlPlane` has no independent lifecycle - it is logically owned by
    /// this `Kernel`.
    ///
    /// # Safety Contract
    /// The consumer thread **must** be fully quiesced before the `Kernel` is dropped.
    /// Dropping the kernel unconditionally frees the deferred-deletion queue.
    /// If the consumer is still traversing a hot-swapped epoch, the result is
    /// undefined behavior.
    pub fn get_control_plane(&self) -> Arc<ControlPlane<TB_COUNT, STORE_COUNT, LUT_COUNT>> {
        Arc::clone(&self.control_plane)
    }

    #[inline]
    pub fn mem_metadata_capacity(&self) -> usize {
        self.active_epoch.mem_metadata.capacity()
    }

    #[inline]
    pub fn mem_read_meta(&self, offset: usize) -> i32 {
        self.active_epoch.mem_metadata.read(offset)
    }

    #[inline]
    pub fn mem_write_meta(&self, offset: usize, value: i32) {
        self.active_epoch.mem_metadata.write(offset, value);
    }

    #[inline]
    pub fn get_user_tb(&'_ self, tb_id: TripleBufferId) -> TbWriter<'_> {
        debug_assert!(
            tb_id.0 != TripleBufferId::DEFAULT.0,
            "Kernel::get_user_tb | default TB cannot be accessed using get_user_tb()",
        );

        TbWriter::bind(self.active_epoch.tb_registry.get(tb_id))
    }

    #[inline]
    pub fn get_entry_store(&self, store_id: EntryStoreId) -> &EntryStoreWriter {
        self.active_epoch.store_registry.get(store_id)
    }

    #[inline]
    pub fn get_lut(&self, lut_id: LutId) -> &LutWriter {
        self.active_epoch.lut_registry.get(lut_id)
    }

    #[inline]
    pub fn node_capacity(&self) -> usize {
        self.active_epoch.network.node_capacity()
    }

    #[inline]
    pub fn node_count(&self) -> usize {
        self.active_epoch.network.node_count()
    }

    #[inline]
    pub fn node_utilization(&self) -> f32 {
        self.active_epoch.network.node_utilization()
    }

    #[inline]
    pub fn synapse_capacity(&self) -> usize {
        self.active_epoch.network.synapse_capacity()
    }

    #[inline]
    pub fn synapse_count(&self) -> usize {
        self.active_epoch.network.synapse_count()
    }

    #[inline]
    pub fn synapse_utilization(&self) -> f32 {
        self.active_epoch.network.synapse_utilization()
    }

    #[inline]
    pub fn peek_utilization(&self) -> f32 {
        self.active_epoch.network.peek_utilization()
    }

    #[inline]
    pub fn get_node(&'_ self, slot: SlotId) -> NodeHandle<'_> {
        self.active_epoch.network.get_node_handle(slot)
    }

    #[inline]
    pub fn get_synapse(&'_ self, slot: SlotId) -> SynapseView<'_> {
        self.active_epoch.network.get_synapse_handle(slot)
    }

    pub fn insert_node(&self, kind: i32) -> Result<SlotId, KernelError> {
        match self.active_epoch.network.insert_node(kind) {
            Some(slot) => Ok(slot),
            None => Err(KernelError::CapacityExhausted),
        }
    }

    pub fn insert_node_after(&self, prev_slot: SlotId, kind: i32) -> Result<SlotId, KernelError> {
        match self.active_epoch.network.insert_node_after(prev_slot, kind) {
            Some(slot) => Ok(slot),
            None => Err(KernelError::CapacityExhausted),
        }
    }

    pub fn insert_node_before(&self, next_slot: SlotId, kind: i32) -> Result<SlotId, KernelError> {
        match self
            .active_epoch
            .network
            .insert_node_before(next_slot, kind)
        {
            Some(slot) => Ok(slot),
            None => Err(KernelError::CapacityExhausted),
        }
    }

    pub fn remove_node(&self, slot: SlotId) -> Result<(), SlotAllocatorError> {
        self.active_epoch.network.remove_node(slot)
    }

    pub fn remove_chain(&self, head_slot: SlotId) -> Result<(), SlotAllocatorError> {
        self.active_epoch.network.remove_chain(head_slot)
    }

    pub fn connect(
        &self,
        source_slot: SlotId,
        target_slot: SlotId,
        kind: i32,
    ) -> Result<SlotId, KernelError> {
        match self
            .active_epoch
            .network
            .connect(source_slot, target_slot, kind)
        {
            Some(slot) => Ok(slot),
            None => Err(KernelError::CapacityExhausted),
        }
    }

    pub fn disconnect(
        &self,
        source_slot: SlotId,
        target_slot: SlotId,
    ) -> Result<(), SlotAllocatorError> {
        self.active_epoch
            .network
            .disconnect(source_slot, target_slot)
    }

    pub fn disconnect_synapse(&self, slot: SlotId) -> Result<(), SlotAllocatorError> {
        self.active_epoch.network.disconnect_synapse(slot)
    }

    #[inline]
    pub fn should_grow(&self, target_resize_threshold: f32) -> bool {
        self.peek_utilization() > target_resize_threshold
    }

    pub fn publish(&mut self) {
        self.active_epoch.publish();
        let ack = self.control_plane.get_reader_ack_generation();

        while let Some((_, generation)) = self.readers_pending_deletion.front() {
            if (*generation).wrapping_sub(ack) > 0 {
                break;
            }

            self.readers_pending_deletion.pop_front();
        }
    }

    pub fn publish_tb(&self, tb_id: TripleBufferId) {
        self.active_epoch.publish_tb(tb_id);
    }

    pub fn grow(
        &mut self,
        config: KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>,
    ) -> Result<(), KernelError> {
        self.validate_config_compatibility(&config)?;

        self.mem = Self::create_mem_stamp(Self::calculate_size_on_mem(&config));
        let new_writer = Epoch::new(Arc::clone(&self.mem), config.clone(), Self::HEADERS_SIZE);

        new_writer.copy_from(&self.active_epoch);

        let new_reader = Box::new(new_writer.to_mirror());

        self.config = config;
        self.active_epoch = new_writer;
        let old_reader = self.control_plane.swap_epoch(new_reader);
        self.readers_pending_deletion.push_back(old_reader);

        Ok(())
    }

    fn validate_config_compatibility(
        &self,
        config: &KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>,
    ) -> Result<(), KernelError> {
        let new_nc = &config.network_config;
        let old_nc = &self.config.network_config;

        if new_nc.node_meta_stride != old_nc.node_meta_stride
            || new_nc.node_attr_stride != old_nc.node_attr_stride
            || new_nc.synapse_meta_stride != old_nc.synapse_meta_stride
            || new_nc.synapse_attr_stride != old_nc.synapse_attr_stride
        {
            return Err(KernelError::SchemaMismatch);
        }

        if config.network_config.node_capacity < self.node_capacity() as u32
            || config.network_config.synapse_capacity < self.synapse_capacity() as u32
            || config.mem_metadata_size < self.config.mem_metadata_size
        {
            return Err(KernelError::InsufficientCapacity);
        }

        self.validate_store_defs_compatibility(&config.store_defs)?;
        self.validate_tb_defs_compatibility(&config.tb_defs)?;
        self.validate_lut_defs_compatibility(&config.lut_defs)?;

        Ok(())
    }

    fn validate_tb_defs_compatibility(
        &self,
        tb_defs: &[TripleBufferDef; TB_COUNT],
    ) -> Result<(), KernelError> {
        for i in 0..self.config.tb_defs.len() {
            let old_def = &self.config.tb_defs[i];
            let new_def = tb_defs
                .iter()
                .find(|d| d.id == old_def.id)
                .ok_or(KernelError::SchemaMismatch)?;

            if new_def.buffer_capacity < old_def.buffer_capacity {
                return Err(KernelError::InsufficientCapacity);
            }
        }

        Ok(())
    }

    fn validate_store_defs_compatibility(
        &self,
        store_defs: &[EntryStoreDef; STORE_COUNT],
    ) -> Result<(), KernelError> {
        for i in 0..self.config.store_defs.len() {
            let old_def = &self.config.store_defs[i];
            let new_def = store_defs
                .iter()
                .find(|d| d.id == old_def.id)
                .ok_or(KernelError::SchemaMismatch)?;

            if new_def.tb_id != old_def.tb_id
                || new_def.config.core_stride != old_def.config.core_stride
                || new_def.config.meta_stride != old_def.config.meta_stride
                || new_def.config.attr_stride != old_def.config.attr_stride
            {
                return Err(KernelError::SchemaMismatch);
            }

            if new_def.config.capacity < old_def.config.capacity {
                return Err(KernelError::InsufficientCapacity);
            }
        }

        Ok(())
    }

    fn validate_lut_defs_compatibility(
        &self,
        lut_defs: &[LutDef; LUT_COUNT],
    ) -> Result<(), KernelError> {
        for i in 0..self.config.lut_defs.len() {
            let old_def = &self.config.lut_defs[i];
            let new_def = lut_defs
                .iter()
                .find(|d| d.id == old_def.id)
                .ok_or(KernelError::SchemaMismatch)?;

            if new_def.tb_id != old_def.tb_id {
                return Err(KernelError::SchemaMismatch);
            }

            if new_def.size < old_def.size {
                return Err(KernelError::InsufficientCapacity);
            }
        }

        Ok(())
    }

    /// Returns a raw handle to the backing `AtomicBuffer`.
    ///
    /// # Safety Contract
    /// The caller assumes full responsibility for memory correctness.
    /// Writing to structural or lifecycle regions will corrupt the graph.
    /// Intended exclusively for read-only telemetry and debugging.
    pub fn get_mem(&self) -> AtomicBuffer {
        Arc::clone(&self.mem)
    }

    fn create_mem(size: usize) -> AtomicBuffer {
        let mem: AtomicBuffer = (0..size).map(|_| AtomicI32::new(0)).collect();

        mem
    }

    fn create_mem_stamp(size: usize) -> AtomicBuffer {
        let mem = Self::create_mem(size);

        Self::stamp_mem(&mem);

        mem
    }

    fn stamp_mem(mem: &AtomicBuffer) {
        mem[0].store(KERNEL_MAGIC, Ordering::Release);
        mem[1].store(KERNEL_VERSION, Ordering::Release);
    }
}

#[cfg(debug_assertions)]
impl<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize> Drop
    for Kernel<TB_COUNT, STORE_COUNT, LUT_COUNT>
{
    fn drop(&mut self) {
        debug_assert_eq!(
            Arc::strong_count(&self.control_plane),
            1,
            "Kernel::drop | Consumer must be quiesced before dropping kernel"
        )
    }
}
