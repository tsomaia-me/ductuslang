use proptest::prelude::*;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;
use synaptic_kernel::topology::node::node_store_config::NodeStoreConfig;
use synaptic_kernel::topology::node::node_store_writer::NodeStoreWriter;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const MEM_SIZE: usize = 65536;
const TB_START: usize = 0;
const TB_BUF_CAP: u32 = 16384;
const FL_START: usize = 50000;
const NODE_START_OFFSET: usize = 0;
const CAPACITY: u32 = 64;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

fn setup_chain() -> NodeStoreWriter {
    let mem = create_mem(MEM_SIZE);
    let writer = TripleBufferWriter::new(Arc::clone(&mem), TB_START, TB_BUF_CAP);
    NodeStoreWriter::new(
        mem,
        writer,
        NodeStoreConfig {
            meta_stride: NODE_META,
            attr_stride: NODE_ATTR,
            capacity: CAPACITY,
        },
        FL_START,
        NODE_START_OFFSET,
    )
}

/// Test-side model of a single sub-chain. The kernel does not track "the head"
/// — these tests pin one explicitly so they can assert structural integrity
/// of a particular sub-chain. `slots` is in chain order: slots[0] is head,
/// slots[len-1] is tail.
#[derive(Default)]
struct ChainModel {
    slots: Vec<SlotId>,
}

impl ChainModel {
    fn head(&self) -> Option<SlotId> {
        self.slots.first().copied()
    }
}

#[derive(Debug, Clone)]
enum ChainOp {
    PushFront,
    InsertAfter(usize),
    InsertBefore(usize),
    Remove(usize),
}

fn chain_op_strategy() -> impl Strategy<Value = ChainOp> {
    prop_oneof![
        3 => Just(ChainOp::PushFront),
        2 => (0..64usize).prop_map(ChainOp::InsertAfter),
        2 => (0..64usize).prop_map(ChainOp::InsertBefore),
        2 => (0..64usize).prop_map(ChainOp::Remove),
    ]
}

/// Walk from the model's head and verify:
/// - Head's prev is None; tail's next is None
/// - Backward links are consistent (node.next.prev == node)
/// - Traversal visits exactly the model's slots, in the same order
fn verify_chain_integrity(chain: &NodeStoreWriter, model: &ChainModel) {
    let Some(head_slot) = model.head() else {
        return;
    };

    let head = chain.get_node(head_slot);
    assert!(head.get_prev_ptr().is_none(), "head's prev must be None");

    let mut visited = Vec::with_capacity(model.slots.len());
    let mut current = head_slot;
    let mut guard = 0;
    loop {
        visited.push(current);
        let node = chain.get_node(current);
        let next = node.get_next_ptr();

        let Some(next_slot) = next else {
            break;
        };

        let next_node = chain.get_node(next_slot);
        assert_eq!(
            next_node.get_prev_ptr(),
            Some(current),
            "backward link broken: {}->{}, but {}.prev = {:?}",
            current,
            next_slot,
            next_slot,
            next_node.get_prev_ptr()
        );

        current = next_slot;
        guard += 1;
        assert!(guard <= CAPACITY as usize, "cycle detected in chain traversal");
    }

    assert_eq!(
        visited, model.slots,
        "traversal order mismatch: visited {:?}, expected {:?}",
        visited, model.slots
    );
}

// ============ Explicit insert_before on chain head ============

#[test]
fn insert_before_chain_head_makes_new_head() {
    let chain = setup_chain();

    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node_before(a, 2).unwrap();

    // Chain: b -> a
    assert!(chain.get_node(b).get_prev_ptr().is_none(), "b is now the head");
    assert_eq!(chain.get_node(b).get_next_ptr(), Some(a));
    assert_eq!(chain.get_node(a).get_prev_ptr(), Some(b));
    assert!(chain.get_node(a).get_next_ptr().is_none());
}

#[test]
fn insert_before_head_twice_builds_correct_chain() {
    let chain = setup_chain();

    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node_before(a, 2).unwrap();
    let c = chain.insert_node_before(b, 3).unwrap();

    // Chain: c -> b -> a
    assert!(chain.get_node(c).get_prev_ptr().is_none());
    assert_eq!(chain.get_node(c).get_next_ptr(), Some(b));
    assert_eq!(chain.get_node(b).get_prev_ptr(), Some(c));
    assert_eq!(chain.get_node(b).get_next_ptr(), Some(a));
    assert_eq!(chain.get_node(a).get_prev_ptr(), Some(b));
    assert!(chain.get_node(a).get_next_ptr().is_none());

    verify_chain_integrity(&chain, &ChainModel { slots: vec![c, b, a] });
}

#[test]
fn remove_chain_head_promotes_successor() {
    let chain = setup_chain();

    let a = chain.insert_node(1).unwrap();
    let b = chain.insert_node_before(a, 2).unwrap();
    // chain: b -> a

    chain.remove_node(b).unwrap();
    // chain: a (a is the new head)

    assert!(chain.get_node(a).get_prev_ptr().is_none());
    assert!(chain.get_node(a).get_next_ptr().is_none());

    verify_chain_integrity(&chain, &ChainModel { slots: vec![a] });
}

// ============ Property-based tests ============

proptest! {
    #[test]
    fn node_store_random_ops_preserve_doubly_linked_invariants(
        ops in proptest::collection::vec(chain_op_strategy(), 1..100)
    ) {
        let chain = setup_chain();
        let mut model = ChainModel::default();
        let mut kind_counter = 0i32;

        for op in ops {
            match op {
                ChainOp::PushFront => {
                    if model.slots.len() < CAPACITY as usize {
                        kind_counter += 1;
                        let new_slot = if let Some(head) = model.head() {
                            chain.insert_node_before(head, kind_counter)
                        } else {
                            chain.insert_node(kind_counter)
                        };
                        if let Some(slot) = new_slot {
                            model.slots.insert(0, slot);
                        }
                    }
                }
                ChainOp::InsertAfter(idx) => {
                    if !model.slots.is_empty() && model.slots.len() < CAPACITY as usize {
                        let i = idx % model.slots.len();
                        let target = model.slots[i];
                        kind_counter += 1;
                        if let Some(slot) = chain.insert_node_after(target, kind_counter) {
                            model.slots.insert(i + 1, slot);
                        }
                    }
                }
                ChainOp::InsertBefore(idx) => {
                    if !model.slots.is_empty() && model.slots.len() < CAPACITY as usize {
                        let i = idx % model.slots.len();
                        let target = model.slots[i];
                        kind_counter += 1;
                        if let Some(slot) = chain.insert_node_before(target, kind_counter) {
                            model.slots.insert(i, slot);
                        }
                    }
                }
                ChainOp::Remove(idx) => {
                    if !model.slots.is_empty() {
                        let i = idx % model.slots.len();
                        let slot = model.slots.remove(i);
                        let _ = chain.remove_node(slot);
                    }
                }
            }

            verify_chain_integrity(&chain, &model);
        }
    }

    #[test]
    fn node_store_insert_remove_all_leaves_empty(
        count in 1..32usize
    ) {
        let chain = setup_chain();
        let mut slots: Vec<SlotId> = Vec::new();

        for i in 0..count {
            if let Some(s) = chain.insert_node(i as i32) {
                slots.push(s);
            }
        }

        // Remove all in reverse order. remove_node defers frees, so a single
        // publish leaves them on the previous-list; reclaim takes a full
        // publish + ack + publish cycle.
        while let Some(s) = slots.pop() {
            let _ = chain.remove_node(s);
        }

        chain.publish();
        chain.to_reader().ack_generation();
        chain.publish();

        prop_assert_eq!(chain.len(), 0, "store should be empty after removing all nodes");
    }
}
