mod common;

use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;
use synaptic_kernel::epoch_consumer::EpochConsumer;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;

type TestKernel = Kernel<1, 1, 1>;
type TestProcessor = EpochConsumer<1, 1, 1>;

fn config() -> KernelConfig<1, 1, 1> {
    common::kernel_config_1_1_full(
        4,
        4,
        4,
        NODE_META,
        NODE_ATTR,
        SYNAPSE_META,
        SYNAPSE_ATTR,
        32768,
    )
}

// ============ mem_metadata: write/read round-trip ============

#[test]
fn mem_metadata_write_read_round_trip() {
    let controller = TestKernel::new(config());

    controller.mem_write_meta(0, 42);
    controller.mem_write_meta(1, -100);
    controller.mem_write_meta(2, i32::MAX);
    controller.mem_write_meta(3, i32::MIN);

    assert_eq!(controller.mem_read_meta(0), 42);
    assert_eq!(controller.mem_read_meta(1), -100);
    assert_eq!(controller.mem_read_meta(2), i32::MAX);
    assert_eq!(controller.mem_read_meta(3), i32::MIN);
}

#[test]
fn mem_metadata_overwrite() {
    let controller = TestKernel::new(config());

    controller.mem_write_meta(0, 1);
    assert_eq!(controller.mem_read_meta(0), 1);

    controller.mem_write_meta(0, 999);
    assert_eq!(controller.mem_read_meta(0), 999);
}

#[test]
fn mem_metadata_independent_slots() {
    let controller = TestKernel::new(config());

    controller.mem_write_meta(0, 10);
    controller.mem_write_meta(1, 20);
    controller.mem_write_meta(2, 30);
    controller.mem_write_meta(3, 40);

    // Writing one slot doesn't affect others
    controller.mem_write_meta(2, 999);
    assert_eq!(controller.mem_read_meta(0), 10);
    assert_eq!(controller.mem_read_meta(1), 20);
    assert_eq!(controller.mem_read_meta(2), 999);
    assert_eq!(controller.mem_read_meta(3), 40);
}

#[test]
fn mem_metadata_default_zero() {
    let controller = TestKernel::new(config());

    for i in 0..4 {
        assert_eq!(controller.mem_read_meta(i), 0, "mem_metadata slot {} should default to 0", i);
    }
}

// ============ mem_metadata: visible to reader immediately (shared AtomicBuffer) ============

#[test]
fn mem_metadata_visible_to_reader_without_publish() {
    let mut controller = TestKernel::new(config());

    controller.mem_write_meta(0, 42);
    controller.mem_write_meta(1, 99);

    // Publish to get the graph, but mem_metadata is on the shared AtomicBuffer
    // so writes should be visible immediately (Relaxed atomics, same-thread)
    controller.publish();

    let mut processor = TestProcessor::new(controller.get_control_plane());
    let graph = processor.acquire_mirror();

    assert_eq!(graph.mem_read_meta(0), 42);
    assert_eq!(graph.mem_read_meta(1), 99);
}

#[test]
fn mem_metadata_update_between_reads() {
    let mut controller = TestKernel::new(config());
    controller.publish();

    let mut processor = TestProcessor::new(controller.get_control_plane());

    // First read
    let graph = processor.acquire_mirror();
    assert_eq!(graph.mem_read_meta(0), 0);

    // Writer updates
    controller.mem_write_meta(0, 777);

    // Reader sees update immediately (shared atomics)
    let graph = processor.acquire_mirror();
    assert_eq!(graph.mem_read_meta(0), 777);
}

// ============ Capacity ============

#[test]
fn mem_metadata_capacity_matches_config() {
    let controller = TestKernel::new(config());
    assert_eq!(controller.mem_metadata_capacity(), 4);
}

// ============ Metadata survives grow ============

#[test]
fn mem_metadata_survives_grow() {
    let mut controller = TestKernel::new(config());

    controller.mem_write_meta(0, 42);
    controller.mem_write_meta(1, 99);
    controller.publish();

    controller
        .grow(common::kernel_config_1_1_full(
            4,
            8,
            8,
            NODE_META,
            NODE_ATTR,
            SYNAPSE_META,
            SYNAPSE_ATTR,
            32768,
        ))
        .unwrap();

    assert_eq!(controller.mem_read_meta(0), 42);
    assert_eq!(controller.mem_read_meta(1), 99);
}

// ============ Mixed: metadata + structural mutations ============

#[test]
fn metadata_coexists_with_node_mutations() {
    let mut controller = TestKernel::new(config());

    controller.mem_write_meta(0, 10);

    let n = controller.insert_node(1).unwrap();
    controller.get_node(n).attr_write(0, 42);

    controller.publish();

    assert_eq!(controller.mem_read_meta(0), 10);
    // Node intact
    assert_eq!(controller.get_node(n).get_kind(), 1);
}
