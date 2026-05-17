use crate::epoch_mirror::EpochMirror;
use crate::kernel_config::KernelConfig;
use crate::metadata::mem_metadata_writer::MemMetadataWriter;
use crate::primitives::entry_store_writer_registry::EntryStoreWriterRegistry;
use crate::primitives::lut_writer_registry::LutWriterRegistry;
use crate::primitives::triple_buffer_def::TripleBufferId;
use crate::primitives::triple_buffer_writer_registry::TripleBufferWriterRegistry;
use crate::primitives::types::AtomicBuffer;
use crate::topology::network::network_writer::NetworkWriter;
use std::sync::Arc;

/// Producer-side network and data orchestrator.
///
/// Provides the unified API for mutating the lock-free graph topology and attributes.
/// It encapsulates the underlying memory hierarchy and handles deploying structural updates
/// to the consumer via the `publish()`.
///
/// # Threading
/// Producer thread only.
///
/// # MEM Memory Layout (direct plane)
/// Segments are packed sequentially in a single AtomicBuffer.
///
/// ```text
/// Order       Segment                 Size
/// -------------------------------------------------------------------------------------------
/// 1           Mem Metadata            MemMetadataWriter::calculate_size_on_mem()
/// 2           TB Registry             TripleBufferWriterRegistry::calculate_size_on_mem()
/// 3           Network                 NetworkWriter::calculate_size_on_mem()
/// 4           Entry Store Registry    EntryStoreWriterRegistry::calculate_size_on_mem()
/// ```
///
/// # DEFAULT TB Memory Layout (triple-buffered plane)
/// Segments are packed sequentially within the TripleBufferWriter.
///
/// ```text
/// Order       Segment                 Size
/// --------------------------------------------------------------------------
/// 1           Network                 NetworkWriter::calculate_size_on_tb()
/// 2           Entry Store Registry    EntryStoreWriterRegistry::calculate_size_on_default_tb()
/// 3           LUT Registry            LutWriterRegistry::calculate_size_on_default_tb()
/// ```
///
/// # Deployment
/// 1. Structural updates (e.g. `insert_node`, `connect`) and tb_metadata are written to the active
///    triple-buffer segment.
/// 2. Non-structural updates (e.g. node/synapse attributes) and mem_metadata are written
///    directly to `mem` (direct) plane, making such writes immediately visible to the consumer.
/// 3. `publish()` flushes deferred frees and swaps the default triple-buffer,
///    exposing the new state to the consumer.
/// 4. `publish_tb(id)` independently publishes single user TB.
///
/// # Traits
/// - Memory sizing is defined at compile time via const generics.
/// - Use `to_mirror()` to create the paired `EpochMirror`.
#[derive(Clone)]
pub struct Epoch<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize> {
    pub mem_metadata: MemMetadataWriter,
    pub tb_registry: TripleBufferWriterRegistry<TB_COUNT>,
    pub network: NetworkWriter,
    pub store_registry: EntryStoreWriterRegistry<TB_COUNT, STORE_COUNT>,
    pub lut_registry: LutWriterRegistry<TB_COUNT, LUT_COUNT>,
}

impl<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize>
    Epoch<TB_COUNT, STORE_COUNT, LUT_COUNT>
{
    pub fn new(
        mem: AtomicBuffer,
        config: KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>,
        mem_start_offset: usize,
    ) -> Self {
        Self::create(mem, config, mem_start_offset, false)
    }

    pub fn bind(
        mem: AtomicBuffer,
        config: KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>,
        mem_start_offset: usize,
    ) -> Self {
        Self::create(mem, config, mem_start_offset, true)
    }

    pub fn create(
        mem: AtomicBuffer,
        config: KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>,
        mem_start_offset: usize,
        bind: bool,
    ) -> Self {
        let tb_start_offset = 0;

        let mem_metadata = MemMetadataWriter::create(
            Arc::clone(&mem),
            mem_start_offset,
            config.mem_metadata_size,
            bind,
        );
        let tb_registry = TripleBufferWriterRegistry::create(
            Arc::clone(&mem),
            config.tb_defs,
            mem_metadata.mem_end_offset(),
            Self::calculate_size_on_default_tb(&config) as u32,
            bind,
        );
        let network = NetworkWriter::create(
            Arc::clone(&mem),
            tb_registry.get(TripleBufferId::DEFAULT).clone(),
            config.network_config,
            tb_registry.mem_end_offset(),
            tb_start_offset,
            bind,
        );

        let store_registry = EntryStoreWriterRegistry::create(
            Arc::clone(&mem),
            tb_registry.clone(),
            config.store_defs,
            network.mem_end_offset(),
            network.tb_end_offset(),
            [0; TB_COUNT], // Extra TB-s have no prior consumers than user-defined entry stores.
            bind,
        );

        let lut_registry = LutWriterRegistry::create(
            tb_registry.clone(),
            config.lut_defs,
            store_registry.default_tb_end_offset(),
            store_registry.extra_tb_end_offsets(),
            bind,
        );

        Epoch {
            mem_metadata,
            tb_registry,
            network,
            store_registry,
            lut_registry,
        }
    }

    pub fn calculate_size_on_mem(config: &KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>) -> usize {
        MemMetadataWriter::calculate_size_on_mem(config.mem_metadata_size)
            + TripleBufferWriterRegistry::calculate_size_on_mem(
                Self::calculate_size_on_default_tb(&config),
                &config.tb_defs,
            )
            + NetworkWriter::calculate_size_on_mem(&config.network_config)
            + EntryStoreWriterRegistry::<TB_COUNT, STORE_COUNT>::calculate_size_on_mem(
                &config.store_defs,
            )
    }

    pub fn calculate_size_on_default_tb(
        config: &KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>,
    ) -> usize {
        NetworkWriter::calculate_size_on_tb(&config.network_config)
            + EntryStoreWriterRegistry::<TB_COUNT, STORE_COUNT>::calculate_size_on_default_tb(
                &config.store_defs,
            )
            + LutWriterRegistry::<TB_COUNT, LUT_COUNT>::calculate_size_on_default_tb(
                &config.lut_defs,
            )
    }

    pub fn to_mirror(&self) -> EpochMirror<TB_COUNT, STORE_COUNT, LUT_COUNT> {
        EpochMirror::bind(
            self.mem_metadata.to_reader(),
            self.tb_registry.to_reader(),
            self.store_registry.to_reader(),
            self.lut_registry.to_reader(),
            self.network.to_reader(),
        )
    }

    pub fn publish(&self) {
        self.network.publish();
        self.store_registry.publish_all();
        self.tb_registry.get(TripleBufferId::DEFAULT).publish();
    }

    pub fn publish_tb(&self, tb_id: TripleBufferId) {
        debug_assert!(
            tb_id.0 != TripleBufferId::DEFAULT.0,
            "Epoch::publish_tb | publish_tb() cannot be called on default TB",
        );

        self.tb_registry.get(tb_id).publish();
    }

    pub fn copy_from(&self, source: &Self) {
        debug_assert!(
            source.mem_metadata.capacity() <= self.mem_metadata.capacity(),
            "Epoch.copy_from | source.mem_metadata.capacity() {} cannot be greater than destination.mem_metadata.capacity() {}",
            source.mem_metadata.capacity(),
            self.mem_metadata.capacity(),
        );

        debug_assert!(
            source.network.node_capacity() <= self.network.node_capacity(),
            "Epoch.copy_from | source.network.node_capacity() {} cannot be greater than destination.network.node_capacity() {}",
            source.network.node_capacity(),
            self.network.node_capacity(),
        );

        debug_assert!(
            source.network.synapse_capacity() <= self.network.synapse_capacity(),
            "Epoch.copy_from | source.network.synapse_capacity() {} cannot be greater than destination.network.synapse_capacity() {}",
            source.network.synapse_capacity(),
            self.network.synapse_capacity(),
        );

        self.mem_metadata.copy_from(&source.mem_metadata);
        self.tb_registry
            .copy_metadata_regions_from(&source.tb_registry);
        self.network.copy_from(&source.network);
        self.store_registry.copy_from(&source.store_registry);
        self.lut_registry.copy_from(&source.lut_registry);
    }
}
