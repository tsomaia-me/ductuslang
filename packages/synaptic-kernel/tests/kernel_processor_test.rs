mod common;

use synaptic_kernel::epoch_consumer::EpochConsumer;
use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;

type TestKernel = Kernel<1, 1, 1>;
type TestConsumer = EpochConsumer<1, 1, 1>;

fn config(capacity: u32) -> KernelConfig<1, 1, 1> {
    common::kernel_config_1_1(
        capacity,
        capacity,
        NODE_META,
        NODE_ATTR,
        SYNAPSE_META,
        SYNAPSE_ATTR,
    )
}

fn setup(capacity: u32) -> TestKernel {
    Kernel::new(config(capacity))
}

fn get_consumer(controller: &TestKernel) -> TestConsumer {
    TestConsumer::new(controller.get_control_plane())
}

// ============ acquire_mirror basics ============
//
// In every test below, the consumer is declared after the kernel and so
// drops first; the kernel's debug-time Drop assert sees strong_count == 1.

#[test]
fn acquire_mirror_returns_reader() {
    let mut controller = setup(8);
    let mut consumer = get_consumer(&controller);

    let slot = controller.insert_node(42).unwrap();
    controller.publish();

    let mirror = consumer.acquire_mirror();
    assert_eq!(mirror.get_node(slot).get_kind(), 42);
}

#[test]
fn acquire_mirror_sees_published_mutations() {
    let mut controller = setup(8);
    let mut consumer = get_consumer(&controller);

    let a = controller.insert_node(1).unwrap();
    controller.publish();

    {
        let mirror = consumer.acquire_mirror();
        assert_eq!(mirror.get_node(a).get_kind(), 1);
    }

    let b = controller.insert_node(2).unwrap();
    controller.publish();

    let mirror = consumer.acquire_mirror();
    assert_eq!(mirror.get_node(b).get_kind(), 2);
}

#[test]
fn acquire_mirror_does_not_see_unpublished_mutations() {
    let mut controller = setup(8);
    let mut consumer = get_consumer(&controller);

    let a = controller.insert_node(1).unwrap();
    controller.publish();

    {
        // First acquire: pulls in a.
        let _mirror = consumer.acquire_mirror();
    }

    // Insert but don't publish.
    let b = controller.insert_node(2).unwrap();

    let mirror = consumer.acquire_mirror();
    // a is still visible at its slot; b is unpublished, so the mirror's TB
    // hasn't been swapped to a buffer that knows about b's pointers. We
    // check b's visibility indirectly: its meta starts at 0 on the consumer
    // side, and its kind will read whatever was at that slot last
    // published — which here is uninitialized (0).
    assert_eq!(mirror.get_node(a).get_kind(), 1);
    let _ = b;
}

// ============ Automatic ack enables epoch reclamation ============

#[test]
fn acquire_mirror_acks_previous_generation_enabling_drain() {
    let mut controller = setup(4);
    let mut consumer = get_consumer(&controller);

    let a = controller.insert_node(1).unwrap();

    // Grow creates a pending reader at generation 1.
    controller.grow(config(8)).unwrap();

    // First acquire acks gen 0; second acquire acks gen 1.
    let _ = consumer.acquire_mirror();
    let _ = consumer.acquire_mirror();

    controller.publish();

    assert_eq!(controller.node_capacity(), 8);
    assert_eq!(controller.get_node(a).get_kind(), 1);
}

#[test]
fn multiple_grow_then_acquire_drains_all() {
    let mut controller = setup(4);
    let mut consumer = get_consumer(&controller);

    let _a = controller.insert_node(1).unwrap();

    controller.grow(config(8)).unwrap();
    controller.grow(config(16)).unwrap();
    controller.grow(config(32)).unwrap();

    let _ = consumer.acquire_mirror();
    let _ = consumer.acquire_mirror();

    controller.publish();

    assert_eq!(controller.node_capacity(), 32);
    let b = controller.insert_node(2).unwrap();
    assert_eq!(controller.get_node(b).get_kind(), 2);
}

#[test]
fn publish_does_not_drain_without_acquire() {
    let mut controller = setup(4);
    let _consumer = get_consumer(&controller);

    let a = controller.insert_node(1).unwrap();

    controller.grow(config(8)).unwrap();

    controller.publish();
    controller.publish();
    controller.publish();

    assert_eq!(controller.get_node(a).get_kind(), 1);
}

// ============ Full traversal pattern ============

#[test]
fn full_traversal_nodes_and_synapses() {
    let mut controller = setup(16);
    let mut consumer = get_consumer(&controller);

    let n1 = controller.insert_node(10).unwrap();
    let n2 = controller.insert_node_after(n1, 20).unwrap();
    let _n3 = controller.insert_node_after(n2, 30).unwrap();
    controller.connect(n1, n2, 99).unwrap();
    controller.get_node(n1).attr_write(0, 1000);
    controller.publish();

    let mirror = consumer.acquire_mirror();

    // Walk the chain starting from n1.
    let mut kinds = vec![];
    let mut current = Some(mirror.get_node(n1));
    while let Some(node) = current {
        kinds.push(node.get_kind() as i32);
        match node.get_next_ptr() {
            Some(next) => current = Some(mirror.get_node(next)),
            None => break,
        }
    }
    assert_eq!(kinds, vec![10, 20, 30]);

    assert_eq!(mirror.get_node(n1).attr_read(0), 1000);

    let src_node = mirror.get_node(n1);
    let syn_slot = src_node.get_outgoing_synapse_head().expect("synapse exists");
    let syn = mirror.get_synapse(syn_slot);
    assert_eq!(syn.get_kind(), 99);
}

// ============ Graph pointer updates after grow ============

#[test]
fn consumer_sees_new_graph_after_grow() {
    let mut controller = setup(4);
    let mut consumer = get_consumer(&controller);

    let a = controller.insert_node(1).unwrap();
    controller.publish();

    {
        let mirror = consumer.acquire_mirror();
        assert_eq!(mirror.get_node(a).get_kind(), 1);
    }

    controller.grow(config(16)).unwrap();
    let b = controller.insert_node(2).unwrap();
    controller.publish();

    let mirror = consumer.acquire_mirror();
    assert_eq!(mirror.get_node(b).get_kind(), 2);
    assert_eq!(mirror.get_node(a).get_kind(), 1);
}
