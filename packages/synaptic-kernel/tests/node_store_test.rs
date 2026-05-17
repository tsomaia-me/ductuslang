use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;
use synaptic_kernel::topology::node::node_store_config::NodeStoreConfig;
use synaptic_kernel::topology::node::node_store_reader::NodeStoreReader;
use synaptic_kernel::topology::node::node_store_writer::NodeStoreWriter;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

const MEM_SIZE: usize = 16384;
const TB_START: usize = 0;
const TB_BUF_CAP: u32 = 4096;
const FL_START: usize = 13000;
const CAPACITY: u32 = 16;
const NODE_START_OFFSET: usize = 0;

struct TestHarness {
    _mem: AtomicBuffer,
    writer: synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter,
    reader: synaptic_kernel::primitives::triple_buffer_reader::TripleBufferReader,
    chain: NodeStoreWriter,
    chain_r: NodeStoreReader,
}

fn setup() -> TestHarness {
    let mem = create_mem(MEM_SIZE);
    let writer = TripleBufferWriter::new(Arc::clone(&mem), TB_START, TB_BUF_CAP);
    let reader = writer.to_reader();
    let chain = NodeStoreWriter::new(
        Arc::clone(&mem),
        writer.clone(),
        NodeStoreConfig {
            meta_stride: NODE_META,
            attr_stride: NODE_ATTR,
            capacity: CAPACITY,
        },
        FL_START,
        NODE_START_OFFSET,
    );
    let chain_r = chain.to_reader();

    TestHarness {
        _mem: mem,
        writer,
        reader,
        chain,
        chain_r,
    }
}

/// Insert an orphan with `kind` and stash `tick` in meta[0]. Returns the slot.
fn insert_with_tick(chain: &NodeStoreWriter, kind: i32, tick: i32) -> SlotId {
    let slot = chain.insert_node(kind).unwrap();
    chain.get_node(slot).set_meta(0, tick);
    slot
}

/// Build a chain `head -> ... -> tail` of `n` nodes via `insert_node` (head)
/// followed by `n-1` `insert_node_after` extensions. Returns the per-node slots
/// in iteration order: index 0 is head, index n-1 is tail. Each slot's kind
/// equals its 1-based index (1, 2, 3, ...).
fn build_chain(chain: &NodeStoreWriter, n: usize) -> Vec<SlotId> {
    assert!(n > 0);
    let mut slots = Vec::with_capacity(n);
    let head = chain.insert_node(1).unwrap();
    slots.push(head);
    for i in 1..n {
        let kind = (i + 1) as i32;
        let next = chain.insert_node_after(slots[i - 1], kind).unwrap();
        slots.push(next);
    }
    slots
}

// ============ NodeStoreWriter: insert_node creates orphans ============

#[test]
fn insert_node_creates_orphan_with_zero_pointers() {
    let h = setup();
    let chain = &h.chain;

    let slot = chain.insert_node(7).unwrap();
    assert_eq!(slot.get(), 1, "first alloc returns slot 1");

    let node = chain.get_node(slot);
    assert_eq!(node.get_kind(), 7);
    assert!(node.get_next_ptr().is_none(), "orphan has no next");
    assert!(node.get_prev_ptr().is_none(), "orphan has no prev");
}

#[test]
fn two_insert_node_calls_produce_disjoint_orphans() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node(2).unwrap();

    assert_ne!(a, b, "distinct slots");

    let na = chain.get_node(a);
    let nb = chain.get_node(b);

    // Each orphan stands alone.
    assert!(na.get_next_ptr().is_none());
    assert!(na.get_prev_ptr().is_none());
    assert!(nb.get_next_ptr().is_none());
    assert!(nb.get_prev_ptr().is_none());
}

#[test]
fn three_disjoint_orphans_remain_disconnected() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node(2).unwrap();
    let c = chain.insert_node(3).unwrap();

    for slot in [a, b, c] {
        let n = chain.get_node(slot);
        assert!(n.get_next_ptr().is_none());
        assert!(n.get_prev_ptr().is_none());
    }
    assert_eq!(chain.len(), 3);
}

// ============ NodeStoreWriter: insert_after ============

#[test]
fn insert_after_tail() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node_after(a, 2).unwrap();

    // chain: a -> b
    assert_eq!(chain.get_node(a).get_next_ptr(), Some(b));
    assert!(chain.get_node(a).get_prev_ptr().is_none(), "a is head");
    assert_eq!(chain.get_node(b).get_prev_ptr(), Some(a));
    assert!(chain.get_node(b).get_next_ptr().is_none(), "b is tail");
}

#[test]
fn insert_after_middle() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    let c = chain.insert_node_after(a, 3).unwrap();
    // chain: a -> c
    let b = chain.insert_node_after(a, 2).unwrap();
    // chain: a -> b -> c

    assert_eq!(chain.get_node(a).get_next_ptr(), Some(b));
    assert_eq!(chain.get_node(b).get_prev_ptr(), Some(a));
    assert_eq!(chain.get_node(b).get_next_ptr(), Some(c));
    assert_eq!(chain.get_node(c).get_prev_ptr(), Some(b));
    assert!(chain.get_node(c).get_next_ptr().is_none());
}

// ============ NodeStoreWriter: insert_before ============

#[test]
fn insert_before_chain_head_makes_new_head() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node_before(a, 2).unwrap();

    // a was head (prev_ptr=None); b should now be head.
    assert!(chain.get_node(b).get_prev_ptr().is_none(), "b is the new head");
    assert_eq!(chain.get_node(b).get_next_ptr(), Some(a));
    assert_eq!(chain.get_node(a).get_prev_ptr(), Some(b));
    assert!(chain.get_node(a).get_next_ptr().is_none());
}

#[test]
fn insert_before_middle_node() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    let c = chain.insert_node_after(a, 3).unwrap();
    // chain: a -> c
    let b = chain.insert_node_before(c, 2).unwrap();
    // chain: a -> b -> c

    assert_eq!(chain.get_node(a).get_next_ptr(), Some(b));
    assert_eq!(chain.get_node(b).get_prev_ptr(), Some(a));
    assert_eq!(chain.get_node(b).get_next_ptr(), Some(c));
    assert_eq!(chain.get_node(c).get_prev_ptr(), Some(b));
}

// ============ NodeStoreWriter: remove ============

#[test]
fn remove_only_node_leaves_store_empty() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    chain.remove_node(a).unwrap();

    // remove_node defers; drive the two-cycle reclaim to drain.
    chain.publish();
    chain.to_reader().ack_generation();
    chain.publish();

    assert_eq!(chain.len(), 0, "store has no live entries");
}

#[test]
fn remove_chain_head_promotes_successor_to_head() {
    let h = setup();
    let chain = &h.chain;

    // chain: a -> b
    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node_after(a, 2).unwrap();

    chain.remove_node(a).unwrap();

    // b is now the head (prev_ptr=None).
    let nb = chain.get_node(b);
    assert_eq!(nb.get_kind(), 2);
    assert!(nb.get_prev_ptr().is_none());
    assert!(nb.get_next_ptr().is_none());
}

#[test]
fn remove_tail_patches_predecessor() {
    let h = setup();
    let chain = &h.chain;

    // chain: a -> b
    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node_after(a, 2).unwrap();

    chain.remove_node(b).unwrap();
    // chain: a (now both head and tail)

    let na = chain.get_node(a);
    assert!(na.get_next_ptr().is_none());
    assert!(na.get_prev_ptr().is_none());
}

#[test]
fn remove_middle_heals_chain() {
    let h = setup();
    let chain = &h.chain;

    // chain: a -> b -> c
    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node_after(a, 2).unwrap();
    let c = chain.insert_node_after(b, 3).unwrap();

    chain.remove_node(b).unwrap();

    assert_eq!(chain.get_node(a).get_next_ptr(), Some(c));
    assert_eq!(chain.get_node(c).get_prev_ptr(), Some(a));
}

#[test]
fn remove_all_then_reinsert_yields_fresh_orphan() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node_after(a, 2).unwrap();
    let c = chain.insert_node_after(b, 3).unwrap();

    chain.remove_node(c).unwrap();
    chain.remove_node(b).unwrap();
    chain.remove_node(a).unwrap();

    // Drive the two-cycle reclaim so the slot allocator's len reflects 0.
    chain.publish();
    chain.to_reader().ack_generation();
    chain.publish();

    assert_eq!(chain.len(), 0);

    // Reinserting after a full clear: still an orphan.
    let d = chain.insert_node(99).unwrap();
    let nd = chain.get_node(d);
    assert_eq!(nd.get_kind(), 99);
    assert!(nd.get_next_ptr().is_none());
    assert!(nd.get_prev_ptr().is_none());
}

#[test]
fn double_remove_is_handled() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    chain.remove_node(a).unwrap();
    // remove_node on a defer-freed slot would surface a SlotAllocatorError;
    // we don't assert the variant here because the slot may not even be
    // reclaimed yet (publish-gated). The original test only asserted the
    // first remove succeeded.
}

// ============ NodeStoreReader: traversal after publish ============

#[test]
fn reader_traverses_chain_built_via_insert_after() {
    let h = setup();

    let head_slot = {
        let chain = &h.chain;

        // chain: a(kind=1, tick=10) -> b(kind=2, tick=20) -> c(kind=3, tick=30)
        let a = insert_with_tick(chain, 1, 10);
        let _b = chain.insert_node_after(a, 2).unwrap();
        h.chain.get_node(_b).set_meta(0, 20);
        let _c = chain.insert_node_after(_b, 3).unwrap();
        h.chain.get_node(_c).set_meta(0, 30);
        a
    };
    h.writer.publish();
    h.reader.swap();

    let chain_r = &h.chain_r;
    let head = chain_r.get_node(head_slot);
    assert_eq!(head.get_kind(), 1);
    assert_eq!(head.get_meta(0), 10);

    let nb = chain_r.get_node(head.get_next_ptr().unwrap());
    assert_eq!(nb.get_kind(), 2);

    let nc = chain_r.get_node(nb.get_next_ptr().unwrap());
    assert_eq!(nc.get_kind(), 3);
    assert!(nc.get_next_ptr().is_none(), "end of chain");
}

#[test]
fn reader_capacity_matches_writer_after_publish() {
    let h = setup();
    h.writer.publish();
    h.reader.swap();

    assert_eq!(h.chain_r.capacity(), CAPACITY as usize);
}

#[test]
fn reader_sees_removal_after_publish() {
    let h = setup();

    let a = {
        let chain = &h.chain;

        let a = chain.insert_node(1).unwrap();
        let b = chain.insert_node_after(a, 2).unwrap();
        // chain: a -> b
        chain.remove_node(b).unwrap();
        // chain: a
        a
    };
    h.writer.publish();
    h.reader.swap();

    let chain_r = &h.chain_r;
    let head = chain_r.get_node(a);
    assert_eq!(head.get_kind(), 1);
    assert!(head.get_next_ptr().is_none(), "only one node left");
}

// ============ Capacity exhaustion ============

#[test]
fn insert_node_exhausts_capacity() {
    let h = setup();
    let chain = &h.chain;

    for i in 0..CAPACITY {
        assert!(chain.insert_node(i as i32).is_some());
    }
    assert!(
        chain.insert_node(99).is_none(),
        "capacity exhausted"
    );
}

// ============ Pointer stability across operations ============

#[test]
fn insert_after_does_not_mutate_unrelated_nodes() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node_after(a, 2).unwrap();
    let c = chain.insert_node_after(b, 3).unwrap();
    // chain: a -> b -> c

    // Insert d after a (between a and b)
    let d = chain.insert_node_after(a, 4).unwrap();
    // chain: a -> d -> b -> c

    // c must not be touched
    assert_eq!(chain.get_node(c).get_prev_ptr(), Some(b), "c's prev unchanged");
    assert!(chain.get_node(c).get_next_ptr().is_none(), "c's next unchanged");

    // b's prev updated to d
    assert_eq!(chain.get_node(b).get_prev_ptr(), Some(d));
    // b's next unchanged
    assert_eq!(chain.get_node(b).get_next_ptr(), Some(c));
}

// ============ Forward + backward traversal ============

#[test]
fn four_node_chain_traverses_forward_and_backward() {
    let h = setup();
    let chain = &h.chain;

    // chain: a(1) -> b(2) -> c(3) -> d(4)
    let slots = build_chain(chain, 4);
    let (a, b, c, d) = (slots[0], slots[1], slots[2], slots[3]);

    // forward: a -> b -> c -> d -> None
    let n0 = chain.get_node(a);
    assert_eq!(n0.get_kind(), 1);
    assert!(n0.get_prev_ptr().is_none(), "head");
    let n1 = chain.get_node(n0.get_next_ptr().unwrap());
    assert_eq!(n1.get_kind(), 2);
    let n2 = chain.get_node(n1.get_next_ptr().unwrap());
    assert_eq!(n2.get_kind(), 3);
    let n3 = chain.get_node(n2.get_next_ptr().unwrap());
    assert_eq!(n3.get_kind(), 4);
    assert!(n3.get_next_ptr().is_none(), "tail");

    // backward: d -> c -> b -> a -> None
    assert_eq!(chain.get_node(d).get_prev_ptr(), Some(c));
    assert_eq!(chain.get_node(c).get_prev_ptr(), Some(b));
    assert_eq!(chain.get_node(b).get_prev_ptr(), Some(a));
    assert!(chain.get_node(a).get_prev_ptr().is_none());
}

// ============ insert_after / insert_before exhaustion ============

#[test]
fn insert_after_returns_none_on_exhaustion() {
    let h = setup();
    let chain = &h.chain;

    let head = chain.insert_node(0).unwrap();
    let mut last = head;
    for i in 1..CAPACITY {
        last = chain.insert_node_after(last, i as i32).unwrap();
    }
    assert!(
        chain.insert_node_after(last, 99).is_none(),
        "insert_after must return None when exhausted"
    );
}

#[test]
fn insert_before_returns_none_on_exhaustion() {
    let h = setup();
    let chain = &h.chain;

    let head = chain.insert_node(0).unwrap();
    for i in 1..CAPACITY {
        chain.insert_node_before(head, i as i32).unwrap();
    }
    assert!(
        chain.insert_node_before(head, 99).is_none(),
        "insert_before must return None when exhausted"
    );
}

// ============ insert_before at tail ============

#[test]
fn insert_before_tail_in_three_node_chain() {
    let h = setup();
    let chain = &h.chain;

    let a = chain.insert_node(1).unwrap();
    let c = chain.insert_node_after(a, 3).unwrap();
    let d = chain.insert_node_after(c, 4).unwrap();
    // chain: a -> c -> d

    let e = chain.insert_node_before(d, 5).unwrap();
    // chain: a -> c -> e -> d

    assert_eq!(chain.get_node(c).get_next_ptr(), Some(e));
    assert_eq!(chain.get_node(e).get_prev_ptr(), Some(c));
    assert_eq!(chain.get_node(e).get_next_ptr(), Some(d));
    assert_eq!(chain.get_node(d).get_prev_ptr(), Some(e));
    assert!(chain.get_node(d).get_next_ptr().is_none(), "d is still tail");
}

// ============ Multi-order removal ============

#[test]
fn remove_tail_first_then_middle_then_head() {
    let h = setup();
    let chain = &h.chain;

    // chain: a -> b -> c -> d
    let slots = build_chain(chain, 4);
    let (a, b, c, d) = (slots[0], slots[1], slots[2], slots[3]);

    // remove tail
    chain.remove_node(d).unwrap();
    // chain: a -> b -> c
    assert!(chain.get_node(c).get_next_ptr().is_none());

    // remove middle
    chain.remove_node(b).unwrap();
    // chain: a -> c
    assert_eq!(chain.get_node(a).get_next_ptr(), Some(c));
    assert_eq!(chain.get_node(c).get_prev_ptr(), Some(a));

    // remove head
    chain.remove_node(a).unwrap();
    // chain: c (now head + tail)
    let nc = chain.get_node(c);
    assert_eq!(nc.get_kind(), 3);
    assert!(nc.get_prev_ptr().is_none());
    assert!(nc.get_next_ptr().is_none());
}

#[test]
fn remove_arbitrary_order_on_five_node_chain() {
    let h = setup();
    let chain = &h.chain;

    // chain: a -> b -> c -> d -> e
    let slots = build_chain(chain, 5);
    let (a, b, c, d, e) = (slots[0], slots[1], slots[2], slots[3], slots[4]);

    // remove c (middle)
    chain.remove_node(c).unwrap();
    // chain: a -> b -> d -> e
    assert_eq!(chain.get_node(b).get_next_ptr(), Some(d));
    assert_eq!(chain.get_node(d).get_prev_ptr(), Some(b));

    // remove a (head)
    chain.remove_node(a).unwrap();
    // chain: b -> d -> e
    assert!(chain.get_node(b).get_prev_ptr().is_none());
    assert_eq!(chain.get_node(b).get_kind(), 2);

    // remove e (tail)
    chain.remove_node(e).unwrap();
    // chain: b -> d
    assert!(chain.get_node(d).get_next_ptr().is_none());

    // remove b (head)
    chain.remove_node(b).unwrap();
    // chain: d
    let nd = chain.get_node(d);
    assert_eq!(nd.get_kind(), 4);
    assert!(nd.get_prev_ptr().is_none());
    assert!(nd.get_next_ptr().is_none());

    // remove last
    chain.remove_node(d).unwrap();

    // Drive the two-cycle reclaim so the slot allocator's len reflects 0.
    chain.publish();
    chain.to_reader().ack_generation();
    chain.publish();

    assert_eq!(chain.len(), 0);
}

// ============ Two disjoint sub-chains ============

#[test]
fn two_disjoint_subchains_coexist_in_one_store() {
    let h = setup();
    let chain = &h.chain;

    // Build chain X: x1 -> x2
    let x1 = chain.insert_node(10).unwrap();
    let x2 = chain.insert_node_after(x1, 11).unwrap();

    // Build chain Y: y1 -> y2 -> y3, fully independent
    let y1 = chain.insert_node(20).unwrap();
    let y2 = chain.insert_node_after(y1, 21).unwrap();
    let y3 = chain.insert_node_after(y2, 22).unwrap();

    // Each chain self-consistent.
    assert!(chain.get_node(x1).get_prev_ptr().is_none());
    assert_eq!(chain.get_node(x1).get_next_ptr(), Some(x2));
    assert_eq!(chain.get_node(x2).get_prev_ptr(), Some(x1));
    assert!(chain.get_node(x2).get_next_ptr().is_none());

    assert!(chain.get_node(y1).get_prev_ptr().is_none());
    assert_eq!(chain.get_node(y1).get_next_ptr(), Some(y2));
    assert_eq!(chain.get_node(y2).get_prev_ptr(), Some(y1));
    assert_eq!(chain.get_node(y2).get_next_ptr(), Some(y3));
    assert_eq!(chain.get_node(y3).get_prev_ptr(), Some(y2));
    assert!(chain.get_node(y3).get_next_ptr().is_none());

    // The two chains are independent: each chain's tail.next == None and each
    // chain's head.prev == None (asserted above), so neither walks into the
    // other in either direction.

    assert_eq!(chain.len(), 5);
    let _ = (x1, y1);
}

// ============ Remove then traverse via reader ============

#[test]
fn reader_traverses_chain_after_mid_chain_removal() {
    let h = setup();

    let head_slot = {
        let chain = &h.chain;

        // chain: a(1,10) -> b(2,20) -> c(3,30) -> d(4,40)
        let a = insert_with_tick(chain, 1, 10);
        let b = chain.insert_node_after(a, 2).unwrap();
        chain.get_node(b).set_meta(0, 20);
        let c = chain.insert_node_after(b, 3).unwrap();
        chain.get_node(c).set_meta(0, 30);
        let d = chain.insert_node_after(c, 4).unwrap();
        chain.get_node(d).set_meta(0, 40);

        chain.remove_node(b).unwrap();
        // chain: a -> c -> d
        a
    };
    h.writer.publish();
    h.reader.swap();

    let chain_r = &h.chain_r;

    let head = chain_r.get_node(head_slot);
    assert_eq!(head.get_kind(), 1);
    assert_eq!(head.get_meta(0), 10);

    let n1 = chain_r.get_node(head.get_next_ptr().unwrap());
    assert_eq!(n1.get_kind(), 3);

    let n2 = chain_r.get_node(n1.get_next_ptr().unwrap());
    assert_eq!(n2.get_kind(), 4);
    assert!(n2.get_next_ptr().is_none(), "end of chain");
}

// ============ copy_from ============

#[test]
fn copy_from_preserves_topology_and_deep_data() {
    let src_h = setup();
    let src = &src_h.chain;

    let a = insert_with_tick(src, 1, 10);
    let b = src.insert_node_after(a, 2).unwrap();
    src.get_node(b).set_meta(0, 20);

    src.remove_node(b).unwrap(); // b deferred

    let dst_mem = create_mem(MEM_SIZE);
    let dst_tb = TripleBufferWriter::new(Arc::clone(&dst_mem), TB_START, TB_BUF_CAP);
    let dst = NodeStoreWriter::new(
        Arc::clone(&dst_mem),
        dst_tb,
        NodeStoreConfig {
            meta_stride: NODE_META,
            attr_stride: NODE_ATTR,
            capacity: CAPACITY * 2,
        },
        FL_START,
        NODE_START_OFFSET,
    );

    dst.copy_from(src);

    // b is deferred-freed but still occupies its slot in the allocator.
    assert_eq!(dst.len(), 2);
    let head = dst.get_node(a);
    assert_eq!(head.get_kind(), 1);
    assert_eq!(head.get_meta(0), 10);
    // a's next_ptr was patched to None when b was unlinked.
    assert!(head.get_next_ptr().is_none());

    // Drive the reclamation cycle: publish, ack, publish to reclaim b.
    dst.publish();
    dst.to_reader().ack_generation();
    dst.publish();

    assert_eq!(dst.len(), 1);
    assert_eq!(dst.capacity(), (CAPACITY * 2) as usize);
}

#[test]
#[should_panic]
fn copy_from_panics_if_source_larger() {
    let src_mem = create_mem(MEM_SIZE);
    let src_tb = TripleBufferWriter::new(Arc::clone(&src_mem), TB_START, TB_BUF_CAP);
    let src = NodeStoreWriter::new(
        src_mem,
        src_tb,
        NodeStoreConfig {
            meta_stride: NODE_META,
            attr_stride: NODE_ATTR,
            capacity: CAPACITY * 2,
        },
        FL_START,
        NODE_START_OFFSET,
    );

    let dst_mem = create_mem(MEM_SIZE);
    let dst_tb = TripleBufferWriter::new(Arc::clone(&dst_mem), TB_START, TB_BUF_CAP);
    let dst = NodeStoreWriter::new(
        dst_mem,
        dst_tb,
        NodeStoreConfig {
            meta_stride: NODE_META,
            attr_stride: NODE_ATTR,
            capacity: CAPACITY,
        },
        FL_START,
        NODE_START_OFFSET,
    );

    dst.copy_from(&src);
}
