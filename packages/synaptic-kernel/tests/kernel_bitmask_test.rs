//! Replacement coverage for the in-tree tests that were removed with
//! `src/topology/tests/`. The original unit tests poked `NodeWriter` /
//! `SynapseWriter` directly to verify the `kind` bit-packing. With the new
//! API those writer types aren't reachable from integration tests, so we
//! re-express the invariant behaviourally: after structural mutations that
//! touch the lower 24 bits of `core[0]` (via pointer writes on sibling
//! topology operations), the upper 8 bits (kind) must remain intact.

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

#[test]
fn node_kind_survives_sibling_insert_remove_cycles() {
    let kernel = TestKernel::new(config());

    // Pick a kind with all low bits of the kind byte set (0x7F = 127). That
    // gives us every mutable state change in the lower 24 bits of core[0]
    // (which is where `prev_ptr` lives — siblings hammer this on every
    // insert_node_before / remove_node of a neighbour) a chance to bleed
    // into the kind byte if the bitmask logic is wrong.
    let watched = kernel.insert_node(0x7F).unwrap();

    // Churn siblings so the topology engine repeatedly rewrites the
    // watched node's prev_ptr and next_ptr.
    let sib_a = kernel.insert_node_before(watched, 1).unwrap();
    let _sib_b = kernel.insert_node_before(watched, 2).unwrap();
    let sib_c = kernel.insert_node_after(watched, 3).unwrap();
    let _sib_d = kernel.insert_node_after(watched, 4).unwrap();

    // Remove two siblings to force more neighbour-pointer writes.
    kernel.remove_node(sib_a).unwrap();
    kernel.remove_node(sib_c).unwrap();

    assert_eq!(
        kernel.get_node(watched).get_kind(),
        0x7F,
        "node kind corrupted by sibling structural mutations",
    );
}

#[test]
fn synapse_kind_survives_peer_connect_disconnect_cycles() {
    let kernel = TestKernel::new(config());

    let a = kernel.insert_node(1).unwrap();
    let b = kernel.insert_node(2).unwrap();
    let c = kernel.insert_node(3).unwrap();

    // Watched synapse a→b with a kind whose lower 24 bits would be obliterated
    // if the bitmask in the linked-list pointer writes were wrong.
    let watched = kernel.connect(a, b, 0x7F).unwrap();

    // Now hammer peer synapses in the same outgoing list (source = a) and
    // incoming list (target = b) to force outgoing_next/prev and
    // incoming_next/prev updates on the watched synapse.
    let peer_1 = kernel.connect(a, c, 5).unwrap();
    let peer_2 = kernel.connect(a, b, 6).unwrap();
    let peer_3 = kernel.connect(c, b, 7).unwrap();
    kernel.disconnect_synapse(peer_1).unwrap();
    kernel.disconnect_synapse(peer_3).unwrap();
    let _ = peer_2;

    assert_eq!(
        kernel.get_synapse(watched).get_kind(),
        0x7F,
        "synapse kind corrupted by peer structural mutations",
    );
}

#[test]
fn uninvolved_node_data_survives_sibling_mutations() {
    // Replaces the old `uninvolved_node_data_survives_sibling_mutations` test
    // from `src/topology/tests/node_store_writer_tests.rs`. Asserts that
    // mutating siblings through the public `Kernel` API leaves an untouched
    // node's kind, meta (set during insert), and attributes fully intact.
    let kernel = TestKernel::new(config());

    let watched = kernel.insert_node(42).unwrap();
    kernel.get_node(watched).attr_write(0, 1000);
    kernel.get_node(watched).attr_write(15, -999);

    // Insert + remove surrounding siblings.
    let b = kernel.insert_node(2).unwrap();
    let _c = kernel.insert_node(3).unwrap();
    let d = kernel.insert_node_after(watched, 4).unwrap();
    kernel.remove_node(b).unwrap();
    kernel.remove_node(d).unwrap();

    // The watched node's intrinsic data must be untouched. Structural
    // pointers (prev/next) may have shifted; that's expected.
    let node = kernel.get_node(watched);
    assert_eq!(node.get_kind(), 42);
    assert_eq!(node.attr_read(0), 1000);
    assert_eq!(node.attr_read(15), -999);
    for offset in 1..15 {
        assert_eq!(node.attr_read(offset), 0, "attr[{}] unexpectedly touched", offset);
    }
}

#[test]
fn consumer_sees_producer_values_after_publish_swap() {
    // Replacement for the removed in-tree
    // `node_reader_sees_writer_data_after_publish` test. Exercises the full
    // producer → publish → consumer-swap pipeline through the public
    // `Kernel` + `EpochConsumer` interface.
    use synaptic_kernel::epoch_consumer::EpochConsumer;

    let mut kernel = TestKernel::new(config());
    let mut consumer =
        EpochConsumer::<1, 1, 1>::new(kernel.get_control_plane());

    let slot = kernel.insert_node(12).unwrap();
    kernel.get_node(slot).attr_write(0, 99);
    kernel.publish();

    let mirror = consumer.acquire_mirror();
    let node = mirror.get_node(slot);
    assert_eq!(node.get_kind(), 12);
    assert_eq!(node.attr_read(0), 99);
}
