use proptest::prelude::*;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;
use synaptic_kernel::topology::network::network_config::NetworkConfig;
use synaptic_kernel::topology::network::network_writer::NetworkWriter;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;
const MEM_SIZE: usize = 131072;
const TB_START: usize = 0;
const TB_BUF_CAP: u32 = 32768;
const NODE_CAPACITY: u32 = 32;
const SYNAPSE_CAPACITY: u32 = 64;
const NODE_START_OFFSET: usize = 0;
const NODE_FL_START: usize = 80000;

fn net_config() -> NetworkConfig {
    NetworkConfig {
        node_capacity: NODE_CAPACITY,
        node_meta_stride: NODE_META,
        node_attr_stride: NODE_ATTR,
        synapse_capacity: SYNAPSE_CAPACITY,
        synapse_meta_stride: SYNAPSE_META,
        synapse_attr_stride: SYNAPSE_ATTR,
    }
}

type TestNetwork = NetworkWriter;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

struct TestHarness {
    /// `node_chain` and `synapse_chain` are clones of the same `NetworkWriter`.
    node_chain: TestNetwork,
    synapse_chain: TestNetwork,
}

fn setup() -> TestHarness {
    let mem = create_mem(MEM_SIZE);
    let writer = TripleBufferWriter::new(Arc::clone(&mem), TB_START, TB_BUF_CAP);
    let network = NetworkWriter::new(
        Arc::clone(&mem),
        writer.clone(),
        net_config(),
        NODE_FL_START,
        NODE_START_OFFSET,
    );
    let node_chain = network.clone();
    let synapse_chain = network;
    TestHarness {
        node_chain,
        synapse_chain,
    }
}

/// Tracks the graph state for verification
struct GraphState {
    node_slots: Vec<SlotId>,
    /// (synapse_slot, source_node, target_node)
    synapse_edges: Vec<(SlotId, SlotId, SlotId)>,
}

impl GraphState {
    fn new() -> Self {
        GraphState {
            node_slots: Vec::new(),
            synapse_edges: Vec::new(),
        }
    }
}

/// Verify all synapse chain invariants:
/// - For each node: outgoing chain is a valid doubly-linked list
/// - For each node: incoming chain is a valid doubly-linked list
/// - Every active synapse is reachable from its source's outgoing chain AND target's incoming chain
/// - head/tail pointers are consistent: head.prev is None, tail.next is None
fn verify_synapse_integrity(h: &TestHarness, state: &GraphState) {
    // Collect all active synapses by source and by target
    let mut outgoing_by_node: std::collections::HashMap<SlotId, Vec<SlotId>> =
        std::collections::HashMap::new();
    let mut incoming_by_node: std::collections::HashMap<SlotId, Vec<SlotId>> =
        std::collections::HashMap::new();

    for &(syn_slot, src, tgt) in &state.synapse_edges {
        outgoing_by_node.entry(src).or_default().push(syn_slot);
        incoming_by_node.entry(tgt).or_default().push(syn_slot);
    }

    // Verify outgoing chains for all nodes
    for &node_slot in &state.node_slots {
        let node = h.node_chain.get_node(node_slot);
        let expected_out = outgoing_by_node.get(&node_slot);
        let expected_count = expected_out.map_or(0, |v| v.len());

        if expected_count == 0 {
            assert!(
                node.get_outgoing_synapse_head().is_none(),
                "node {} has no outgoing synapses but head is set",
                node_slot
            );
            assert!(
                node.get_outgoing_synapse_tail().is_none(),
                "node {} has no outgoing synapses but tail is set",
                node_slot
            );
            continue;
        }

        // Walk outgoing chain
        let head = node.get_outgoing_synapse_head().expect(
            "node has outgoing synapses but head is None",
        );

        let head_syn = h.synapse_chain.get_synapse(head);
        assert!(
            head_syn.get_outgoing_prev_ptr().is_none(),
            "outgoing head synapse {}'s prev must be None",
            head
        );

        let mut visited = Vec::new();
        let mut current_opt: Option<SlotId> = Some(head);
        let mut last: Option<SlotId> = None;
        let mut guard = 0;
        while let Some(current) = current_opt {
            visited.push(current);
            let syn = h.synapse_chain.get_synapse(current);

            // Verify source pointer
            assert_eq!(
                syn.get_source_ptr(),
                node_slot,
                "synapse {} source should be {} but is {}",
                current,
                node_slot,
                syn.get_source_ptr()
            );

            last = Some(current);
            current_opt = syn.get_outgoing_next_ptr();

            // Verify backward link
            if let Some(next) = current_opt {
                let next_syn = h.synapse_chain.get_synapse(next);
                assert_eq!(
                    next_syn.get_outgoing_prev_ptr(),
                    Some(current),
                    "outgoing backward link broken at synapse {}",
                    next
                );
            }

            guard += 1;
            assert!(
                guard <= SYNAPSE_CAPACITY as usize,
                "cycle in outgoing chain of node {}",
                node_slot
            );
        }

        // Verify tail
        assert_eq!(
            node.get_outgoing_synapse_tail(),
            last,
            "node {} outgoing tail should be {:?} but is {:?}",
            node_slot,
            last,
            node.get_outgoing_synapse_tail()
        );

        // All expected synapses should be visited
        let expected_set: std::collections::HashSet<SlotId> =
            expected_out.unwrap().iter().cloned().collect();
        let visited_set: std::collections::HashSet<SlotId> = visited.iter().cloned().collect();
        assert_eq!(
            expected_set, visited_set,
            "node {} outgoing: expected {:?} but visited {:?}",
            node_slot, expected_set, visited_set
        );
    }

    // Verify incoming chains for all nodes
    for &node_slot in &state.node_slots {
        let node = h.node_chain.get_node(node_slot);
        let expected_in = incoming_by_node.get(&node_slot);
        let expected_count = expected_in.map_or(0, |v| v.len());

        if expected_count == 0 {
            assert!(
                node.get_incoming_synapse_head().is_none(),
                "node {} has no incoming synapses but head is set",
                node_slot
            );
            assert!(
                node.get_incoming_synapse_tail().is_none(),
                "node {} has no incoming synapses but tail is set",
                node_slot
            );
            continue;
        }

        let head = node.get_incoming_synapse_head().expect(
            "node has incoming synapses but head is None",
        );

        let head_syn = h.synapse_chain.get_synapse(head);
        assert!(
            head_syn.get_incoming_prev_ptr().is_none(),
            "incoming head synapse {}'s prev must be None",
            head
        );

        let mut visited = Vec::new();
        let mut current_opt: Option<SlotId> = Some(head);
        let mut last: Option<SlotId> = None;
        let mut guard = 0;
        while let Some(current) = current_opt {
            visited.push(current);
            let syn = h.synapse_chain.get_synapse(current);

            // Verify target pointer
            assert_eq!(
                syn.get_target_ptr(),
                node_slot,
                "synapse {} target should be {} but is {}",
                current,
                node_slot,
                syn.get_target_ptr()
            );

            last = Some(current);
            current_opt = syn.get_incoming_next_ptr();

            if let Some(next) = current_opt {
                let next_syn = h.synapse_chain.get_synapse(next);
                assert_eq!(
                    next_syn.get_incoming_prev_ptr(),
                    Some(current),
                    "incoming backward link broken at synapse {}",
                    next
                );
            }

            guard += 1;
            assert!(
                guard <= SYNAPSE_CAPACITY as usize,
                "cycle in incoming chain of node {}",
                node_slot
            );
        }

        assert_eq!(
            node.get_incoming_synapse_tail(),
            last,
            "node {} incoming tail should be {:?} but is {:?}",
            node_slot,
            last,
            node.get_incoming_synapse_tail()
        );

        let expected_set: std::collections::HashSet<SlotId> =
            expected_in.unwrap().iter().cloned().collect();
        let visited_set: std::collections::HashSet<SlotId> = visited.iter().cloned().collect();
        assert_eq!(
            expected_set, visited_set,
            "node {} incoming: expected {:?} but visited {:?}",
            node_slot, expected_set, visited_set
        );
    }
}

// ============ Operations for property-based testing ============

#[derive(Debug, Clone)]
enum SynapseOp {
    AddNode,
    Connect(usize, usize), // indices into node_slots
    Disconnect(usize),     // index into synapse_edges
}

fn synapse_op_strategy() -> impl Strategy<Value = SynapseOp> {
    prop_oneof![
        2 => Just(SynapseOp::AddNode),
        4 => (0..32usize, 0..32usize).prop_map(|(a, b)| SynapseOp::Connect(a, b)),
        3 => (0..64usize).prop_map(SynapseOp::Disconnect),
    ]
}

proptest! {
    #[test]
    fn synapse_chain_random_ops_preserve_dual_linked_invariants(
        ops in proptest::collection::vec(synapse_op_strategy(), 1..150)
    ) {
        let h = setup();
        let mut state = GraphState::new();
        let mut kind_counter = 0i32;

        // Seed at least 2 nodes
        for i in 0..2 {
            if let Some(slot) = h.node_chain.insert_node(i) {
                state.node_slots.push(slot);
            }
        }

        for op in ops {
            match op {
                SynapseOp::AddNode => {
                    if state.node_slots.len() < NODE_CAPACITY as usize {
                        kind_counter += 1;
                        if let Some(slot) = h.node_chain.insert_node(kind_counter) {
                            state.node_slots.push(slot);
                        }
                    }
                }
                SynapseOp::Connect(src_idx, tgt_idx) => {
                    if state.node_slots.len() >= 2
                        && state.synapse_edges.len() < SYNAPSE_CAPACITY as usize
                    {
                        let src = state.node_slots[src_idx % state.node_slots.len()];
                        let tgt = state.node_slots[tgt_idx % state.node_slots.len()];
                        kind_counter += 1;
                        if let Some(syn_slot) = h.synapse_chain.connect(src, tgt, kind_counter) {
                            state.synapse_edges.push((syn_slot, src, tgt));
                        }
                    }
                }
                SynapseOp::Disconnect(idx) => {
                    if !state.synapse_edges.is_empty() {
                        let actual_idx = idx % state.synapse_edges.len();
                        let (syn_slot, _, _) = state.synapse_edges.remove(actual_idx);
                        let _ = h.synapse_chain.disconnect_synapse(syn_slot);
                    }
                }
            }

            // INVARIANT: all synapse chains are valid after every operation
            verify_synapse_integrity(&h, &state);
        }
    }

    #[test]
    fn synapse_chain_connect_disconnect_all_leaves_clean(
        edge_count in 1..30usize
    ) {
        let h = setup();
        let mut state = GraphState::new();

        // Create 4 nodes
        for i in 0..4 {
            let slot = h.node_chain.insert_node(i).unwrap();
            state.node_slots.push(slot);
        }

        // Connect edges
        for i in 0..edge_count {
            let src = state.node_slots[i % state.node_slots.len()];
            let tgt = state.node_slots[(i + 1) % state.node_slots.len()];
            if let Some(syn_slot) = h.synapse_chain.connect(src, tgt, i as i32) {
                state.synapse_edges.push((syn_slot, src, tgt));
            }
        }

        verify_synapse_integrity(&h, &state);

        // Disconnect all
        while let Some((syn_slot, _, _)) = state.synapse_edges.pop() {
            h.synapse_chain.disconnect_synapse(syn_slot).unwrap();
        }

        // All nodes should have clean synapse pointers
        for &node_slot in &state.node_slots {
            let node = h.node_chain.get_node(node_slot);
            prop_assert!(node.get_outgoing_synapse_head().is_none());
            prop_assert!(node.get_outgoing_synapse_tail().is_none());
            prop_assert!(node.get_incoming_synapse_head().is_none());
            prop_assert!(node.get_incoming_synapse_tail().is_none());
        }
    }

    #[test]
    fn synapse_chain_self_loops_work(
        count in 1..16usize
    ) {
        let h = setup();
        let mut state = GraphState::new();

        let node = h.node_chain.insert_node(1).unwrap();
        state.node_slots.push(node);

        // Create self-loops
        for i in 0..count {
            if let Some(syn_slot) = h.synapse_chain.connect(node, node, i as i32) {
                state.synapse_edges.push((syn_slot, node, node));
            }
        }

        verify_synapse_integrity(&h, &state);

        // Disconnect all self-loops
        while let Some((syn_slot, _, _)) = state.synapse_edges.pop() {
            h.synapse_chain.disconnect_synapse(syn_slot).unwrap();
        }

        let n = h.node_chain.get_node(node);
        prop_assert!(n.get_outgoing_synapse_head().is_none());
        prop_assert!(n.get_outgoing_synapse_tail().is_none());
        prop_assert!(n.get_incoming_synapse_head().is_none());
        prop_assert!(n.get_incoming_synapse_tail().is_none());
    }
}

// ============ Explicit edge cases ============

#[test]
fn disconnect_middle_of_outgoing_chain() {
    let h = setup();

    let a = h.node_chain.insert_node(1).unwrap();
    let b = h.node_chain.insert_node(2).unwrap();

    let s1 = h.synapse_chain.connect(a, b, 10).unwrap();
    let s2 = h.synapse_chain.connect(a, b, 20).unwrap();
    let s3 = h.synapse_chain.connect(a, b, 30).unwrap();

    // Disconnect middle
    h.synapse_chain.disconnect_synapse(s2).unwrap();

    // Chain: s1 -> s3
    let syn1 = h.synapse_chain.get_synapse(s1);
    let syn3 = h.synapse_chain.get_synapse(s3);

    assert_eq!(syn1.get_outgoing_next_ptr(), Some(s3));
    assert_eq!(syn3.get_outgoing_prev_ptr(), Some(s1));
    assert!(syn1.get_outgoing_prev_ptr().is_none());
    assert!(syn3.get_outgoing_next_ptr().is_none());

    let node_a = h.node_chain.get_node(a);
    assert_eq!(node_a.get_outgoing_synapse_head(), Some(s1));
    assert_eq!(node_a.get_outgoing_synapse_tail(), Some(s3));
}

#[test]
fn disconnect_head_of_incoming_chain() {
    let h = setup();

    let a = h.node_chain.insert_node(1).unwrap();
    let b = h.node_chain.insert_node(2).unwrap();
    let c = h.node_chain.insert_node(3).unwrap();

    // All point to b
    let s1 = h.synapse_chain.connect(a, b, 10).unwrap();
    let s2 = h.synapse_chain.connect(c, b, 20).unwrap();

    // Disconnect head of b's incoming chain
    h.synapse_chain.disconnect_synapse(s1).unwrap();

    let node_b = h.node_chain.get_node(b);
    assert_eq!(node_b.get_incoming_synapse_head(), Some(s2));
    assert_eq!(node_b.get_incoming_synapse_tail(), Some(s2));

    let syn2 = h.synapse_chain.get_synapse(s2);
    assert!(syn2.get_incoming_prev_ptr().is_none());
    assert!(syn2.get_incoming_next_ptr().is_none());
}

#[test]
fn fan_out_and_fan_in_topology() {
    let h = setup();
    let mut state = GraphState::new();

    let hub = h.node_chain.insert_node(0).unwrap();
    state.node_slots.push(hub);

    // Create 8 spoke nodes
    let mut spokes = Vec::new();
    for i in 1..=8 {
        let s = h.node_chain.insert_node(i).unwrap();
        state.node_slots.push(s);
        spokes.push(s);
    }

    // Fan-out: hub -> all spokes
    for &spoke in &spokes {
        let syn = h.synapse_chain.connect(hub, spoke, 1).unwrap();
        state.synapse_edges.push((syn, hub, spoke));
    }

    // Fan-in: all spokes -> hub
    for &spoke in &spokes {
        let syn = h.synapse_chain.connect(spoke, hub, 2).unwrap();
        state.synapse_edges.push((syn, spoke, hub));
    }

    verify_synapse_integrity(&h, &state);

    // Disconnect all fan-out
    let fan_out_syns: Vec<_> = state
        .synapse_edges
        .iter()
        .filter(|&&(_, src, _)| src == hub)
        .map(|&(s, _, _)| s)
        .collect();
    for syn in fan_out_syns {
        h.synapse_chain.disconnect_synapse(syn).unwrap();
        state.synapse_edges.retain(|&(s, _, _)| s != syn);
    }

    verify_synapse_integrity(&h, &state);

    // Hub should have no outgoing but still have incoming
    let hub_node = h.node_chain.get_node(hub);
    assert!(hub_node.get_outgoing_synapse_head().is_none());
    assert!(hub_node.get_outgoing_synapse_tail().is_none());
    assert!(hub_node.get_incoming_synapse_head().is_some());
}
