use crate::metadata::mem_metadata_reader::MemMetadataReader;
use crate::primitives::entry_store_def::EntryStoreId;
use crate::primitives::entry_store_reader::EntryStoreReader;
use crate::primitives::entry_store_reader_registry::EntryStoreReaderRegistry;
use crate::primitives::lut_def::LutId;
use crate::primitives::lut_reader::LutReader;
use crate::primitives::lut_reader_registry::LutReaderRegistry;
use crate::primitives::slot::SlotId;
use crate::primitives::tb_reader::TbReader;
use crate::primitives::triple_buffer_def::TripleBufferId;
use crate::primitives::triple_buffer_reader_registry::TripleBufferReaderRegistry;
use crate::topology::network::network_reader::NetworkReader;
use crate::topology::network::synapse_reader::SynapseReader;
use crate::topology::node::node_reader::NodeReader;

/// Consumer-side epoch mirror.
///
/// Provides the unified API for traversing the lock-free and wait-free graph
/// topology and attributes.
/// It encapsulates the underlying memory hierarchy and processes incoming structural updates
/// by the producer via `swap()`.
///
/// # Threading
/// Consumer thread only.
///
/// # Memory Layout
/// Shares backing MEM (direct plane) and TB (triple-buffered plane) regions
/// with `Epoch`. See its layout.
///
/// # Deployment
/// 1. `swap()` consumes any pending structural updates (node, synapses, tb_metadata) published
///    by the producer on the default TB.
///    Returns `true` if a new buffer was available, `false` otherwise.
/// 2. `swap_tb(id)` independently consumes a single user TB's pending updates.
/// 3. Non-structural updates (e.g. node/synapse attributes) and mem_metadata read directly
///    from the `mem` plane.
///
/// # Traits
/// - Memory sizing is defined at compile time via const generics.
/// - Created exclusively via `Epoch::to_mirror()`.
#[derive(Clone)]
pub struct EpochMirror<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize> {
    mem_metadata: MemMetadataReader,
    tb_registry: TripleBufferReaderRegistry<TB_COUNT>,
    store_registry: EntryStoreReaderRegistry<TB_COUNT, STORE_COUNT>,
    lut_registry: LutReaderRegistry<TB_COUNT, LUT_COUNT>,
    network: NetworkReader,
}

impl<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize>
    EpochMirror<TB_COUNT, STORE_COUNT, LUT_COUNT>
{
    pub(crate) fn bind(
        mem_metadata: MemMetadataReader,
        tb_registry: TripleBufferReaderRegistry<TB_COUNT>,
        store_registry: EntryStoreReaderRegistry<TB_COUNT, STORE_COUNT>,
        lut_registry: LutReaderRegistry<TB_COUNT, LUT_COUNT>,
        network: NetworkReader,
    ) -> Self {
        EpochMirror {
            mem_metadata,
            tb_registry,
            store_registry,
            lut_registry,
            network,
        }
    }

    #[inline]
    pub fn mem_metadata_capacity(&self) -> usize {
        self.mem_metadata.capacity()
    }

    #[inline]
    pub fn mem_read_meta(&self, offset: usize) -> i32 {
        self.mem_metadata.read(offset)
    }

    #[inline]
    pub fn get_user_tb(&'_ self, tb_id: TripleBufferId) -> TbReader<'_> {
        debug_assert!(
            tb_id.0 != TripleBufferId::DEFAULT.0,
            "EpochMirror::get_user_tb | default TB cannot be accessed using get_user_tb()",
        );

        TbReader::bind(self.tb_registry.get(tb_id))
    }

    #[inline]
    pub fn get_entry_store(&'_ self, store_id: EntryStoreId) -> &EntryStoreReader {
        self.store_registry.get(store_id)
    }

    #[inline]
    pub fn get_lut(&'_ self, lut_id: LutId) -> &LutReader {
        self.lut_registry.get(lut_id)
    }

    #[inline]
    pub fn get_node(&'_ self, slot: SlotId) -> NodeReader<'_> {
        self.network.get_node(slot)
    }

    #[inline]
    pub fn get_synapse(&'_ self, slot: SlotId) -> SynapseReader<'_> {
        self.network.get_synapse(slot)
    }

    pub fn swap(&self) -> bool {
        let swapped = self.tb_registry.get(TripleBufferId::DEFAULT).swap();
        self.network.ack_generation();
        swapped
    }

    pub fn swap_tb(&self, tb_id: TripleBufferId) {
        debug_assert!(
            tb_id.0 != TripleBufferId::DEFAULT.0,
            "EpochMirror::swap_tb | swap_tb() cannot be called on default TB",
        );

        self.tb_registry.get(tb_id).swap();
    }
}
