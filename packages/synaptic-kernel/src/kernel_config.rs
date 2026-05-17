use crate::primitives::entry_store_def::EntryStoreDef;
use crate::primitives::lut_def::LutDef;
use crate::primitives::triple_buffer_def::TripleBufferDef;
use crate::topology::network::network_config::NetworkConfig;

/// Configuration for memory sizing of a Kernel.
///
/// Defines the capacities and metadata sizes used to pre-compute the required memory pool
/// sizes ahead of initialization.
///
/// # Fields
/// - `mem_metadata_size`: Power-of-2 size of the global metadata region residing
///    on the `mem` (direct) plane.
/// - `tb_defs`: Definitions for `TB_COUNT` user-allocated triple-buffers.
///   IDs must form a permutation of `[0, TB_COUNT-1]`.
///   The kernel-internal default TB is managed separately and is not to be included
///   in this array.
/// - `store_defs`: Definitions for `STORE_COUNT` user-allocated entity stores.
///   IDs must form a permutation of `[0, STORE_COUNT-1]`.
/// - `lut_defs`: Definitions for `LUT_COUNT` user-allocated LUTs.
///   IDs must form a permutation of `[0, LUT_COUNT-1]`.
/// - `network_config`: Network configuration of strides, node counts and synapse counts.
#[derive(Clone)]
pub struct KernelConfig<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize> {
    pub mem_metadata_size: usize,
    pub tb_defs: [TripleBufferDef; TB_COUNT],
    pub store_defs: [EntryStoreDef; STORE_COUNT],
    pub lut_defs: [LutDef; LUT_COUNT],
    pub network_config: NetworkConfig,
}
