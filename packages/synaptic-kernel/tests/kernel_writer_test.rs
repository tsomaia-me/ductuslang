mod common;

use synaptic_kernel::epoch_consumer::EpochConsumer;
use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;
use synaptic_kernel::primitives::slot::SlotId;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;

type TestKernel = Kernel<1, 1, 1>;
type TestConsumer = EpochConsumer<1, 1, 1>;

fn config() -> KernelConfig<1, 1, 1> {
    common::kernel_config_1_1(16, 32, NODE_META, NODE_ATTR, SYNAPSE_META, SYNAPSE_ATTR)
}

fn create_writer() -> TestKernel {
    TestKernel::new(config())
}

fn insert_with_tick(kernel: &TestKernel, kind: i32, tick: i32) -> SlotId {
    let slot = kernel.insert_node(kind).unwrap();
    kernel.get_node(slot).set_meta(0, tick);
    slot
}

// ============ Construction ============

#[test]
fn kernel_new_starts_empty() {
    let kernel = create_writer();
    assert_eq!(kernel.node_count(), 0);
    assert_eq!(kernel.synapse_count(), 0);
}

// ============ Node insertion ============

#[test]
fn insert_node_returns_slot() {
    let kernel = create_writer();
    let slot = kernel.insert_node(1);
    assert!(slot.is_ok());
    // SlotId is non-zero by construction; just verify insert succeeded.
    let _ = slot.unwrap();
}

#[test]
fn insert_node_writes_kind_and_tick_meta() {
    let kernel = create_writer();
    let slot = insert_with_tick(&kernel, 5, 999);

    let node = kernel.get_node(slot);
    assert_eq!(node.get_kind(), 5);
    assert_eq!(node.get_meta(0), 999);
}

#[test]
fn insert_node_creates_orphan() {
    let kernel = create_writer();
    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node(2).unwrap();

    // Both are orphans: prev_ptr = next_ptr = None, no chain links.
    let na = kernel.get_node(a);
    let nb = kernel.get_node(b);
    assert!(na.get_next_ptr().is_none());
    assert!(na.get_prev_ptr().is_none());
    assert!(nb.get_next_ptr().is_none());
    assert!(nb.get_prev_ptr().is_none());
    assert_ne!(a, b);
}

#[test]
fn insert_after_splices_correctly() {
    let kernel = create_writer();

    let a = kernel.insert_node(1).unwrap();
    let c = kernel.insert_node_after(a, 3).unwrap();
    let b = kernel.insert_node_after(a, 2).unwrap();

    // chain: a -> b -> c
    assert_eq!(kernel.get_node(a).get_next_ptr(), Some(b));
    assert_eq!(kernel.get_node(b).get_next_ptr(), Some(c));
    assert!(kernel.get_node(c).get_next_ptr().is_none());
}

#[test]
fn insert_before_splices_correctly() {
    let kernel = create_writer();

    let a = kernel.insert_node(1).unwrap();
    let c = kernel.insert_node_after(a, 3).unwrap();
    let b = kernel.insert_node_before(c, 2).unwrap();

    // a -> b -> c
    assert_eq!(kernel.get_node(a).get_next_ptr(), Some(b));
    assert_eq!(kernel.get_node(b).get_next_ptr(), Some(c));
    assert_eq!(kernel.get_node(c).get_prev_ptr(), Some(b));
}

#[test]
fn insert_exhausts_capacity() {
    let kernel = create_writer();

    for i in 0..16 {
        assert!(kernel.insert_node(i).is_ok());
    }
    assert!(kernel.insert_node(99).is_err());
}

// ============ Node removal + deferred frees ============

#[test]
fn remove_node_heals_chain() {
    let kernel = create_writer();

    // chain: a -> b -> c
    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node_after(a, 2).unwrap();
    let c = kernel.insert_node_after(b, 3).unwrap();

    kernel.remove_node(b).unwrap();
    // chain: a -> c

    assert_eq!(kernel.get_node(a).get_next_ptr(), Some(c));
    assert_eq!(kernel.get_node(c).get_prev_ptr(), Some(a));
}

#[test]
fn remove_then_publish_reclaims_slot() {
    let mut kernel = create_writer();

    // fill all slots
    let mut slots = Vec::new();
    for i in 0..16 {
        slots.push(kernel.insert_node(i).unwrap());
    }
    assert!(kernel.insert_node(99).is_err(), "capacity full");

    // remove one
    kernel.remove_node(slots[0]).unwrap();

    // slot not yet reclaimed (deferred)
    assert!(kernel.insert_node(99).is_err(), "still deferred");

    // publish #1: shift to previous list
    kernel.publish();

    // explicitly acknowledge the generation boundary
    TestConsumer::new(kernel.get_control_plane()).acquire_mirror();

    // publish #2: drains the previous list
    kernel.publish();

    // now the slot is available
    let reclaimed = kernel.insert_node(99);
    assert!(reclaimed.is_ok(), "slot reclaimed after publish");
}

// ============ Synapse connect/disconnect ============
#[test]
fn connect_creates_synapse() {
    let kernel = create_writer();

    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();

    let syn = kernel.connect(src, tgt, 10).unwrap();

    let s = kernel.get_synapse(syn);
    assert_eq!(s.get_kind(), 10);
    assert_eq!(s.get_source_ptr(), src);
    assert_eq!(s.get_target_ptr(), tgt);
}

#[test]
fn connect_updates_node_synapse_pointers() {
    let kernel = create_writer();

    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();
    let syn = kernel.connect(src, tgt, 10).unwrap();

    assert_eq!(kernel.get_node(src).get_outgoing_synapse_head(), Some(syn));
    assert_eq!(kernel.get_node(src).get_outgoing_synapse_tail(), Some(syn));
    assert_eq!(kernel.get_node(tgt).get_incoming_synapse_head(), Some(syn));
    assert_eq!(kernel.get_node(tgt).get_incoming_synapse_tail(), Some(syn));
}

#[test]
fn disconnect_heals_synapse_chain() {
    let kernel = create_writer();

    let src = kernel.insert_node(1).unwrap();
    let tgt1 = kernel.insert_node(2).unwrap();
    let tgt2 = kernel.insert_node(3).unwrap();

    let s1 = kernel.connect(src, tgt1, 10).unwrap();
    let s2 = kernel.connect(src, tgt2, 20).unwrap();

    kernel.disconnect_synapse(s1).unwrap();

    assert_eq!(kernel.get_node(src).get_outgoing_synapse_head(), Some(s2));
    assert_eq!(kernel.get_node(src).get_outgoing_synapse_tail(), Some(s2));
    assert!(kernel.get_synapse(s2).get_outgoing_prev_ptr().is_none());
}

#[test]
fn disconnect_then_publish_reclaims_synapse_slot() {
    let mut kernel = create_writer();

    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();

    // fill all synapse slots
    let mut synapses = Vec::new();
    for i in 0..32 {
        synapses.push(kernel.connect(src, tgt, i).unwrap());
    }
    assert!(
        kernel.connect(src, tgt, 99).is_err(),
        "synapse capacity full"
    );

    kernel.disconnect_synapse(synapses[0]).unwrap();

    // not yet reclaimed
    assert!(
        kernel.connect(src, tgt, 99).is_err(),
        "still deferred"
    );

    kernel.publish();
    
    // explicitly acknowledge the generation boundary
    TestConsumer::new(kernel.get_control_plane()).acquire_mirror();
    
    kernel.publish(); // Two cycle deferral required to physically reclaim

    // now reclaimed
    assert!(
        kernel.connect(src, tgt, 99).is_ok(),
        "reclaimed after publish"
    );
}

// ============ Node attributes ============
#[test]
fn set_node_attribute_single_field() {
    let kernel = create_writer();
    let slot = kernel.insert_node(1).unwrap();

    kernel.get_node(slot).attr_write(0, 60); // pitch
    kernel.get_node(slot).attr_write(1, 100); // velocity

    assert_eq!(kernel.get_node(slot).attr_read(0), 60);
    assert_eq!(kernel.get_node(slot).attr_read(1), 100);
}

#[test]
fn set_node_attributes_bulk() {
    let kernel = create_writer();
    let slot = kernel.insert_node(1).unwrap();

    kernel.get_node(slot).attr_write(0, 72);
    kernel.get_node(slot).attr_write(1, 90);
    kernel.get_node(slot).attr_write(2, 960);
    kernel.get_node(slot).attr_write(3, 64);
    kernel.get_node(slot).attr_write(4, 10);
    kernel.get_node(slot).attr_write(5, 20);
    kernel.get_node(slot).attr_write(6, 30);
    kernel.get_node(slot).attr_write(7, -3);
    kernel.get_node(slot).attr_write(8, 7);
    kernel.get_node(slot).attr_write(9, 0);

    assert_eq!(kernel.get_node(slot).attr_read(0), 72);
    assert_eq!(kernel.get_node(slot).attr_read(1), 90);
    assert_eq!(kernel.get_node(slot).attr_read(2), 960);
}

#[test]
fn get_node_attributes_returns_view() {
    let kernel = create_writer();
    let slot = kernel.insert_node(1).unwrap();

    kernel.get_node(slot).attr_write(0, 42);
    kernel.get_node(slot).attr_write(5, 99);

    // The standalone `get_node_attributes(slot)` view type is gone; the new API
    // returns attributes via `get_node(slot).attr_read(...)` / `attr_read_all()`.
    assert_eq!(kernel.get_node(slot).attr_read(0), 42);
    assert_eq!(kernel.get_node(slot).attr_read(5), 99);
    let mut buf = [0i32; NODE_ATTR];
    kernel.get_node(slot).attr_read_all(&mut buf);
    assert_eq!(buf[0], 42);
    assert_eq!(buf[5], 99);
}

#[test]
fn node_attributes_independent_across_slots() {
    let kernel = create_writer();
    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node(2).unwrap();

    kernel.get_node(a).attr_write(0, 111);
    kernel.get_node(b).attr_write(0, 222);

    assert_eq!(kernel.get_node(a).attr_read(0), 111);
    assert_eq!(kernel.get_node(b).attr_read(0), 222);
}

// ============ Synapse attributes ============

#[test]
fn set_synapse_attribute_single_field() {
    let kernel = create_writer();
    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();
    let syn = kernel.connect(src, tgt, 10).unwrap();

    kernel.get_synapse(syn).attr_write(0, 1000); // weight
    kernel.get_synapse(syn).attr_write(1, -10); // tick_offset

    assert_eq!(kernel.get_synapse(syn).attr_read(0), 1000);
    assert_eq!(kernel.get_synapse(syn).attr_read(1), -10);
}

#[test]
fn set_synapse_attributes_bulk() {
    let kernel = create_writer();
    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();
    let syn = kernel.connect(src, tgt, 10).unwrap();

    kernel.get_synapse(syn).attr_write(0, 500);
    kernel.get_synapse(syn).attr_write(1, 3);
    kernel.get_synapse(syn).attr_write(2, -7);
    kernel.get_synapse(syn).attr_write(3, 100);
    kernel.get_synapse(syn).attr_write(4, 200);
    kernel.get_synapse(syn).attr_write(5, 50);

    assert_eq!(kernel.get_synapse(syn).attr_read(0), 500);
    assert_eq!(kernel.get_synapse(syn).attr_read(1), 3);
    assert_eq!(kernel.get_synapse(syn).attr_read(2), -7);
}

// ============ Publish lifecycle ============

#[test]
fn publish_succeeds_on_empty_kernel() {
    let mut kernel = create_writer();
    kernel.publish();
}

#[test]
fn publish_after_mutations_succeeds() {
    let mut kernel = create_writer();

    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();
    kernel.connect(src, tgt, 10).unwrap();

    kernel.publish();
}

#[test]
fn multiple_publish_cycles() {
    let mut kernel = create_writer();

    // cycle 1: insert
    let a = kernel.insert_node(1).unwrap();
    kernel.publish();

    // cycle 2: insert + remove
    let b = kernel.insert_node(2).unwrap();
    kernel.remove_node(a).unwrap();
    kernel.publish();

    // cycle 3: insert another node. (Without a consumer acking, a's slot
    // is still deferred — but the writer can keep going until capacity is hit.)
    let c = kernel.insert_node(3).unwrap();
    kernel.publish();

    // b and c are alive; a was removed.
    assert_eq!(kernel.get_node(b).get_kind(), 2);
    assert_eq!(kernel.get_node(c).get_kind(), 3);
}

#[test]
fn deferred_free_two_cycle_delay() {
    let mut kernel = create_writer();

    // fill capacity
    let mut slots = Vec::new();
    for i in 0..16 {
        slots.push(kernel.insert_node(i).unwrap());
    }

    // remove in cycle 0 (pushes to current deferred list)
    kernel.remove_node(slots[0]).unwrap();

    // publish #1: drains previous list (empty), toggles.
    // Now slots[0] is in the "previous" list.
    kernel.publish();
    
    // explicitly acknowledge the generation boundary
    TestConsumer::new(kernel.get_control_plane()).acquire_mirror();

    // publish #2: drains previous list (contains slots[0]). Slot reclaimed.
    kernel.publish();

    // slot should be available now
    assert!(kernel.insert_node(99).is_ok());
}

// ============ Capacity-saturation cycling ============
//
// The full saturation lifecycle: alloc to capacity, defer-free every
// allocation, drive the two-cycle reclaim, alloc to capacity again. This
// catches off-by-one errors in the deferred-reclamation path where some
// slots get stuck in the previous-list and never come back.

#[test]
fn full_capacity_saturation_cycle_reclaims_all_slots() {
    let mut kernel = create_writer();
    const CAP: usize = 16;

    // Round 1: fill to capacity.
    let mut slots = Vec::with_capacity(CAP);
    for i in 0..CAP {
        slots.push(kernel.insert_node(i as i32).unwrap());
    }
    assert!(
        kernel.insert_node(99).is_err(),
        "must be at capacity after CAP inserts"
    );

    // Defer-free every slot.
    for slot in &slots {
        kernel.remove_node(*slot).unwrap();
    }

    // Drive the two-cycle reclaim.
    kernel.publish();
    TestConsumer::new(kernel.get_control_plane()).acquire_mirror();
    kernel.publish();

    // All slots must have returned to the free list.
    assert_eq!(
        kernel.node_count(),
        0,
        "expected 0 live slots after full saturation/drain cycle"
    );

    // Round 2: must accept CAP fresh inserts again.
    let mut new_slots = Vec::with_capacity(CAP);
    for i in 0..CAP {
        new_slots.push(
            kernel
                .insert_node((i + 100) as i32)
                .expect("post-drain insert must succeed up to capacity"),
        );
    }
    assert!(
        kernel.insert_node(99).is_err(),
        "must hit capacity again after re-filling"
    );

    // The new entries must read back the values we just wrote — proves
    // the reclaimed slots were properly cleared, not left with stale
    // round-1 data that survived the publish/swap dance.
    for (i, slot) in new_slots.iter().enumerate() {
        assert_eq!(kernel.get_node(*slot).get_kind(), (i + 100) as i32);
    }
}

#[test]
fn synapse_capacity_saturation_cycle_reclaims_all_slots() {
    // Same shape as above, but exercises the synapse store's deferred
    // reclamation. Synapses share the staging-buffer protocol with nodes
    // but live in their own EntryStoreWriter, so a bug in one wouldn't
    // necessarily surface in the other.
    let mut kernel = create_writer();
    const SYN_CAP: usize = 32;

    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();

    let mut synapses = Vec::with_capacity(SYN_CAP);
    for i in 0..SYN_CAP {
        synapses.push(kernel.connect(src, tgt, i as i32).unwrap());
    }
    assert!(
        kernel.connect(src, tgt, 999).is_err(),
        "must be at synapse capacity"
    );

    for s in &synapses {
        kernel.disconnect_synapse(*s).unwrap();
    }

    kernel.publish();
    TestConsumer::new(kernel.get_control_plane()).acquire_mirror();
    kernel.publish();

    assert_eq!(
        kernel.synapse_count(),
        0,
        "expected 0 live synapses after full saturation/drain cycle"
    );

    // Re-saturate.
    for i in 0..SYN_CAP {
        kernel
            .connect(src, tgt, (i + 100) as i32)
            .expect("post-drain connect must succeed");
    }
    assert!(
        kernel.connect(src, tgt, 999).is_err(),
        "must hit synapse capacity after re-saturating"
    );
}

// ============ Self-loop ============

#[test]
fn self_loop_connect_disconnect() {
    let kernel = create_writer();
    let n = kernel.insert_node(1).unwrap();

    let syn = kernel.connect(n, n, 99).unwrap();

    assert_eq!(kernel.get_node(n).get_outgoing_synapse_head(), Some(syn));
    assert_eq!(kernel.get_node(n).get_incoming_synapse_head(), Some(syn));

    kernel.disconnect_synapse(syn).unwrap();

    assert!(kernel.get_node(n).get_outgoing_synapse_head().is_none());
    assert!(kernel.get_node(n).get_incoming_synapse_head().is_none());
}

// ============ compute_mem_size ============

#[test]
fn calculate_mem_size_is_positive() {
    let cfg = config();
    assert!(TestKernel::calculate_size_on_mem(&cfg) > 0);
}

// ============ Grow (absorbed copy_from semantics) ============
//
// `Kernel::copy_from` is gone. `Kernel::grow(new_config)` is the public
// replacement: it allocates a larger backing buffer, migrates all state
// (topology + attributes) into it, and hot-swaps the consumer's mirror via
// the `ControlPlane`. The topology-preservation assertion below is what the
// old `copy_from_scales_full_topology_graph` test guarded.

#[test]
fn grow_scales_full_topology_graph() {
    let mut kernel = create_writer();

    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();
    let syn = kernel.connect(src, tgt, 10).unwrap();

    kernel.get_node(src).attr_write(0, 60);
    kernel.publish();

    kernel
        .grow(common::kernel_config_1_1(
            32,
            64,
            NODE_META,
            NODE_ATTR,
            SYNAPSE_META,
            SYNAPSE_ATTR,
        ))
        .unwrap();

    // Nodes survived
    let n_src = kernel.get_node(src);
    assert_eq!(n_src.get_kind(), 1);
    let n_tgt = kernel.get_node(tgt);
    assert_eq!(n_tgt.get_kind(), 2);

    // Synapse survived
    let s_syn = kernel.get_synapse(syn);
    assert_eq!(s_syn.get_kind(), 10);
    assert_eq!(s_syn.get_source_ptr(), src);
    assert_eq!(s_syn.get_target_ptr(), tgt);

    // Structural links intact
    assert_eq!(n_src.get_outgoing_synapse_head(), Some(syn));
    assert_eq!(n_tgt.get_incoming_synapse_head(), Some(syn));

    // Attributes survived
    assert_eq!(kernel.get_node(src).attr_read(0), 60);
}

#[test]
fn grow_rejects_smaller_capacity() {
    use synaptic_kernel::errors::kernel_error::KernelError;

    let mut kernel = create_writer();

    // Current kernel was created with node_capacity = 16, synapse_capacity = 32.
    // Attempting to grow into a smaller config must fail with InsufficientCapacity
    // rather than silently truncate — this is the modern replacement for the
    // `copy_from_panics_if_source_larger` invariant.
    let result = kernel.grow(common::kernel_config_1_1(
        8,
        16,
        NODE_META,
        NODE_ATTR,
        SYNAPSE_META,
        SYNAPSE_ATTR,
    ));
    assert!(matches!(result, Err(KernelError::InsufficientCapacity)));
}

// ============ Kind boundary tests ============
//
// `kind` lives in the high 8 bits of `core[0]`; the low 24 bits are
// reserved for future internal flags. `set_kind` must round-trip 0..=255
// and reject 256 in debug builds. The bitmask preservation invariant
// (low 24 bits unchanged after `set_kind`) is structural — the only way
// to corrupt it would be `kind << 24` accidentally OR-ing into the wider
// region or shifting wrong, both of which surface as wrong `get_kind`.

#[test]
fn node_kind_zero_round_trips() {
    let kernel = create_writer();
    let slot = kernel.insert_node(0).unwrap();
    assert_eq!(kernel.get_node(slot).get_kind(), 0);
}

#[test]
fn node_kind_255_round_trips() {
    let kernel = create_writer();
    let slot = kernel.insert_node(255).unwrap();
    assert_eq!(kernel.get_node(slot).get_kind(), 255);
}

#[test]
fn node_kind_survives_neighbor_pointer_mutations() {
    // Inserting siblings overwrites `next_ptr` / `prev_ptr` on the
    // watched node. If `set_kind`'s bitmask shift is ever broken so that
    // it overlaps with neighbouring core slots, that mutation could
    // bleed into kind. Test all sibling-pointer mutations leave the
    // watched node's high-edge kind value (0xFF) intact.
    let kernel = create_writer();
    let watched = kernel.insert_node(255).unwrap();

    // Mutate next_ptr / prev_ptr by inserting siblings.
    let _before = kernel.insert_node_before(watched, 1).unwrap();
    let _after = kernel.insert_node_after(watched, 2).unwrap();

    assert_eq!(
        kernel.get_node(watched).get_kind(),
        255,
        "kind must survive neighbour-pointer mutations"
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "out of bounds [0, 256)")]
fn node_kind_256_debug_panics() {
    // 256 overflows the 8-bit kind slot. In debug builds, the
    // `debug_assert!(value >= 0 && value < 256, ...)` in
    // `NodeWriter::set_kind` (called from `insert_node`) fires. Release
    // builds skip this check — by design — so the test is gated.
    let kernel = create_writer();
    let _ = kernel.insert_node(256);
}

#[test]
fn synapse_kind_zero_round_trips() {
    let kernel = create_writer();
    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();
    let s = kernel.connect(src, tgt, 0).unwrap();
    assert_eq!(kernel.get_synapse(s).get_kind(), 0);
}

#[test]
fn synapse_kind_255_round_trips() {
    let kernel = create_writer();
    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();
    let s = kernel.connect(src, tgt, 255).unwrap();
    assert_eq!(kernel.get_synapse(s).get_kind(), 255);
}

#[test]
fn synapse_kind_survives_peer_synapse_mutations() {
    // Peer synapses on the same source/target lists mutate the watched
    // synapse's outgoing/incoming next/prev pointers. Same bitmask
    // preservation invariant as the node test.
    let kernel = create_writer();
    let src = kernel.insert_node(1).unwrap();
    let t1 = kernel.insert_node(2).unwrap();
    let t2 = kernel.insert_node(3).unwrap();

    let watched = kernel.connect(src, t1, 255).unwrap();

    // Append a peer synapse: rewrites `outgoing_next_ptr` on watched.
    let _peer = kernel.connect(src, t2, 7).unwrap();

    assert_eq!(
        kernel.get_synapse(watched).get_kind(),
        255,
        "synapse kind must survive peer synapse list mutations"
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "out of bounds [0, 256)")]
fn synapse_kind_256_debug_panics() {
    let kernel = create_writer();
    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();
    let _ = kernel.connect(src, tgt, 256);
}
