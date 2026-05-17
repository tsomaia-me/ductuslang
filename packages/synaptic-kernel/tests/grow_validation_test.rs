//! Negative- and positive-path coverage for `Kernel::grow` config validation.
//!
//! `grow` accepts a new `KernelConfig` and runs `validate_config_compatibility`
//! before touching any state. Two error variants surface here:
//!
//! - `KernelError::SchemaMismatch` — the new config differs from the old in
//!   any field that affects per-entry layout or where data physically lives:
//!   network strides, per-store strides + tb_id, per-LUT tb_id, plus any
//!   tb/store/lut id present in the old config but missing from the new one
//!   (renamed or removed).
//! - `KernelError::InsufficientCapacity` — schema matches but a capacity
//!   field would shrink: mem_metadata_size, network capacities, per-TB
//!   buffer_capacity, per-store capacity, per-LUT size.
//!
//! Validation runs entirely before any mutation, so a failed `grow` leaves
//! the kernel intact.

mod common;

use synaptic_kernel::errors::kernel_error::KernelError;
use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;
use synaptic_kernel::primitives::entry_store_config::EntryStoreConfig;
use synaptic_kernel::primitives::entry_store_def::{EntryStoreDef, EntryStoreId};
use synaptic_kernel::primitives::lut_def::{LutDef, LutId};
use synaptic_kernel::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use synaptic_kernel::topology::network::network_config::NetworkConfig;

const NODE_META: usize = 4;
const NODE_ATTR: usize = 8;
const SYNAPSE_META: usize = 4;
const SYNAPSE_ATTR: usize = 8;

const STORE_CORE: usize = 1;
const STORE_META: usize = 1;
const STORE_ATTR: usize = 1;
const STORE_CAP: u32 = 4;

const TB0_CAP: usize = 32768;
const LUT0_SIZE: usize = 1;

const NODE_CAP: u32 = 16;
const SYNAPSE_CAP: u32 = 16;
const MEM_METADATA_SIZE: usize = 1;

type Kernel111 = Kernel<1, 1, 1>;

/// Build the canonical `Kernel<1, 1, 1>` baseline config.
fn baseline_111() -> KernelConfig<1, 1, 1> {
    KernelConfig {
        mem_metadata_size: MEM_METADATA_SIZE,
        tb_defs: [TripleBufferDef {
            id: TripleBufferId(0),
            buffer_capacity: TB0_CAP,
        }],
        store_defs: [EntryStoreDef::new(
            EntryStoreId(0),
            TripleBufferId::DEFAULT,
            EntryStoreConfig {
                core_stride: STORE_CORE,
                meta_stride: STORE_META,
                attr_stride: STORE_ATTR,
                capacity: STORE_CAP,
            },
        )],
        lut_defs: [LutDef::new(LutId(0), TripleBufferId::DEFAULT, LUT0_SIZE)],
        network_config: NetworkConfig {
            node_capacity: NODE_CAP,
            node_meta_stride: NODE_META,
            node_attr_stride: NODE_ATTR,
            synapse_capacity: SYNAPSE_CAP,
            synapse_meta_stride: SYNAPSE_META,
            synapse_attr_stride: SYNAPSE_ATTR,
        },
    }
}

/// `Kernel<2, 1, 1>` baseline — used by the tb_id-mutation tests, which need
/// at least two user TBs so a store/LUT can be moved between them.
fn baseline_211() -> KernelConfig<2, 1, 1> {
    KernelConfig {
        mem_metadata_size: MEM_METADATA_SIZE,
        tb_defs: [
            TripleBufferDef {
                id: TripleBufferId(0),
                buffer_capacity: TB0_CAP,
            },
            TripleBufferDef {
                id: TripleBufferId(1),
                buffer_capacity: 256,
            },
        ],
        store_defs: [EntryStoreDef::new(
            EntryStoreId(0),
            TripleBufferId(0),
            EntryStoreConfig {
                core_stride: STORE_CORE,
                meta_stride: STORE_META,
                attr_stride: STORE_ATTR,
                capacity: STORE_CAP,
            },
        )],
        lut_defs: [LutDef::new(LutId(0), TripleBufferId(0), LUT0_SIZE)],
        network_config: NetworkConfig {
            node_capacity: NODE_CAP,
            node_meta_stride: NODE_META,
            node_attr_stride: NODE_ATTR,
            synapse_capacity: SYNAPSE_CAP,
            synapse_meta_stride: SYNAPSE_META,
            synapse_attr_stride: SYNAPSE_ATTR,
        },
    }
}

// =========================================================
// SCHEMA MISMATCH — NetworkConfig stride fields
// =========================================================

#[test]
fn grow_rejects_changed_node_meta_stride() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.network_config.node_meta_stride = NODE_META + 1;

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

#[test]
fn grow_rejects_changed_node_attr_stride() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.network_config.node_attr_stride = NODE_ATTR + 1;

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

#[test]
fn grow_rejects_changed_synapse_meta_stride() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.network_config.synapse_meta_stride = SYNAPSE_META + 1;

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

#[test]
fn grow_rejects_changed_synapse_attr_stride() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.network_config.synapse_attr_stride = SYNAPSE_ATTR + 1;

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

// =========================================================
// SCHEMA MISMATCH — Per-store layout fields
// =========================================================

#[test]
fn grow_rejects_changed_per_store_tb_id() {
    let mut kernel: Kernel<2, 1, 1> = Kernel::new(baseline_211());

    // Move the store from TripleBufferId(0) to TripleBufferId(1).
    let mut new_config = baseline_211();
    new_config.store_defs[0] = EntryStoreDef::new(
        EntryStoreId(0),
        TripleBufferId(1),
        EntryStoreConfig {
            core_stride: STORE_CORE,
            meta_stride: STORE_META,
            attr_stride: STORE_ATTR,
            capacity: STORE_CAP,
        },
    );

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

#[test]
fn grow_rejects_changed_per_store_core_stride() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.store_defs[0] = EntryStoreDef::new(
        EntryStoreId(0),
        TripleBufferId::DEFAULT,
        EntryStoreConfig {
            core_stride: STORE_CORE + 1,
            meta_stride: STORE_META,
            attr_stride: STORE_ATTR,
            capacity: STORE_CAP,
        },
    );

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

#[test]
fn grow_rejects_changed_per_store_meta_stride() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.store_defs[0] = EntryStoreDef::new(
        EntryStoreId(0),
        TripleBufferId::DEFAULT,
        EntryStoreConfig {
            core_stride: STORE_CORE,
            meta_stride: STORE_META + 1,
            attr_stride: STORE_ATTR,
            capacity: STORE_CAP,
        },
    );

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

#[test]
fn grow_rejects_changed_per_store_attr_stride() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.store_defs[0] = EntryStoreDef::new(
        EntryStoreId(0),
        TripleBufferId::DEFAULT,
        EntryStoreConfig {
            core_stride: STORE_CORE,
            meta_stride: STORE_META,
            attr_stride: STORE_ATTR + 1,
            capacity: STORE_CAP,
        },
    );

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

// =========================================================
// SCHEMA MISMATCH — Per-LUT layout fields
// =========================================================

#[test]
fn grow_rejects_changed_per_lut_tb_id() {
    let mut kernel: Kernel<2, 1, 1> = Kernel::new(baseline_211());

    // Move the LUT from TripleBufferId(0) to TripleBufferId(1).
    let mut new_config = baseline_211();
    new_config.lut_defs[0] = LutDef::new(LutId(0), TripleBufferId(1), LUT0_SIZE);

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

// =========================================================
// SCHEMA MISMATCH — Renamed (or removed) IDs
// =========================================================

#[test]
fn grow_rejects_renamed_tb_id() {
    // Need a kernel with at least one user TB. baseline_111 has TripleBufferId(0).
    let mut kernel = Kernel111::new(baseline_111());

    // Rename the only user TB from id 0 to id (still in [0, TB_COUNT-1]).
    // For TB_COUNT=1 the only valid id is 0, so to *rename* we'd produce an
    // out-of-range id; tests that exercise rename for TBs need TB_COUNT >= 2.
    let mut kernel_2tb: Kernel<2, 1, 1> = Kernel::new(baseline_211());
    let mut new_config = baseline_211();
    new_config.tb_defs[1] = TripleBufferDef {
        // was TripleBufferId(1) — rename to a fresh id not present in the old config
        id: TripleBufferId(7),
        buffer_capacity: 256,
    };

    assert!(matches!(
        kernel_2tb.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));

    // Also exercise the single-TB rename path: even with TB_COUNT=1, swapping
    // the user TB's id triggers SchemaMismatch (validate_tb_defs_compatibility
    // can't find the old id in the new defs).
    let mut new_config_111 = baseline_111();
    new_config_111.tb_defs[0] = TripleBufferDef {
        id: TripleBufferId(3),
        buffer_capacity: TB0_CAP,
    };
    assert!(matches!(
        kernel.grow(new_config_111),
        Err(KernelError::SchemaMismatch)
    ));
}

#[test]
fn grow_rejects_renamed_store_id() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.store_defs[0] = EntryStoreDef::new(
        EntryStoreId(5), // was EntryStoreId(0)
        TripleBufferId::DEFAULT,
        EntryStoreConfig {
            core_stride: STORE_CORE,
            meta_stride: STORE_META,
            attr_stride: STORE_ATTR,
            capacity: STORE_CAP,
        },
    );

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

#[test]
fn grow_rejects_renamed_lut_id() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.lut_defs[0] = LutDef::new(
        LutId(9), // was LutId(0)
        TripleBufferId::DEFAULT,
        LUT0_SIZE,
    );

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::SchemaMismatch)
    ));
}

// =========================================================
// POSITIVE — load-bearing controls
// =========================================================

#[test]
fn grow_with_only_capacity_increases_succeeds() {
    // Increase every capacity dimension; leave every schema field alone.
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.mem_metadata_size = MEM_METADATA_SIZE * 2;
    new_config.network_config.node_capacity = NODE_CAP * 2;
    new_config.network_config.synapse_capacity = SYNAPSE_CAP * 2;
    new_config.tb_defs[0].buffer_capacity = TB0_CAP * 2;
    new_config.store_defs[0] = EntryStoreDef::new(
        EntryStoreId(0),
        TripleBufferId::DEFAULT,
        EntryStoreConfig {
            core_stride: STORE_CORE,
            meta_stride: STORE_META,
            attr_stride: STORE_ATTR,
            capacity: STORE_CAP * 2,
        },
    );
    new_config.lut_defs[0] = LutDef::new(LutId(0), TripleBufferId::DEFAULT, LUT0_SIZE * 2);

    kernel.grow(new_config).expect("capacity-only grow must succeed");
    assert_eq!(kernel.node_capacity(), (NODE_CAP * 2) as usize);
    assert_eq!(kernel.synapse_capacity(), (SYNAPSE_CAP * 2) as usize);
}

#[test]
fn grow_with_no_changes_succeeds() {
    // grow accepts equal-or-greater on every dimension. An identical config
    // is the strongest no-op control: every comparison must be `>=`, not `>`.
    let mut kernel = Kernel111::new(baseline_111());
    kernel.grow(baseline_111()).expect("identical-config grow must succeed");
    assert_eq!(kernel.node_capacity(), NODE_CAP as usize);
}

// =========================================================
// CAPACITY SHRINK — InsufficientCapacity
// =========================================================
//
// Each one shrinks exactly one capacity field, with every schema field
// left untouched, and asserts `Err(KernelError::InsufficientCapacity)`.

#[test]
fn grow_rejects_shrunk_mem_metadata_size() {
    // Need a kernel whose mem_metadata_size is > 1 so we can request 1 < that.
    let mut config = baseline_111();
    config.mem_metadata_size = 4;
    let mut kernel = Kernel111::new(config);

    let mut new_config = baseline_111();
    new_config.mem_metadata_size = 2;

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::InsufficientCapacity)
    ));
}

#[test]
fn grow_rejects_shrunk_node_capacity() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.network_config.node_capacity = NODE_CAP / 2;

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::InsufficientCapacity)
    ));
}

#[test]
fn grow_rejects_shrunk_synapse_capacity() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.network_config.synapse_capacity = SYNAPSE_CAP / 2;

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::InsufficientCapacity)
    ));
}

#[test]
fn grow_rejects_shrunk_per_tb_buffer_capacity() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.tb_defs[0].buffer_capacity = TB0_CAP / 2;

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::InsufficientCapacity)
    ));
}

#[test]
fn grow_rejects_shrunk_per_store_capacity() {
    let mut kernel = Kernel111::new(baseline_111());
    let mut new_config = baseline_111();
    new_config.store_defs[0] = EntryStoreDef::new(
        EntryStoreId(0),
        TripleBufferId::DEFAULT,
        EntryStoreConfig {
            core_stride: STORE_CORE,
            meta_stride: STORE_META,
            attr_stride: STORE_ATTR,
            capacity: STORE_CAP / 2,
        },
    );

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::InsufficientCapacity)
    ));
}

#[test]
fn grow_rejects_shrunk_per_lut_size() {
    // Need a baseline where LUT size is > 1 so we can shrink it below the old size.
    let mut config = baseline_111();
    config.lut_defs[0] = LutDef::new(LutId(0), TripleBufferId::DEFAULT, 4);
    let mut kernel = Kernel111::new(config);

    let mut new_config = baseline_111();
    new_config.lut_defs[0] = LutDef::new(LutId(0), TripleBufferId::DEFAULT, 2);

    assert!(matches!(
        kernel.grow(new_config),
        Err(KernelError::InsufficientCapacity)
    ));
}

// =========================================================
// FAILED-GROW LEAVES KERNEL INTACT
// =========================================================

#[test]
fn failed_grow_leaves_kernel_unchanged() {
    // After a SchemaMismatch / InsufficientCapacity rejection, the kernel
    // must still be fully usable at its original capacity.
    let mut kernel = Kernel111::new(baseline_111());

    let mut bad_config = baseline_111();
    bad_config.network_config.node_meta_stride = NODE_META + 1;
    assert!(matches!(
        kernel.grow(bad_config),
        Err(KernelError::SchemaMismatch)
    ));

    let mut shrink_config = baseline_111();
    shrink_config.network_config.node_capacity = NODE_CAP / 2;
    assert!(matches!(
        kernel.grow(shrink_config),
        Err(KernelError::InsufficientCapacity)
    ));

    // Original kernel is still functional with the original capacity.
    assert_eq!(kernel.node_capacity(), NODE_CAP as usize);
    let n = kernel.insert_node(7).unwrap();
    assert_eq!(kernel.get_node(n).get_kind(), 7);
}
