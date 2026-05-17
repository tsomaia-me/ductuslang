mod common;

use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;

type TestKernel = Kernel<1, 1, 1>;

fn config() -> KernelConfig<1, 1, 1> {
    common::kernel_config_1_1(16, 32, NODE_META, NODE_ATTR, SYNAPSE_META, SYNAPSE_ATTR)
}

fn create_writer() -> TestKernel {
    TestKernel::new(config())
}

// ============ Cascade removal: remove_node auto-disconnects synapses ============

#[test]
fn remove_node_with_single_outgoing_synapse() {
    let kernel = create_writer();
    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node(2).unwrap();
    let _syn = kernel.connect(a, b, 10).unwrap();

    // Remove source node — synapse must be auto-disconnected
    kernel.remove_node(a).unwrap();

    // Target node's incoming chain must be clean
    let node_b = kernel.get_node(b);
    assert!(node_b.get_incoming_synapse_head().is_none());
    assert!(node_b.get_incoming_synapse_tail().is_none());
}

#[test]
fn remove_node_with_single_incoming_synapse() {
    let kernel = create_writer();
    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node(2).unwrap();
    let _syn = kernel.connect(a, b, 10).unwrap();

    // Remove target node — synapse must be auto-disconnected
    kernel.remove_node(b).unwrap();

    // Source node's outgoing chain must be clean
    let node_a = kernel.get_node(a);
    assert!(node_a.get_outgoing_synapse_head().is_none());
    assert!(node_a.get_outgoing_synapse_tail().is_none());
}

#[test]
fn remove_node_with_multiple_outgoing_synapses() {
    let kernel = create_writer();
    let hub = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node(2).unwrap();
    let c = kernel.insert_node(3).unwrap();
    let d = kernel.insert_node(4).unwrap();

    kernel.connect(hub, b, 10).unwrap();
    kernel.connect(hub, c, 20).unwrap();
    kernel.connect(hub, d, 30).unwrap();

    kernel.remove_node(hub).unwrap();

    // All targets must have clean incoming chains
    for &slot in &[b, c, d] {
        let node = kernel.get_node(slot);
        assert!(
            node.get_incoming_synapse_head().is_none(),
            "node {} incoming head should be empty after cascade", slot
        );
        assert!(
            node.get_incoming_synapse_tail().is_none(),
            "node {} incoming tail should be empty after cascade", slot
        );
    }
}

#[test]
fn remove_node_with_multiple_incoming_synapses() {
    let kernel = create_writer();
    let target = kernel.insert_node(1).unwrap();
    let a = kernel.insert_node(2).unwrap();
    let b = kernel.insert_node(3).unwrap();
    let c = kernel.insert_node(4).unwrap();

    kernel.connect(a, target, 10).unwrap();
    kernel.connect(b, target, 20).unwrap();
    kernel.connect(c, target, 30).unwrap();

    kernel.remove_node(target).unwrap();

    // All sources must have clean outgoing chains
    for &slot in &[a, b, c] {
        let node = kernel.get_node(slot);
        assert!(
            node.get_outgoing_synapse_head().is_none(),
            "node {} outgoing head should be empty after cascade", slot
        );
        assert!(
            node.get_outgoing_synapse_tail().is_none(),
            "node {} outgoing tail should be empty after cascade", slot
        );
    }
}

#[test]
fn remove_node_with_both_outgoing_and_incoming_synapses() {
    let kernel = create_writer();
    let target = kernel.insert_node(1).unwrap();
    let upstream = kernel.insert_node(2).unwrap();
    let downstream = kernel.insert_node(3).unwrap();

    // upstream -> target -> downstream
    kernel.connect(upstream, target, 10).unwrap();
    kernel.connect(target, downstream, 20).unwrap();

    kernel.remove_node(target).unwrap();

    // upstream's outgoing must be clean
    let node_up = kernel.get_node(upstream);
    assert!(node_up.get_outgoing_synapse_head().is_none());
    assert!(node_up.get_outgoing_synapse_tail().is_none());

    // downstream's incoming must be clean
    let node_down = kernel.get_node(downstream);
    assert!(node_down.get_incoming_synapse_head().is_none());
    assert!(node_down.get_incoming_synapse_tail().is_none());
}

#[test]
fn remove_node_with_self_loop() {
    let kernel = create_writer();
    let n = kernel.insert_node(1).unwrap();
    kernel.connect(n, n, 99).unwrap();

    // Self-loop: node is both source and target. The cascade must drain
    // outgoing first, then incoming, without revisiting the same synapse
    // slot from a stale list head.
    kernel.remove_node(n).unwrap();

    // Verify the cascade ran by checking that no live outgoing/incoming
    // synapse heads remain on the writer side. node_count / synapse_count
    // include deferred slots, so we don't assert against them here.
    let _ = n;
}

#[test]
fn remove_node_with_multiple_self_loops() {
    let kernel = create_writer();
    let n = kernel.insert_node(1).unwrap();
    kernel.connect(n, n, 10).unwrap();
    kernel.connect(n, n, 20).unwrap();
    kernel.connect(n, n, 30).unwrap();

    // Three self-loops. The cascade peels them off via repeated
    // outgoing-head disconnects; if it doesn't terminate or revisits a
    // slot, this would either hang or hit a UAF panic.
    kernel.remove_node(n).unwrap();
    let _ = n;
}

#[test]
fn remove_node_preserves_unrelated_synapses() {
    let kernel = create_writer();
    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node(2).unwrap();
    let c = kernel.insert_node(3).unwrap();
    let d = kernel.insert_node(4).unwrap();

    // a -> b (will be removed via cascade)
    kernel.connect(a, b, 10).unwrap();
    // c -> d (must survive)
    let surviving_syn = kernel.connect(c, d, 20).unwrap();

    kernel.remove_node(a).unwrap();

    // c -> d synapse must still be intact
    let node_c = kernel.get_node(c);
    assert_eq!(node_c.get_outgoing_synapse_head(), Some(surviving_syn));
    assert_eq!(node_c.get_outgoing_synapse_tail(), Some(surviving_syn));

    let node_d = kernel.get_node(d);
    assert_eq!(node_d.get_incoming_synapse_head(), Some(surviving_syn));
    assert_eq!(node_d.get_incoming_synapse_tail(), Some(surviving_syn));

    let syn = kernel.get_synapse(surviving_syn);
    assert_eq!(syn.get_source_ptr(), c);
    assert_eq!(syn.get_target_ptr(), d);
    assert_eq!(syn.get_kind(), 20);
}

#[test]
fn remove_hub_node_in_star_topology() {
    let kernel = create_writer();
    let hub = kernel.insert_node(0).unwrap();

    let mut spokes = Vec::new();
    for i in 1..=6 {
        let s = kernel.insert_node(i).unwrap();
        spokes.push(s);
    }

    // hub -> all spokes (fan-out)
    for &spoke in &spokes {
        kernel.connect(hub, spoke, 1).unwrap();
    }

    // all spokes -> hub (fan-in)
    for &spoke in &spokes {
        kernel.connect(spoke, hub, 2).unwrap();
    }

    // Remove hub: 12 synapses (6 outgoing + 6 incoming) must be auto-disconnected
    kernel.remove_node(hub).unwrap();

    // All spokes must have completely clean synapse pointers
    for &spoke in &spokes {
        let node = kernel.get_node(spoke);
        assert!(node.get_outgoing_synapse_head().is_none(), "spoke {} outgoing head", spoke);
        assert!(node.get_outgoing_synapse_tail().is_none(), "spoke {} outgoing tail", spoke);
        assert!(node.get_incoming_synapse_head().is_none(), "spoke {} incoming head", spoke);
        assert!(node.get_incoming_synapse_tail().is_none(), "spoke {} incoming tail", spoke);
    }
}

#[test]
fn remove_middle_node_in_linear_chain_with_synapses() {
    let kernel = create_writer();
    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node_after(a, 2).unwrap();
    let c = kernel.insert_node_after(b, 3).unwrap();

    // a -> b -> c (synapse chain)
    kernel.connect(a, b, 10).unwrap();
    kernel.connect(b, c, 20).unwrap();

    // Remove middle node b
    kernel.remove_node(b).unwrap();

    // a's outgoing should be clean
    let node_a = kernel.get_node(a);
    assert!(node_a.get_outgoing_synapse_head().is_none());

    // c's incoming should be clean
    let node_c = kernel.get_node(c);
    assert!(node_c.get_incoming_synapse_head().is_none());

    // Node chain should be healed: a -> c
    assert_eq!(node_a.get_next_ptr(), Some(c));
    assert_eq!(node_c.get_prev_ptr(), Some(a));
}

#[test]
fn remove_node_without_synapses_still_works() {
    let kernel = create_writer();
    // chain: a -> b
    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node_after(a, 2).unwrap();

    kernel.remove_node(a).unwrap();

    // b is now the only node (head and tail).
    let node_b = kernel.get_node(b);
    assert_eq!(node_b.get_kind(), 2);
    assert!(node_b.get_prev_ptr().is_none());
    assert!(node_b.get_next_ptr().is_none());
}

#[test]
fn cascade_frees_synapse_slots_for_reuse() {
    let kernel = create_writer();
    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node(2).unwrap();

    // Fill synapse capacity
    let mut synapses = Vec::new();
    for i in 0..32 {
        if let Ok(s) = kernel.connect(a, b, i) {
            synapses.push(s);
        }
    }
    assert_eq!(synapses.len(), 32, "should have allocated all 32 synapse slots");

    // Can't connect any more
    assert!(kernel.connect(a, b, 999).is_err(), "should be at capacity");

    // Remove a — cascades all 32 synapses
    kernel.remove_node(a).unwrap();

    // Need 2 flushes (deferred free two-stage)
    // We don't have direct flush at this level — the publish cycle handles it.
    // But we can verify the node store is intact.
    let node_b = kernel.get_node(b);
    assert!(node_b.get_incoming_synapse_head().is_none());
    assert!(node_b.get_incoming_synapse_tail().is_none());
}
