//! Multi-config (non-`<1, 1, 1>`) registry tests.
//!
//! The `EntryStoreWriterRegistry::create` cursor math stacks stores within
//! a single triple buffer: when N stores share TB X, store K's tb_start is
//! `sum(size_on_tb(stores[0..K]) where tb_id == X)`. This cursor logic
//! (`extra_tb_cursors[index] += def.size_on_tb()`) is exercised here.
//!
//! The kernel is built with two user TBs and four entry stores split
//! across them so the cursors advance in both the default-TB and user-TB
//! paths. Each store gets a distinct entry, and the test verifies their
//! per-store data does not bleed into a sibling store sharing the same TB
//! after publish/swap.

mod common;

use synaptic_kernel::epoch_consumer::EpochConsumer;
use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;
use synaptic_kernel::primitives::entry_store_config::EntryStoreConfig;
use synaptic_kernel::primitives::entry_store_def::{EntryStoreDef, EntryStoreId};
use synaptic_kernel::primitives::lut_def::{LutDef, LutId};
use synaptic_kernel::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use synaptic_kernel::topology::network::network_config::NetworkConfig;

const NODE_META: usize = 4;
const NODE_ATTR: usize = 4;
const SYNAPSE_META: usize = 4;
const SYNAPSE_ATTR: usize = 4;

/// `Kernel<TB_COUNT=2, STORE_COUNT=4, LUT_COUNT=2>`.
///
/// Layout:
///   - default TB: stores 0 and 1, both with distinct strides
///   - user TB 0: stores 2 and 3 + LUT 0
///   - user TB 1: LUT 1
///
/// This stacks two stores in the default TB AND two stores in a user TB,
/// hitting both cursor branches in `EntryStoreWriterRegistry::create`.
type TestKernel = Kernel<2, 4, 2>;
type TestConsumer = EpochConsumer<2, 4, 2>;

const STORE_0_CAP: u32 = 4;
const STORE_1_CAP: u32 = 8;
const STORE_2_CAP: u32 = 4;
const STORE_3_CAP: u32 = 4;

const STORE_0_CORE: usize = 2;
const STORE_0_META: usize = 1;
const STORE_0_ATTR: usize = 2;
const STORE_1_CORE: usize = 4;
const STORE_1_META: usize = 2;
const STORE_1_ATTR: usize = 1;
const STORE_2_CORE: usize = 1;
const STORE_2_META: usize = 1;
const STORE_2_ATTR: usize = 1;
const STORE_3_CORE: usize = 3;
const STORE_3_META: usize = 1;
const STORE_3_ATTR: usize = 2;

fn build_config() -> KernelConfig<2, 4, 2> {
    KernelConfig {
        mem_metadata_size: 1,
        tb_defs: [
            TripleBufferDef {
                id: TripleBufferId(0),
                buffer_capacity: 1024,
            },
            TripleBufferDef {
                id: TripleBufferId(1),
                buffer_capacity: 256,
            },
        ],
        store_defs: [
            EntryStoreDef::new(
                EntryStoreId(0),
                TripleBufferId::DEFAULT,
                EntryStoreConfig {
                    core_stride: STORE_0_CORE,
                    meta_stride: STORE_0_META,
                    attr_stride: STORE_0_ATTR,
                    capacity: STORE_0_CAP,
                },
            ),
            EntryStoreDef::new(
                EntryStoreId(1),
                TripleBufferId::DEFAULT,
                EntryStoreConfig {
                    core_stride: STORE_1_CORE,
                    meta_stride: STORE_1_META,
                    attr_stride: STORE_1_ATTR,
                    capacity: STORE_1_CAP,
                },
            ),
            EntryStoreDef::new(
                EntryStoreId(2),
                TripleBufferId(0),
                EntryStoreConfig {
                    core_stride: STORE_2_CORE,
                    meta_stride: STORE_2_META,
                    attr_stride: STORE_2_ATTR,
                    capacity: STORE_2_CAP,
                },
            ),
            EntryStoreDef::new(
                EntryStoreId(3),
                TripleBufferId(0),
                EntryStoreConfig {
                    core_stride: STORE_3_CORE,
                    meta_stride: STORE_3_META,
                    attr_stride: STORE_3_ATTR,
                    capacity: STORE_3_CAP,
                },
            ),
        ],
        lut_defs: [
            LutDef::new(LutId(0), TripleBufferId(0), 4),
            LutDef::new(LutId(1), TripleBufferId(1), 8),
        ],
        network_config: NetworkConfig {
            node_capacity: 8,
            node_meta_stride: NODE_META,
            node_attr_stride: NODE_ATTR,
            synapse_capacity: 8,
            synapse_meta_stride: SYNAPSE_META,
            synapse_attr_stride: SYNAPSE_ATTR,
        },
    }
}

#[test]
fn registry_constructs_with_multiple_stores_sharing_one_tb() {
    // Construction-time cursor math: registry creation must succeed for
    // 2 stores in the default TB AND 2 stores in user TB 0 simultaneously.
    let _kernel = TestKernel::new(build_config());
}

#[test]
fn entries_in_sibling_default_tb_stores_do_not_alias() {
    // Stores 0 and 1 share the default TB. Entries inserted into store 0
    // and store 1 must occupy disjoint TB regions: writing to store 0's
    // entry must not affect store 1's entry, and vice versa.
    let mut kernel = TestKernel::new(build_config());
    let mut consumer = TestConsumer::new(kernel.get_control_plane());

    let s0_slot = kernel.get_entry_store(EntryStoreId(0)).insert().unwrap();
    let s1_slot = kernel.get_entry_store(EntryStoreId(1)).insert().unwrap();

    // Distinct values across all of store 0's core+meta range.
    let store0 = kernel.get_entry_store(EntryStoreId(0)).get(s0_slot);
    for i in 0..STORE_0_CORE {
        store0.core_write(i, 100 + i as i32);
    }
    for i in 0..STORE_0_META {
        store0.meta_write(i, 200 + i as i32);
    }

    // And store 1.
    let store1 = kernel.get_entry_store(EntryStoreId(1)).get(s1_slot);
    for i in 0..STORE_1_CORE {
        store1.core_write(i, 1000 + i as i32);
    }
    for i in 0..STORE_1_META {
        store1.meta_write(i, 2000 + i as i32);
    }

    kernel.publish();
    let mirror = consumer.acquire_mirror();

    let r0 = mirror.get_entry_store(EntryStoreId(0)).get(s0_slot);
    for i in 0..STORE_0_CORE {
        assert_eq!(r0.core_read(i), 100 + i as i32, "store 0 core[{}]", i);
    }
    for i in 0..STORE_0_META {
        assert_eq!(r0.meta_read(i), 200 + i as i32, "store 0 meta[{}]", i);
    }

    let r1 = mirror.get_entry_store(EntryStoreId(1)).get(s1_slot);
    for i in 0..STORE_1_CORE {
        assert_eq!(r1.core_read(i), 1000 + i as i32, "store 1 core[{}]", i);
    }
    for i in 0..STORE_1_META {
        assert_eq!(r1.meta_read(i), 2000 + i as i32, "store 1 meta[{}]", i);
    }
}

#[test]
fn entries_in_sibling_user_tb_stores_do_not_alias() {
    // Stores 2 and 3 share user TB 0. Same isolation contract as above,
    // but exercises the `extra_tb_cursors[index] += ...` branch.
    let kernel = TestKernel::new(build_config());
    let mut consumer = TestConsumer::new(kernel.get_control_plane());

    let s2_slot = kernel.get_entry_store(EntryStoreId(2)).insert().unwrap();
    let s3_slot = kernel.get_entry_store(EntryStoreId(3)).insert().unwrap();

    let store2 = kernel.get_entry_store(EntryStoreId(2)).get(s2_slot);
    for i in 0..STORE_2_CORE {
        store2.core_write(i, 7700 + i as i32);
    }

    let store3 = kernel.get_entry_store(EntryStoreId(3)).get(s3_slot);
    for i in 0..STORE_3_CORE {
        store3.core_write(i, 8800 + i as i32);
    }

    // Stores 2 and 3 live on user TB 0; default-TB publish doesn't expose
    // their core/meta. Drive the user TB independently.
    kernel.publish_tb(TripleBufferId(0));
    let mirror = consumer.acquire_mirror();
    mirror.swap_tb(TripleBufferId(0));

    let r2 = mirror.get_entry_store(EntryStoreId(2)).get(s2_slot);
    for i in 0..STORE_2_CORE {
        assert_eq!(r2.core_read(i), 7700 + i as i32, "store 2 core[{}]", i);
    }

    let r3 = mirror.get_entry_store(EntryStoreId(3)).get(s3_slot);
    for i in 0..STORE_3_CORE {
        assert_eq!(r3.core_read(i), 8800 + i as i32, "store 3 core[{}]", i);
    }
}

#[test]
fn store_capacities_are_independent() {
    // Saturation-isolation: filling store 0 to its full capacity must not
    // prevent store 1 from accepting new entries, and vice versa. Catches
    // a future bug where the cursor math accidentally collapsed the two
    // stores into a shared allocator.
    let kernel = TestKernel::new(build_config());

    let store0 = kernel.get_entry_store(EntryStoreId(0));
    for _ in 0..STORE_0_CAP {
        store0.insert().expect("store 0 must accept up to its capacity");
    }
    assert!(
        store0.insert().is_none(),
        "store 0 should be full at its declared capacity"
    );

    // Store 1 has its own allocator with a different capacity — must
    // accept STORE_1_CAP inserts.
    let store1 = kernel.get_entry_store(EntryStoreId(1));
    for _ in 0..STORE_1_CAP {
        store1.insert().expect("store 1 must accept up to its capacity");
    }
    assert!(
        store1.insert().is_none(),
        "store 1 should be full at its declared capacity"
    );
}

#[test]
fn lut_on_user_tb_visible_after_publish_tb() {
    // LUT 1 lives on user TB 1 — exercises that LUTs and stores can sit
    // on different user TBs simultaneously. Default publish must not
    // affect user TB 1 visibility.
    let mut kernel = TestKernel::new(build_config());
    let mut consumer = TestConsumer::new(kernel.get_control_plane());

    kernel.get_lut(LutId(0)).write(0, 11);
    kernel.get_lut(LutId(1)).write(0, 22);

    kernel.publish_tb(TripleBufferId(0));
    kernel.publish_tb(TripleBufferId(1));
    kernel.publish();

    let mirror = consumer.acquire_mirror();
    mirror.swap_tb(TripleBufferId(0));
    mirror.swap_tb(TripleBufferId(1));

    assert_eq!(mirror.get_lut(LutId(0)).read(0), 11);
    assert_eq!(mirror.get_lut(LutId(1)).read(0), 22);
}
