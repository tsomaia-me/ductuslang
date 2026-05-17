mod common;

use synaptic_kernel::epoch_consumer::EpochConsumer;
use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;
use synaptic_kernel::primitives::entry_store_def::EntryStoreId;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::triple_buffer_def::TripleBufferId;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;

type TestKernel = Kernel<1, 1, 1>;
type TestConsumer = EpochConsumer<1, 1, 1>;

fn config() -> KernelConfig<1, 1, 1> {
    common::kernel_config_1_1(16, 32, NODE_META, NODE_ATTR, SYNAPSE_META, SYNAPSE_ATTR)
}

/// Build a `(Kernel, EpochConsumer)` pair. Returning the tuple in this
/// order is load-bearing: tuple bindings drop in reverse declaration
/// order, so the consumer drops first and releases its
/// `Arc<ControlPlane>` clone before the kernel's debug-time Drop assert
/// runs.
fn setup() -> (TestKernel, TestConsumer) {
    let kernel = TestKernel::new(config());
    let consumer = TestConsumer::new(kernel.get_control_plane());
    (kernel, consumer)
}

fn insert_with_tick(kernel: &TestKernel, kind: i32, tick: i32) -> SlotId {
    let slot = kernel.insert_node(kind).unwrap();
    kernel.get_node(slot).set_meta(0, tick);
    slot
}

// ============ Construction ============

#[test]
fn fresh_consumer_has_nothing_to_swap() {
    let (_kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();
    // First acquire_mirror swap returns false (no pending data).
    assert!(!mirror.swap());
}

// ============ Reader sees nothing before publish/swap ============

#[test]
fn reader_does_not_see_unpublished_topology() {
    let (kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    let _slot = kernel.insert_node(1).unwrap();

    // No publish — nothing new to swap.
    assert!(!mirror.swap());
}

// ============ Reader sees nodes after publish + swap ============

#[test]
fn reader_sees_node_meta_after_publish_swap() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    let slot = insert_with_tick(&kernel, 5, 999);

    // Before publish+swap: TB (meta) plane has not shifted into the reader's
    // active buffer, so the freshly-written meta MUST NOT be visible yet.
    assert_ne!(
        mirror.get_node(slot).get_meta(0),
        999,
        "meta must not be visible before publish+swap",
    );

    kernel.publish();
    assert!(mirror.swap());

    let node = mirror.get_node(slot);
    assert_eq!(node.get_kind(), 5);
    assert_eq!(node.get_meta(0), 999);
}

#[test]
fn reader_traverses_full_chain() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    // chain: a -> b -> c
    let a = insert_with_tick(&kernel, 1, 10);
    let b = kernel.insert_node_after(a, 2).unwrap();
    kernel.get_node(b).set_meta(0, 20);
    let c = kernel.insert_node_after(b, 3).unwrap();
    kernel.get_node(c).set_meta(0, 30);

    kernel.publish();
    assert!(mirror.swap());

    let head = mirror.get_node(a);
    assert_eq!(head.get_kind(), 1);

    let n_b = mirror.get_node(head.get_next_ptr().unwrap());
    assert_eq!(n_b.get_kind(), 2);

    let n_c = mirror.get_node(n_b.get_next_ptr().unwrap());
    assert_eq!(n_c.get_kind(), 3);
    assert!(n_c.get_next_ptr().is_none());
}

// ============ Reader sees removal after publish ============

#[test]
fn reader_sees_removal_after_publish_swap() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    // chain: a -> b
    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node_after(a, 2).unwrap();

    kernel.remove_node(b).unwrap();
    // chain: a

    kernel.publish();
    assert!(mirror.swap());

    let na = mirror.get_node(a);
    assert_eq!(na.get_kind(), 1);
    assert!(na.get_next_ptr().is_none());
}

// ============ Reader snapshot isolation ============

#[test]
fn reader_retains_old_snapshot_without_swap() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    // cycle 1
    let a = kernel.insert_node(1).unwrap();
    kernel.get_node(a).set_meta(0, 11);
    kernel.publish();
    assert!(mirror.swap());
    assert_eq!(mirror.get_node(a).get_meta(0), 11);

    // cycle 2: mutate but reader does NOT swap
    kernel.get_node(a).set_meta(0, 22);
    kernel.publish();

    // reader still sees cycle 1 snapshot
    assert_eq!(mirror.get_node(a).get_meta(0), 11);
}

#[test]
fn reader_sees_updated_snapshot_after_swap() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    // cycle 1
    let a = kernel.insert_node(1).unwrap();
    kernel.publish();
    assert!(mirror.swap());
    assert_eq!(mirror.get_node(a).get_kind(), 1);

    // cycle 2
    let b = kernel.insert_node(2).unwrap();
    kernel.publish();
    assert!(mirror.swap());
    assert_eq!(mirror.get_node(b).get_kind(), 2);
}

// ============ Reader sees synapse data ============

#[test]
fn reader_sees_synapse_after_publish_swap() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();
    let syn = kernel.connect(src, tgt, 42).unwrap();

    kernel.publish();
    assert!(mirror.swap());

    let s = mirror.get_synapse(syn);
    assert_eq!(s.get_kind(), 42);
    assert_eq!(s.get_source_ptr(), src);
    assert_eq!(s.get_target_ptr(), tgt);
}

#[test]
fn reader_traverses_synapse_chain() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    let src = kernel.insert_node(1).unwrap();
    let tgt1 = kernel.insert_node(2).unwrap();
    let tgt2 = kernel.insert_node(3).unwrap();
    let tgt3 = kernel.insert_node(4).unwrap();

    let s1 = kernel.connect(src, tgt1, 10).unwrap();
    let s2 = kernel.connect(src, tgt2, 20).unwrap();
    let s3 = kernel.connect(src, tgt3, 30).unwrap();

    kernel.publish();
    assert!(mirror.swap());

    let src_node = mirror.get_node(src);
    assert_eq!(src_node.get_outgoing_synapse_head(), Some(s1));

    let r1 = mirror.get_synapse(s1);
    assert_eq!(r1.get_kind(), 10);
    assert_eq!(r1.get_outgoing_next_ptr(), Some(s2));

    let r2 = mirror.get_synapse(s2);
    assert_eq!(r2.get_kind(), 20);
    assert_eq!(r2.get_outgoing_next_ptr(), Some(s3));

    let r3 = mirror.get_synapse(s3);
    assert_eq!(r3.get_kind(), 30);
    assert!(r3.get_outgoing_next_ptr().is_none());
}

#[test]
fn reader_sees_disconnect_after_publish_swap() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    let src = kernel.insert_node(1).unwrap();
    let tgt1 = kernel.insert_node(2).unwrap();
    let tgt2 = kernel.insert_node(3).unwrap();

    let s1 = kernel.connect(src, tgt1, 10).unwrap();
    let s2 = kernel.connect(src, tgt2, 20).unwrap();

    kernel.disconnect_synapse(s1).unwrap();

    kernel.publish();
    assert!(mirror.swap());

    let src_node = mirror.get_node(src);
    assert_eq!(src_node.get_outgoing_synapse_head(), Some(s2));

    let r2 = mirror.get_synapse(s2);
    assert!(r2.get_outgoing_prev_ptr().is_none(), "s2 is now head");
    assert!(r2.get_outgoing_next_ptr().is_none(), "s2 is now tail");
}

// ============ Reader sees attributes (shared plane) ============

#[test]
fn reader_sees_node_attributes_immediately() {
    let (kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    let slot = kernel.insert_node(1).unwrap();

    // Attributes are on the MEM plane — visible without publish.
    kernel.get_node(slot).attr_write(0, 60);
    kernel.get_node(slot).attr_write(1, 100);

    assert_eq!(mirror.get_node(slot).attr_read(0), 60);
    assert_eq!(mirror.get_node(slot).attr_read(1), 100);
}

#[test]
fn reader_sees_bulk_node_attributes() {
    let (kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

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

    let mut view = [0i32; NODE_ATTR];
    mirror.get_node(slot).attr_read_all(&mut view);
    assert_eq!(view[0], 72);
    assert_eq!(view[1], 90);
    assert_eq!(view[2], 960);
    assert_eq!(view[3], 64);
}

#[test]
fn reader_sees_synapse_attributes_immediately() {
    let (kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    let src = kernel.insert_node(1).unwrap();
    let tgt = kernel.insert_node(2).unwrap();
    let syn = kernel.connect(src, tgt, 10).unwrap();

    kernel.get_synapse(syn).attr_write(0, 500);
    kernel.get_synapse(syn).attr_write(1, 3);
    kernel.get_synapse(syn).attr_write(2, -7);
    kernel.get_synapse(syn).attr_write(3, 100);
    kernel.get_synapse(syn).attr_write(4, 200);
    kernel.get_synapse(syn).attr_write(5, 50);

    assert_eq!(mirror.get_synapse(syn).attr_read(0), 500);
    assert_eq!(mirror.get_synapse(syn).attr_read(1), 3);
    assert_eq!(mirror.get_synapse(syn).attr_read(2), -7);
}

#[test]
fn reader_attributes_view_matches_individual_reads() {
    let (kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    let slot = kernel.insert_node(1).unwrap();
    kernel.get_node(slot).attr_write(0, 42);
    kernel.get_node(slot).attr_write(5, 99);

    let mut view = [0i32; NODE_ATTR];
    mirror.get_node(slot).attr_read_all(&mut view);
    assert_eq!(view[0], mirror.get_node(slot).attr_read(0));
    assert_eq!(view[5], mirror.get_node(slot).attr_read(5));
}

// ============ Multi-cycle with reader ============

#[test]
fn multi_cycle_insert_remove_connect_disconnect() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    // cycle 1: build chain a -> b with synapse a->b
    let a = insert_with_tick(&kernel, 1, 100);
    let b = kernel.insert_node_after(a, 2).unwrap();
    kernel.get_node(b).set_meta(0, 200);
    let s1 = kernel.connect(a, b, 10).unwrap();
    kernel.get_node(a).attr_write(0, 60);
    kernel.publish();
    assert!(mirror.swap());

    assert_eq!(mirror.get_node(a).get_kind(), 1);
    assert_eq!(mirror.get_node(b).get_kind(), 2);
    assert_eq!(mirror.get_synapse(s1).get_kind(), 10);
    assert_eq!(mirror.get_node(a).attr_read(0), 60);

    // cycle 2: extend with c, connect b->c, disconnect a->b
    let c = kernel.insert_node_after(b, 3).unwrap();
    kernel.get_node(c).set_meta(0, 300);
    let s2 = kernel.connect(b, c, 20).unwrap();
    kernel.disconnect_synapse(s1).unwrap();
    kernel.publish();
    assert!(mirror.swap());

    assert_eq!(mirror.get_node(c).get_kind(), 3);
    let b_node = mirror.get_node(b);
    assert_eq!(b_node.get_outgoing_synapse_head(), Some(s2));
    assert_eq!(mirror.get_synapse(s2).get_source_ptr(), b);
    assert_eq!(mirror.get_synapse(s2).get_target_ptr(), c);

    assert!(mirror.get_node(a).get_outgoing_synapse_head().is_none());
}

// ============ swap() return value ============

#[test]
fn swap_returns_false_when_no_new_data() {
    let (_kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();
    assert!(!mirror.swap(), "no publish happened");
}

#[test]
fn swap_returns_true_when_new_data() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();
    kernel.insert_node(1).unwrap();
    kernel.publish();
    assert!(mirror.swap(), "publish happened");
}

// ============ Empty store after removing all ============

#[test]
fn reader_sees_chain_emptied_after_removing_all() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node_after(a, 2).unwrap();

    kernel.remove_node(a).unwrap();
    kernel.remove_node(b).unwrap();

    // Two-cycle reclaim: publish, ack via swap, publish drains the deferred
    // queue. After this the slot allocator's count is back to zero.
    kernel.publish();
    assert!(mirror.swap());
    kernel.publish();

    assert_eq!(kernel.node_count(), 0);
}

// ============ Attribute mutation visible between publishes ============

#[test]
fn attribute_mutation_visible_between_publishes() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    let slot = kernel.insert_node(1).unwrap();
    kernel.publish();
    assert!(mirror.swap());

    // mutate attribute WITHOUT publishing
    kernel.get_node(slot).attr_write(0, 999);

    // reader sees it immediately (shared plane, not triple-buffered)
    assert_eq!(mirror.get_node(slot).attr_read(0), 999);
}

#[test]
fn get_entry_store_returns_readable_store() {
    let (_kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();
    let store = mirror.get_entry_store(EntryStoreId(0));
    assert!(store.capacity() > 0);
}

#[test]
fn swap_tb_only_affects_targeted_tb() {
    let (mut kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();

    kernel.get_user_tb(TripleBufferId(0)).write(0, 1234);
    let slot = insert_with_tick(&kernel, 5, 999);

    kernel.publish_tb(TripleBufferId(0));
    mirror.swap_tb(TripleBufferId(0));
    assert_eq!(mirror.get_user_tb(TripleBufferId(0)).read(0), 1234);

    assert_ne!(
        mirror.get_node(slot).get_meta(0),
        999,
        "default TB meta must not be visible before kernel.publish + swap"
    );
    assert!(!mirror.swap(), "default TB has no pending publish");

    kernel.publish();
    assert!(mirror.swap());
    let node = mirror.get_node(slot);
    assert_eq!(node.get_kind(), 5);
    assert_eq!(node.get_meta(0), 999);
}

#[test]
fn entry_store_attr_visible_without_swap() {
    let (kernel, mut consumer) = setup();
    let mirror = consumer.acquire_mirror();
    let store = kernel.get_entry_store(EntryStoreId(0));
    let slot = store.insert().unwrap();
    store.get(slot).attr_write(0, 777);
    assert_eq!(
        mirror.get_entry_store(EntryStoreId(0)).get(slot).attr_read(0),
        777
    );
}
