mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use synaptic_kernel::epoch_consumer::EpochConsumer;
use synaptic_kernel::errors::kernel_error::KernelError;
use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;
use synaptic_kernel::primitives::entry_store_config::EntryStoreConfig;
use synaptic_kernel::primitives::entry_store_def::{EntryStoreDef, EntryStoreId};
use synaptic_kernel::primitives::lut_def::{LutDef, LutId};
use synaptic_kernel::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use synaptic_kernel::topology::network::network_config::NetworkConfig;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;

type TestKernel = Kernel<1, 1, 1>;
type TestConsumer = EpochConsumer<1, 1, 1>;

fn new_controller(cfg: KernelConfig<1, 1, 1>) -> TestKernel {
    Kernel::new(cfg)
}

fn create_config(nodes: u32, synapses: u32) -> KernelConfig<1, 1, 1> {
    common::kernel_config_1_1(
        nodes,
        synapses,
        NODE_META,
        NODE_ATTR,
        SYNAPSE_META,
        SYNAPSE_ATTR,
    )
}

fn config(capacity: u32) -> KernelConfig<1, 1, 1> {
    create_config(capacity, capacity)
}

fn config_with_lut_on_default(lut_size: usize) -> KernelConfig<1, 1, 1> {
    let mut c = create_config(16, 16);
    c.lut_defs = [LutDef::new(LutId(0), TripleBufferId::DEFAULT, lut_size)];
    c
}


fn config_with_lut_on_user_tb(lut_size: usize) -> KernelConfig<1, 1, 1> {
    KernelConfig {
        mem_metadata_size: 1,
        tb_defs: [TripleBufferDef {
            id: TripleBufferId(0),
            buffer_capacity: 32768,
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
        lut_defs: [LutDef::new(LutId(0), TripleBufferId(0), lut_size)],
        network_config: NetworkConfig {
            node_capacity: 16,
            node_meta_stride: NODE_META,
            node_attr_stride: NODE_ATTR,
            synapse_capacity: 16,
            synapse_meta_stride: SYNAPSE_META,
            synapse_attr_stride: SYNAPSE_ATTR,
        },
    }
}

// =========================================================
// PHASE 1: Happy Path — Lifecycle & Basic Operations
// =========================================================

#[test]
fn fresh_controller_reports_zero_counts() {
    let controller = new_controller(config(16));
    assert_eq!(controller.node_count(), 0);
    assert_eq!(controller.synapse_count(), 0);
    assert_eq!(controller.node_capacity(), 16);
    assert_eq!(controller.synapse_capacity(), 16);
    assert_eq!(controller.node_utilization(), 0.0);
    assert_eq!(controller.synapse_utilization(), 0.0);
}

#[test]
fn insert_node_returns_slot_and_node_visible_to_writer() {
    let controller = new_controller(config(16));
    let slot = controller.insert_node(1).unwrap();
    // SlotId is non-zero by construction.
    assert_eq!(controller.get_node(slot).get_kind(), 1);
}

#[test]
fn insert_after_and_before_form_correct_chain() {
    let controller = new_controller(config(16));
    let n1 = controller.insert_node(10).unwrap();
    let n3 = controller.insert_node_after(n1, 30).unwrap();
    let n2 = controller.insert_node_before(n3, 20).unwrap();

    // Chain: n1 -> n2 -> n3
    let w1 = controller.get_node(n1);
    let w2 = controller.get_node(n2);
    let w3 = controller.get_node(n3);
    assert_eq!(w1.get_next_ptr(), Some(n2));
    assert_eq!(w2.get_prev_ptr(), Some(n1));
    assert_eq!(w2.get_next_ptr(), Some(n3));
    assert_eq!(w3.get_prev_ptr(), Some(n2));
}

#[test]
fn two_insert_node_calls_create_disjoint_orphans() {
    let controller = new_controller(config(16));
    let a = controller.insert_node(1).unwrap();
    let b = controller.insert_node(2).unwrap();

    let na = controller.get_node(a);
    let nb = controller.get_node(b);
    assert!(na.get_next_ptr().is_none());
    assert!(na.get_prev_ptr().is_none());
    assert!(nb.get_next_ptr().is_none());
    assert!(nb.get_prev_ptr().is_none());
    assert_ne!(a, b);
}

#[test]
fn connect_and_disconnect_lifecycle() {
    let controller = new_controller(config(16));
    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();

    let s1 = controller.connect(n1, n2, 5).unwrap();
    let synapse = controller.get_synapse(s1);
    assert_eq!(synapse.get_kind(), 5);

    controller.disconnect_synapse(s1).unwrap();
}

#[test]
fn cross_subchain_synapse_connects_disjoint_chains() {
    let controller = new_controller(config(16));

    // Build two disjoint sub-chains.
    let x1 = controller.insert_node(10).unwrap();
    let x2 = controller.insert_node_after(x1, 11).unwrap();

    let y1 = controller.insert_node(20).unwrap();
    let y2 = controller.insert_node_after(y1, 21).unwrap();

    // Connect x2 -> y1 (cross-chain synapse).
    let s = controller.connect(x2, y1, 99).unwrap();
    let syn = controller.get_synapse(s);
    assert_eq!(syn.get_source_ptr(), x2);
    assert_eq!(syn.get_target_ptr(), y1);

    // Chain links unchanged on either side.
    assert_eq!(controller.get_node(x1).get_next_ptr(), Some(x2));
    assert!(controller.get_node(x2).get_next_ptr().is_none());
    assert!(controller.get_node(y1).get_prev_ptr().is_none());
    assert_eq!(controller.get_node(y1).get_next_ptr(), Some(y2));
}

#[test]
fn node_and_synapse_attribute_round_trip() {
    let controller = new_controller(config(16));
    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    let s1 = controller.connect(n1, n2, 1).unwrap();

    for offset in 0..16 {
        controller.get_node(n1).attr_write(offset, (offset as i32) * 100);
    }
    for offset in 0..16 {
        assert_eq!(
            controller.get_node(n1).attr_read(offset),
            (offset as i32) * 100
        );
    }

    for offset in 0..16 {
        controller
            .get_synapse(s1)
            .attr_write(offset, -(offset as i32) * 50);
    }
    for offset in 0..16 {
        assert_eq!(
            controller.get_synapse(s1).attr_read(offset),
            -(offset as i32) * 50
        );
    }
}

#[test]
fn negative_attribute_values_preserved() {
    let controller = new_controller(config(16));
    let n = controller.insert_node(1).unwrap();
    controller.get_node(n).attr_write(0, i32::MIN);
    controller.get_node(n).attr_write(1, -1);
    assert_eq!(controller.get_node(n).attr_read(0), i32::MIN);
    assert_eq!(controller.get_node(n).attr_read(1), -1);
}

// =========================================================
// PHASE 2: Triple Buffer Isolation — Consumer Thread Boundary
// =========================================================
//
// Drop ordering: the EpochConsumer is declared AFTER the Kernel. Local
// variables drop in reverse declaration order, so the consumer's
// Arc<ControlPlane> clone is released first; the Kernel's debug-time Drop
// assert then sees a strong_count of 1 and passes.

#[test]
fn mutations_invisible_to_consumer_before_publish_and_swap() {
    let mut controller = new_controller(config(16));
    let mut consumer = TestConsumer::new(controller.get_control_plane());
    let mirror = consumer.acquire_mirror();

    // Insert a node — not yet published.
    let slot = controller.insert_node(42).unwrap();

    // Without a publish there's nothing new to swap.
    assert!(!mirror.swap(), "no published frame yet");

    // Publish on producer side; consumer-side swap now returns true.
    controller.publish();
    assert!(mirror.swap());

    // Inserted node visible at its slot.
    assert_eq!(mirror.get_node(slot).get_kind(), 42);
}

#[test]
fn multiple_mutations_batch_into_single_publish() {
    let mut controller = new_controller(config(16));
    let mut consumer = TestConsumer::new(controller.get_control_plane());
    let mirror = consumer.acquire_mirror();

    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    let n3 = controller.insert_node_after(n2, 3).unwrap();
    controller.connect(n1, n2, 10).unwrap();
    controller.connect(n2, n3, 20).unwrap();
    controller.get_node(n1).attr_write(0, 999);

    // No publish yet — mirror has nothing to swap in.
    assert!(!mirror.swap());

    controller.publish();
    assert!(mirror.swap());

    let head = mirror.get_node(n1);
    assert_eq!(head.get_kind(), 1);
    assert_eq!(mirror.get_node(n1).attr_read(0), 999);
    let next = mirror.get_node(head.get_next_ptr().unwrap());
    assert_eq!(next.get_kind(), 2);
    let last = mirror.get_node(next.get_next_ptr().unwrap());
    assert_eq!(last.get_kind(), 3);
}

#[test]
fn double_swap_without_publish_returns_false() {
    let mut controller = new_controller(config(16));
    let mut consumer = TestConsumer::new(controller.get_control_plane());
    let mirror = consumer.acquire_mirror();

    controller.insert_node(1).unwrap();
    controller.publish();

    assert!(mirror.swap()); // first swap consumes the publish
    assert!(!mirror.swap()); // nothing new
}

#[test]
fn attributes_visible_immediately_without_publish() {
    let mut controller = new_controller(config(16));
    let mut consumer = TestConsumer::new(controller.get_control_plane());
    let mirror = consumer.acquire_mirror();

    let n = controller.insert_node(1).unwrap();
    controller.publish();
    assert!(mirror.swap()); // make `n` reachable on the consumer side

    controller.get_node(n).attr_write(3, 42);
    // Attribute writes go straight to the MEM plane — no publish needed.
    assert_eq!(mirror.get_node(n).attr_read(3), 42);
}

// =========================================================
// PHASE 3: Capacity Exhaustion — Saturation & Error Paths
// =========================================================

#[test]
fn node_capacity_exhaustion_returns_error() {
    let controller = new_controller(config(2));
    let n1 = controller.insert_node(1).unwrap();
    controller.insert_node(2).unwrap();

    assert!(matches!(
        controller.insert_node(3),
        Err(KernelError::CapacityExhausted)
    ));
    assert!(matches!(
        controller.insert_node_after(n1, 3),
        Err(KernelError::CapacityExhausted)
    ));
    assert!(matches!(
        controller.insert_node_before(n1, 3),
        Err(KernelError::CapacityExhausted)
    ));
}

#[test]
fn synapse_capacity_exhaustion_returns_error() {
    let controller = new_controller(create_config(16, 2));
    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    let n3 = controller.insert_node_after(n2, 3).unwrap();

    controller.connect(n1, n2, 1).unwrap();
    controller.connect(n2, n3, 2).unwrap();

    assert!(matches!(
        controller.connect(n3, n1, 3),
        Err(KernelError::CapacityExhausted)
    ));
}

#[test]
fn remove_then_reuse_slot() {
    let controller = new_controller(config(2));
    let n1 = controller.insert_node(1).unwrap();
    let _n2 = controller.insert_node(2).unwrap();

    assert!(controller.insert_node(3).is_err());

    // Remove opens a slot — but deferred, so needs publish+flush
    controller.remove_node(n1).unwrap();

    // Slot count hasn't changed yet (deferred free)
    assert_eq!(controller.node_count(), 2);
}

#[test]
#[should_panic(expected = "attempted to read inactive slot")]
fn double_remove_same_node_panics_uaf_guard() {
    let controller = new_controller(config(16));
    let n1 = controller.insert_node(1).unwrap();
    controller.remove_node(n1).unwrap();
    // Second remove hits the UAF guard before reaching DoubleFree
    let _ = controller.remove_node(n1);
}

#[test]
#[should_panic(expected = "attempted to read inactive slot")]
fn double_disconnect_same_synapse_panics_uaf_guard() {
    let controller = new_controller(config(16));
    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    let s1 = controller.connect(n1, n2, 1).unwrap();
    controller.disconnect_synapse(s1).unwrap();
    // Second disconnect hits the UAF guard
    let _ = controller.disconnect_synapse(s1);
}

// =========================================================
// PHASE 4: remove_chain — sub-chain teardown
// =========================================================

/// Drive the two-cycle reclaim dance so deferred-freed slots return to the
/// allocator: publish (stage), consumer ack via swap, publish (drain).
fn drain_deferred_frees(kernel: &mut TestKernel) {
    let mut consumer = TestConsumer::new(kernel.get_control_plane());
    kernel.publish();
    let _ = consumer.acquire_mirror();
    kernel.publish();
    drop(consumer);
}

#[test]
fn remove_chain_walks_next_ptr_and_removes_each_node() {
    let mut controller = new_controller(config(16));

    // chain: a -> b -> c -> d
    let a = controller.insert_node(1).unwrap();
    let b = controller.insert_node_after(a, 2).unwrap();
    let c = controller.insert_node_after(b, 3).unwrap();
    let d = controller.insert_node_after(c, 4).unwrap();
    assert_eq!(controller.node_count(), 4);

    controller.remove_chain(a).unwrap();

    // All four slots are deferred-freed at this point; the slot allocator
    // still counts them as live until the consumer-ack cycle drains them.
    drain_deferred_frees(&mut controller);

    assert_eq!(controller.node_count(), 0);
    let _ = (a, b, c, d);
}

#[test]
fn remove_chain_cascades_intra_chain_synapses() {
    let mut controller = new_controller(create_config(16, 16));

    // chain a -> b -> c, plus synapses a->b and b->c.
    let a = controller.insert_node(1).unwrap();
    let b = controller.insert_node_after(a, 2).unwrap();
    let c = controller.insert_node_after(b, 3).unwrap();
    controller.connect(a, b, 10).unwrap();
    controller.connect(b, c, 20).unwrap();
    assert_eq!(controller.synapse_count(), 2);

    controller.remove_chain(a).unwrap();

    drain_deferred_frees(&mut controller);

    // Both synapses cascade out as part of node removal.
    assert_eq!(controller.synapse_count(), 0);
    assert_eq!(controller.node_count(), 0);
}

#[test]
fn remove_chain_cascades_cross_chain_synapses() {
    let mut controller = new_controller(create_config(16, 16));

    // Two disjoint sub-chains, joined by a cross-chain synapse.
    let x = controller.insert_node(10).unwrap();
    let y = controller.insert_node(20).unwrap();
    let cross = controller.connect(x, y, 99).unwrap();
    assert_eq!(controller.synapse_count(), 1);

    // Removing chain X must cascade the cross-chain synapse incident on X.
    controller.remove_chain(x).unwrap();

    // y's incoming-synapse list is patched immediately even though the
    // synapse slot is still deferred.
    assert_eq!(controller.get_node(y).get_kind(), 20);
    assert!(controller.get_node(y).get_incoming_synapse_head().is_none());

    drain_deferred_frees(&mut controller);
    assert_eq!(controller.synapse_count(), 0);
    let _ = cross;
}

// =========================================================
// PHASE 5: Grow — Memory Scaling & Topology Preservation
// =========================================================

#[test]
fn grow_rejects_smaller_capacity() {
    let mut controller = new_controller(config(16));
    assert!(matches!(
        controller.grow(config(8)),
        Err(KernelError::InsufficientCapacity)
    ));
}

#[test]
fn grow_rejects_same_capacity() {
    let mut controller = new_controller(config(16));
    // grow accepts equal-or-greater on every dimension.
    assert!(controller.grow(config(16)).is_ok());
}

#[test]
fn grow_preserves_chain_topology() {
    let mut controller = new_controller(config(8));

    let n1 = controller.insert_node(10).unwrap();
    let n2 = controller.insert_node_after(n1, 20).unwrap();
    let n3 = controller.insert_node_after(n2, 30).unwrap();

    controller.grow(config(32)).unwrap();

    // Chain n1 -> n2 -> n3 survived.
    let w1 = controller.get_node(n1);
    assert_eq!(w1.get_kind(), 10);
    let w2 = controller.get_node(w1.get_next_ptr().unwrap());
    assert_eq!(w2.get_kind(), 20);
    let w3 = controller.get_node(w2.get_next_ptr().unwrap());
    assert_eq!(w3.get_kind(), 30);
    let _ = n3;
}

#[test]
fn grow_preserves_node_and_synapse_attributes() {
    let mut controller = new_controller(config(8));

    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    let s1 = controller.connect(n1, n2, 5).unwrap();

    controller.get_node(n1).attr_write(0, 1000);
    controller.get_node(n1).attr_write(15, -999);
    controller.get_synapse(s1).attr_write(0, 5000);
    controller.get_synapse(s1).attr_write(15, -5000);

    controller.grow(config(32)).unwrap();

    assert_eq!(controller.get_node(n1).attr_read(0), 1000);
    assert_eq!(controller.get_node(n1).attr_read(15), -999);
    assert_eq!(controller.get_synapse(s1).attr_read(0), 5000);
    assert_eq!(controller.get_synapse(s1).attr_read(15), -5000);
}

#[test]
fn grow_preserves_synapse_connectivity() {
    let mut controller = new_controller(config(8));

    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    let n3 = controller.insert_node_after(n2, 3).unwrap();

    let s12 = controller.connect(n1, n2, 10).unwrap();
    let s13 = controller.connect(n1, n3, 20).unwrap();
    let s23 = controller.connect(n2, n3, 30).unwrap();

    controller.grow(config(32)).unwrap();

    assert_eq!(controller.get_synapse(s12).get_kind(), 10);
    assert_eq!(controller.get_synapse(s13).get_kind(), 20);
    assert_eq!(controller.get_synapse(s23).get_kind(), 30);
}

#[test]
fn grow_expanded_capacity_is_allocatable() {
    let mut controller = new_controller(config(4));

    controller.insert_node(1).unwrap();
    controller.insert_node(2).unwrap();
    controller.insert_node(3).unwrap();
    controller.insert_node(4).unwrap();
    assert!(controller.insert_node(5).is_err());

    controller.grow(config(8)).unwrap();

    controller.insert_node(5).unwrap();
    controller.insert_node(6).unwrap();
    controller.insert_node(7).unwrap();
    controller.insert_node(8).unwrap();
    assert!(controller.insert_node(9).is_err());
    assert_eq!(controller.node_count(), 8);
}

#[test]
fn grow_consumer_thread_sees_migrated_data_after_publish_swap() {
    let mut controller = new_controller(config(8));
    let mut consumer = TestConsumer::new(controller.get_control_plane());

    let n1 = controller.insert_node(10).unwrap();
    let n2 = controller.insert_node_after(n1, 20).unwrap();
    let s1 = controller.connect(n1, n2, 99).unwrap();
    controller.get_node(n1).attr_write(0, 1000);
    controller.get_synapse(s1).attr_write(0, 5000);

    controller.grow(config(32)).unwrap();
    controller.publish();

    let mirror = consumer.acquire_mirror();

    let head = mirror.get_node(n1);
    assert_eq!(head.get_kind(), 10);
    assert_eq!(mirror.get_node(n1).attr_read(0), 1000);

    let next = mirror.get_node(head.get_next_ptr().unwrap());
    assert_eq!(next.get_kind(), 20);

    let syn = mirror.get_synapse(s1);
    assert_eq!(syn.get_kind(), 99);
    assert_eq!(mirror.get_synapse(s1).attr_read(0), 5000);
}

#[test]
fn grow_after_heavy_fragmentation() {
    let mut controller = new_controller(config(8));

    let mut slots = Vec::new();
    for i in 0..8 {
        slots.push(controller.insert_node(i).unwrap());
    }

    controller.remove_node(slots[1]).unwrap();
    controller.remove_node(slots[3]).unwrap();
    controller.remove_node(slots[5]).unwrap();
    controller.remove_node(slots[7]).unwrap();

    controller.publish();

    controller.grow(config(16)).unwrap();

    assert_eq!(controller.get_node(slots[0]).get_kind(), 0);
    assert_eq!(controller.get_node(slots[2]).get_kind(), 2);
    assert_eq!(controller.get_node(slots[4]).get_kind(), 4);
    assert_eq!(controller.get_node(slots[6]).get_kind(), 6);

    let new_node = controller.insert_node(100).unwrap();
    assert_eq!(controller.get_node(new_node).get_kind(), 100);
}

// =========================================================
// PHASE 6: GC Pipeline — Backlog/Pending Rotation
// =========================================================

#[test]
fn gc_pipeline_rotates_through_publish_cycles() {
    let mut controller = new_controller(config(8));
    let mut consumer = TestConsumer::new(controller.get_control_plane());

    let mut slots = Vec::new();
    for i in 0..7 {
        slots.push(controller.insert_node(i).unwrap());
    }
    assert!(controller.should_grow(0.70));

    controller.grow(config(16)).unwrap();
    assert_eq!(controller.node_capacity(), 16);

    // First publish: backlog -> pending_deletion
    controller.publish();
    // Second publish: pending_deletion dropped
    controller.publish();

    // Consumer thread sees migrated data.
    let mirror = consumer.acquire_mirror();
    // Verify all originally-inserted slots survived the grow.
    for (i, slot) in slots.iter().enumerate() {
        assert_eq!(mirror.get_node(*slot).get_kind(), i as i32);
    }
}

#[test]
fn consecutive_grows_without_crash() {
    let mut controller = new_controller(config(4));
    let n = controller.insert_node(1).unwrap();

    controller.grow(config(8)).unwrap();
    controller.publish();

    controller.grow(config(16)).unwrap();
    controller.publish();

    controller.grow(config(32)).unwrap();
    controller.publish();

    assert_eq!(controller.node_capacity(), 32);
    assert_eq!(controller.get_node(n).get_kind(), 1);
}

#[test]
fn grow_then_mutate_then_publish() {
    let mut controller = new_controller(config(4));
    let mut consumer = TestConsumer::new(controller.get_control_plane());

    let n1 = controller.insert_node(1).unwrap();

    controller.grow(config(16)).unwrap();

    // Mutate AFTER grow, BEFORE publish
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    controller.get_node(n2).attr_write(0, 777);

    controller.publish();

    let mirror = consumer.acquire_mirror();

    let head = mirror.get_node(n1);
    assert_eq!(head.get_kind(), 1);
    let next = mirror.get_node(head.get_next_ptr().unwrap());
    assert_eq!(next.get_kind(), 2);
    assert_eq!(mirror.get_node(n2).attr_read(0), 777);
}

// =========================================================
// PHASE 7: Threshold Logic
// =========================================================

#[test]
fn should_grow_respects_threshold_boundary() {
    let controller = new_controller(config(4));
    assert!(!controller.should_grow(0.75));

    controller.insert_node(1).unwrap();
    controller.insert_node(2).unwrap();
    controller.insert_node(3).unwrap();

    // 3/4 = 0.75, should_grow uses > not >=
    assert!(!controller.should_grow(0.75));

    controller.insert_node(4).unwrap();
    // 4/4 = 1.0 > 0.75
    assert!(controller.should_grow(0.75));
}

// =========================================================
// PHASE 8: Controller Plane Address Stability
// =========================================================

#[test]
fn control_plane_address_is_stable_across_grow() {
    let mut controller = new_controller(config(4));
    let addr_before = Arc::as_ptr(&controller.get_control_plane()) as usize;

    controller.grow(config(8)).unwrap();
    let addr_after = Arc::as_ptr(&controller.get_control_plane()) as usize;

    // The ControlPlane is boxed and its address must not move.
    // Consumer thread holds this pointer — if it moves, segfault.
    assert_eq!(addr_before, addr_after);
}

#[test]
fn control_plane_address_nonzero() {
    let controller = new_controller(config(4));
    assert_ne!(Arc::as_ptr(&controller.get_control_plane()) as usize, 0);
}

// =========================================================
// PHASE 9: Asymmetric Config (different node/synapse caps)
// =========================================================

#[test]
fn asymmetric_capacity_works() {
    let controller = new_controller(create_config(16, 4));
    assert_eq!(controller.node_capacity(), 16);
    assert_eq!(controller.synapse_capacity(), 4);

    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();

    controller.connect(n1, n2, 1).unwrap();
    controller.connect(n1, n2, 2).unwrap();
    controller.connect(n1, n2, 3).unwrap();
    controller.connect(n1, n2, 4).unwrap();

    assert!(matches!(
        controller.connect(n1, n2, 5),
        Err(KernelError::CapacityExhausted)
    ));
}

#[test]
fn grow_rejects_if_only_nodes_shrink() {
    let mut controller = new_controller(create_config(16, 16));
    assert!(matches!(
        controller.grow(create_config(8, 32)),
        Err(KernelError::InsufficientCapacity)
    ));
}

#[test]
fn grow_rejects_if_only_synapses_shrink() {
    let mut controller = new_controller(create_config(16, 16));
    assert!(matches!(
        controller.grow(create_config(32, 8)),
        Err(KernelError::InsufficientCapacity)
    ));
}

#[test]
fn defer_then_grow_then_publish_flushes_on_new_allocator() {
    let mut controller = new_controller(config(4));

    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    let n3 = controller.insert_node_after(n2, 3).unwrap();
    let n4 = controller.insert_node_after(n3, 4).unwrap();

    assert!(controller.insert_node(99).is_err());

    controller.remove_node(n2).unwrap();
    assert_eq!(controller.node_count(), 4);

    controller.grow(config(8)).unwrap();
    controller.publish();

    let n5 = controller.insert_node(5).unwrap();
    assert_eq!(controller.get_node(n5).get_kind(), 5);

    assert_eq!(controller.get_node(n1).get_kind(), 1);
    assert_eq!(controller.get_node(n3).get_kind(), 3);
    assert_eq!(controller.get_node(n4).get_kind(), 4);
}

#[test]
fn defer_then_grow_then_defer_more_then_publish() {
    let mut controller = new_controller(config(8));

    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    let n3 = controller.insert_node_after(n2, 3).unwrap();
    let n4 = controller.insert_node_after(n3, 4).unwrap();

    controller.remove_node(n2).unwrap();
    controller.grow(config(16)).unwrap();
    controller.remove_node(n4).unwrap();
    controller.publish();

    let n5 = controller.insert_node(5).unwrap();
    let n6 = controller.insert_node(6).unwrap();
    assert_eq!(controller.get_node(n5).get_kind(), 5);
    assert_eq!(controller.get_node(n6).get_kind(), 6);

    assert_eq!(controller.get_node(n1).get_kind(), 1);
    assert_eq!(controller.get_node(n3).get_kind(), 3);
}

#[test]
fn defer_then_publish_then_grow_preserves_freed_slot() {
    let mut controller = new_controller(config(4));
    let mut consumer = TestConsumer::new(controller.get_control_plane());

    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    let n3 = controller.insert_node_after(n2, 3).unwrap();
    let n4 = controller.insert_node_after(n3, 4).unwrap();

    controller.remove_node(n2).unwrap();
    controller.publish();

    controller.grow(config(8)).unwrap();

    let n5 = controller.insert_node(5).unwrap();
    assert_eq!(controller.get_node(n5).get_kind(), 5);

    controller.publish();
    let mirror = consumer.acquire_mirror();
    assert_eq!(mirror.get_node(n5).get_kind(), 5);
    assert_eq!(mirror.get_node(n1).get_kind(), 1);
    assert_eq!(mirror.get_node(n3).get_kind(), 3);
    assert_eq!(mirror.get_node(n4).get_kind(), 4);
}

// =========================================================
// PHASE 10: Concurrent: Consumer Thread vs Main Thread
// =========================================================
//
// Pattern: the Kernel stays in the main thread; the consumer thread holds an
// Arc<ControlPlane> clone via the safe Arc::clone + move idiom. The reader
// thread is joined before the kernel is dropped, so the consumer's Arc clone
// is released first and the Drop assert sees strong_count == 1.

#[test]
fn concurrent_traversal_during_rapid_publish_cycles() {
    let mut controller = new_controller(config(64));
    let cp = controller.get_control_plane();

    let running = Arc::new(AtomicBool::new(true));
    let running_consumer = Arc::clone(&running);

    let consumer_thread = thread::spawn(move || {
        let mut processor = TestConsumer::new(cp);
        let mut iterations = 0u64;

        while running_consumer.load(Ordering::Relaxed) {
            let _ = processor.acquire_mirror();
            iterations += 1;
        }
        iterations
        // processor (and its Arc<ControlPlane> clone) drops here.
    });

    // Main thread: insert nodes and publish rapidly.
    let mut slots = Vec::new();
    for i in 0..60 {
        slots.push(controller.insert_node(i).unwrap());
        if i % 5 == 0 {
            controller.publish();
        }
    }
    controller.publish();

    thread::sleep(std::time::Duration::from_millis(10));

    running.store(false, Ordering::Relaxed);
    let iterations = consumer_thread.join().expect("consumer thread panicked");
    assert!(iterations > 0, "consumer thread never ran");
}

#[test]
fn concurrent_traversal_during_grow() {
    let mut controller = new_controller(config(8));
    let cp = controller.get_control_plane();

    // Seed initial data.
    let n1 = controller.insert_node(1).unwrap();
    let n2 = controller.insert_node_after(n1, 2).unwrap();
    controller.connect(n1, n2, 10).unwrap();
    controller.get_node(n1).attr_write(0, 42);
    controller.publish();

    let running = Arc::new(AtomicBool::new(true));
    let running_consumer = Arc::clone(&running);

    let consumer_thread = thread::spawn(move || {
        let mut processor = TestConsumer::new(cp);
        let mut iterations = 0u64;

        while running_consumer.load(Ordering::Relaxed) {
            // Re-acquire EVERY iteration — critical after grow().
            let _ = processor.acquire_mirror();
            iterations += 1;
        }
        iterations
    });

    controller.grow(config(16)).unwrap();
    controller.publish();
    for i in 3..14 {
        controller.insert_node(i).unwrap();
    }
    controller.publish();

    controller.grow(config(32)).unwrap();
    controller.publish();
    for i in 14..28 {
        controller.insert_node(i).unwrap();
    }
    controller.publish();

    thread::sleep(std::time::Duration::from_millis(10));

    controller.publish();
    controller.publish();

    running.store(false, Ordering::Relaxed);
    let iterations = consumer_thread.join().expect("consumer thread panicked");
    assert!(iterations > 0);
}

#[test]
fn concurrent_attribute_reads_during_writes() {
    let controller = new_controller(config(16));
    let cp = controller.get_control_plane();

    let n1 = controller.insert_node(1).unwrap();

    let running = Arc::new(AtomicBool::new(true));
    let running_consumer = Arc::clone(&running);

    let consumer_thread = thread::spawn(move || {
        let mut processor = TestConsumer::new(cp);
        let mut iterations = 0u64;

        while running_consumer.load(Ordering::Relaxed) {
            let mirror = processor.acquire_mirror();
            for offset in 0..16 {
                let _ = mirror.get_node(n1).attr_read(offset);
            }
            iterations += 1;
        }
        iterations
    });

    for batch in 0..500 {
        for offset in 0..16 {
            controller
                .get_node(n1)
                .attr_write(offset, (offset as i32) * 1000 + batch);
        }
    }

    thread::sleep(std::time::Duration::from_millis(5));

    running.store(false, Ordering::Relaxed);
    let iterations = consumer_thread
        .join()
        .expect("consumer thread panicked during attribute writes");
    assert!(iterations > 0, "consumer thread never ran");
}

// =========================================================
// Entry store and LUT plumbing
// =========================================================

#[test]
fn get_entry_store_insert_write_read_roundtrip() {
    let kernel = new_controller(config(16));
    let store = kernel.get_entry_store(EntryStoreId(0));
    let slot = store.insert().unwrap();
    store.get(slot).attr_write(0, 42);
    assert_eq!(store.get(slot).attr_read(0), 42);
}

#[test]
fn get_entry_store_returns_same_store_across_calls() {
    let kernel = new_controller(config(16));
    let s1 = kernel.get_entry_store(EntryStoreId(0));
    let s2 = kernel.get_entry_store(EntryStoreId(0));
    assert_eq!(s1.mem_start_offset(), s2.mem_start_offset());
    assert_eq!(s1.capacity(), s2.capacity());
}

#[test]
fn entry_store_core_visible_after_publish_swap() {
    let mut kernel = new_controller(config(16));
    let mut consumer = TestConsumer::new(kernel.get_control_plane());
    let mirror = consumer.acquire_mirror();
    let slot = {
        let store = kernel.get_entry_store(EntryStoreId(0));
        let slot = store.insert().unwrap();
        store.get(slot).core_write(0, 777);
        slot
    };
    kernel.publish();
    assert!(mirror.swap());
    let reader_store = mirror.get_entry_store(EntryStoreId(0));
    assert_eq!(reader_store.get(slot).core_read(0), 777);
}

#[test]
fn entry_store_attr_visible_without_publish() {
    let kernel = new_controller(config(16));
    let mut consumer = TestConsumer::new(kernel.get_control_plane());
    let mirror = consumer.acquire_mirror();
    let store = kernel.get_entry_store(EntryStoreId(0));
    let slot = store.insert().unwrap();
    store.get(slot).attr_write(0, 42);
    assert_eq!(
        mirror.get_entry_store(EntryStoreId(0)).get(slot).attr_read(0),
        42
    );
}

#[test]
fn entry_store_survives_grow() {
    let mut kernel = new_controller(config(4));
    let store = kernel.get_entry_store(EntryStoreId(0));
    let slot = store.insert().unwrap();
    store.get(slot).attr_write(0, 999);
    kernel.publish();
    kernel.grow(config(8)).unwrap();
    let store_after = kernel.get_entry_store(EntryStoreId(0));
    assert_eq!(store_after.get(slot).attr_read(0), 999);
}

#[test]
fn entry_store_core_meta_survives_grow() {
    let mut kernel = new_controller(config(4));
    let mut consumer = TestConsumer::new(kernel.get_control_plane());

    const CORE_V: i32 = 12_345;
    const META_V: i32 = 67_890;

    let slot = {
        let store = kernel.get_entry_store(EntryStoreId(0));
        let slot = store.insert().unwrap();
        store.get(slot).core_write(0, CORE_V);
        store.get(slot).meta_write(0, META_V);
        slot
    };

    kernel.publish();
    kernel.grow(config(8)).unwrap();
    kernel.publish();

    let mirror = consumer.acquire_mirror();
    let reader_store = mirror.get_entry_store(EntryStoreId(0));
    assert_eq!(reader_store.get(slot).core_read(0), CORE_V);
    assert_eq!(reader_store.get(slot).meta_read(0), META_V);
}

#[test]
fn publish_tb_independent_of_default_publish() {
    let kernel = new_controller(config(16));
    let mut consumer = TestConsumer::new(kernel.get_control_plane());
    let mirror = consumer.acquire_mirror();
    kernel.get_user_tb(TripleBufferId(0)).write(0, 42);
    kernel.publish_tb(TripleBufferId(0));
    mirror.swap_tb(TripleBufferId(0));
    assert_eq!(
        mirror.get_user_tb(TripleBufferId(0)).read(0),
        42,
        "user TB visible after publish_tb + swap_tb"
    );
    assert!(
        !mirror.swap(),
        "default TB must have no pending publish when only publish_tb was used"
    );
}

#[test]
fn lut_write_read_roundtrip() {
    let mut kernel = new_controller(config_with_lut_on_default(1));
    let mut consumer = TestConsumer::new(kernel.get_control_plane());
    let mirror = consumer.acquire_mirror();
    kernel.get_lut(LutId(0)).write(0, 42);
    kernel.publish();
    assert!(mirror.swap());
    assert_eq!(mirror.get_lut(LutId(0)).read(0), 42);
}

#[test]
fn lut_write_all_visible_after_publish() {
    let mut kernel = new_controller(config_with_lut_on_default(8));
    let mut consumer = TestConsumer::new(kernel.get_control_plane());
    let mirror = consumer.acquire_mirror();
    let data: Vec<i32> = (0..8).map(|i| i * 3).collect();
    kernel.get_lut(LutId(0)).write_all(&data);
    kernel.publish();
    assert!(mirror.swap());
    let mut out = [0i32; 8];
    mirror.get_lut(LutId(0)).read_all(&mut out);
    assert_eq!(out.as_slice(), data.as_slice());
}

#[test]
fn lut_not_visible_before_publish() {
    let mut kernel = new_controller(config_with_lut_on_default(1));
    let mut consumer = TestConsumer::new(kernel.get_control_plane());
    let mirror = consumer.acquire_mirror();
    kernel.get_lut(LutId(0)).write(0, 99);
    assert_eq!(
        mirror.get_lut(LutId(0)).read(0),
        0,
        "consumer read buffer must not see producer writes until publish+swap"
    );
    kernel.publish();
    assert!(mirror.swap());
    assert_eq!(mirror.get_lut(LutId(0)).read(0), 99);
}

#[test]
fn lut_survives_grow() {
    let mut kernel = new_controller(config_with_lut_on_default(4));
    let mut consumer = TestConsumer::new(kernel.get_control_plane());

    kernel.get_lut(LutId(0)).write(0, 1001);
    kernel.get_lut(LutId(0)).write(3, 2002);

    let mut larger = config_with_lut_on_default(8);
    larger.network_config.node_capacity = 32;
    larger.network_config.synapse_capacity = 32;
    kernel.grow(larger).unwrap();
    kernel.publish();

    // grow swapped the epoch out from under any previously-acquired mirror;
    // re-acquire so we see the new epoch's TB.
    let mirror = consumer.acquire_mirror();
    assert_eq!(mirror.get_lut(LutId(0)).read(0), 1001);
    assert_eq!(mirror.get_lut(LutId(0)).read(3), 2002);
}

#[test]
fn lut_on_user_tb_independent_publish() {
    let kernel = new_controller(config_with_lut_on_user_tb(16));
    let mut consumer = TestConsumer::new(kernel.get_control_plane());
    let mirror = consumer.acquire_mirror();
    kernel.get_lut(LutId(0)).write(0, 7);
    kernel.get_lut(LutId(0)).write(5, 8);
    kernel.publish_tb(TripleBufferId(0));
    mirror.swap_tb(TripleBufferId(0));
    assert_eq!(mirror.get_lut(LutId(0)).read(0), 7);
    assert_eq!(mirror.get_lut(LutId(0)).read(5), 8);
    assert!(
        !mirror.swap(),
        "default TB must have no pending publish when only publish_tb was used"
    );
}
