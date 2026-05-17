//! Shared helpers for integration tests (not test cases themselves).

use synaptic_kernel::kernel_config::KernelConfig;
use synaptic_kernel::primitives::entry_store_config::EntryStoreConfig;
use synaptic_kernel::primitives::entry_store_def::{EntryStoreDef, EntryStoreId};
use synaptic_kernel::primitives::lut_def::{LutDef, LutId};
use synaptic_kernel::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use synaptic_kernel::topology::network::network_config::NetworkConfig;

/// Same TB / dummy store layout as [`kernel_config_1_1`], but caller supplies `network_config`.
pub fn kernel_config_1_1_network(
    mem_metadata_size: usize,
    network_config: NetworkConfig,
    user_tb_buffer_capacity: usize,
) -> KernelConfig<1, 1, 1> {
    KernelConfig {
        mem_metadata_size,
        tb_defs: [TripleBufferDef {
            id: TripleBufferId(0),
            buffer_capacity: user_tb_buffer_capacity,
        }],
        store_defs: [EntryStoreDef::new(
            EntryStoreId(0),
            TripleBufferId::DEFAULT,
            EntryStoreConfig {
                core_stride: 1,
                meta_stride: 1,
                attr_stride: 1,
                capacity: 4,
            },
        )],
        lut_defs: [LutDef::new(LutId(0), TripleBufferId::DEFAULT, 1)],
        network_config,
    }
}

/// `Kernel<1, 1, 1>` / `Epoch<1, 1, 1>` configuration with one user TB, one dummy entry store, and one dummy LUT.
///
/// `user_tb_buffer_capacity` must be large enough for `Epoch::calculate_size_on_default_tb`.
/// `dummy_store_capacity` keeps the registry layout valid; tests that never touch the user store
/// can use a small positive value.
pub fn kernel_config_1_1(
    node_capacity: u32,
    synapse_capacity: u32,
    node_meta_stride: usize,
    node_attr_stride: usize,
    synapse_meta_stride: usize,
    synapse_attr_stride: usize,
) -> KernelConfig<1, 1, 1> {
    kernel_config_1_1_full(
        1,
        node_capacity,
        synapse_capacity,
        node_meta_stride,
        node_attr_stride,
        synapse_meta_stride,
        synapse_attr_stride,
        32768,
    )
}

pub fn kernel_config_1_1_with_tb(
    node_capacity: u32,
    synapse_capacity: u32,
    node_meta_stride: usize,
    node_attr_stride: usize,
    synapse_meta_stride: usize,
    synapse_attr_stride: usize,
    user_tb_buffer_capacity: usize,
) -> KernelConfig<1, 1, 1> {
    kernel_config_1_1_full(
        1,
        node_capacity,
        synapse_capacity,
        node_meta_stride,
        node_attr_stride,
        synapse_meta_stride,
        synapse_attr_stride,
        user_tb_buffer_capacity,
    )
}

pub fn kernel_config_1_1_full(
    mem_metadata_size: usize,
    node_capacity: u32,
    synapse_capacity: u32,
    node_meta_stride: usize,
    node_attr_stride: usize,
    synapse_meta_stride: usize,
    synapse_attr_stride: usize,
    user_tb_buffer_capacity: usize,
) -> KernelConfig<1, 1, 1> {
    KernelConfig {
        mem_metadata_size,
        tb_defs: [TripleBufferDef {
            id: TripleBufferId(0),
            buffer_capacity: user_tb_buffer_capacity,
        }],
        store_defs: [EntryStoreDef::new(
            EntryStoreId(0),
            TripleBufferId::DEFAULT,
            EntryStoreConfig {
                core_stride: 1,
                meta_stride: 1,
                attr_stride: 1,
                capacity: 4,
            },
        )],
        lut_defs: [LutDef::new(LutId(0), TripleBufferId::DEFAULT, 1)],
        network_config: NetworkConfig {
            node_capacity,
            node_meta_stride,
            node_attr_stride,
            synapse_capacity,
            synapse_meta_stride,
            synapse_attr_stride,
        },
    }
}
