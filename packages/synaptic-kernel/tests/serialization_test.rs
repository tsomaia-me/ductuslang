mod common;

use synaptic_kernel::epoch_consumer::EpochConsumer;
use synaptic_kernel::kernel::Kernel;
use synaptic_kernel::kernel_config::KernelConfig;
use synaptic_kernel::primitives::entry_store_def::EntryStoreId;
use synaptic_kernel::primitives::lut_def::LutId;
use synaptic_kernel::primitives::slot::SlotId;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;

type TestKernel = Kernel<1, 1, 1>;

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

fn flush_deferred(kernel: &mut TestKernel) {
    kernel.publish();
    let cp = kernel.get_control_plane();
    let mut consumer = EpochConsumer::<1, 1, 1>::new(cp);
    let _graph = consumer.acquire_mirror();
    kernel.publish();
}

// =========================================================
// PHASE 1: Empty Kernel Round-Trip
// =========================================================

#[test]
fn empty_kernel_serialize_and_load() {
    let mut kernel = TestKernel::new(config(16));
    let serialized = kernel.serialize();

    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.node_capacity(), 16);
    assert_eq!(loaded.synapse_capacity(), 16);
    assert_eq!(loaded.node_count(), 0);
    assert_eq!(loaded.synapse_count(), 0);
}

#[test]
fn empty_kernel_serialized_config_matches() {
    let mut kernel = TestKernel::new(create_config(32, 64));
    let serialized = kernel.serialize();

    assert_eq!(serialized.config.network_config.node_capacity, 32);
    assert_eq!(serialized.config.network_config.synapse_capacity, 64);
    assert_eq!(serialized.config.mem_metadata_size, 1);
}

// =========================================================
// PHASE 2: Node Topology Preservation
// =========================================================

#[test]
fn single_node_survives_round_trip() {
    let mut kernel = TestKernel::new(config(16));
    let slot = kernel.insert_node(42).unwrap();
    kernel.publish();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.node_count(), 1);
    let node = loaded.get_node(slot);
    assert_eq!(node.get_kind(), 42);
}

#[test]
fn node_store_order_preserved() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(10).unwrap();
    let n2 = kernel.insert_node_after(n1, 20).unwrap();
    let n3 = kernel.insert_node_after(n2, 30).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    let w1 = loaded.get_node(n1);
    let w2 = loaded.get_node(n2);
    let w3 = loaded.get_node(n3);

    assert_eq!(w1.get_kind(), 10);
    assert_eq!(w1.get_next_ptr(), Some(n2));
    assert!(w1.get_prev_ptr().is_none());

    assert_eq!(w2.get_kind(), 20);
    assert_eq!(w2.get_prev_ptr(), Some(n1));
    assert_eq!(w2.get_next_ptr(), Some(n3));

    assert_eq!(w3.get_kind(), 30);
    assert_eq!(w3.get_prev_ptr(), Some(n2));
    assert!(w3.get_next_ptr().is_none());
}

#[test]
fn chain_head_position_preserved() {
    let mut kernel = TestKernel::new(config(16));
    // chain: n1 -> n2 (n1 is the head with prev_ptr=0)
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    let head = loaded.get_node(n1);
    assert_eq!(head.get_kind(), 1);
    assert!(head.get_prev_ptr().is_none(), "n1 is still the chain head");
    assert_eq!(head.get_next_ptr(), Some(n2));

    let tail = loaded.get_node(n2);
    assert_eq!(tail.get_kind(), 2);
    assert_eq!(tail.get_prev_ptr(), Some(n1));
    assert!(tail.get_next_ptr().is_none());
}

// =========================================================
// PHASE 3: Synapse Topology Preservation
// =========================================================

#[test]
fn synapse_connectivity_preserved() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    let n3 = kernel.insert_node_after(n2, 3).unwrap();

    let s12 = kernel.connect(n1, n2, 10).unwrap();
    let s13 = kernel.connect(n1, n3, 20).unwrap();
    let s23 = kernel.connect(n2, n3, 30).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.get_synapse(s12).get_kind(), 10);
    assert_eq!(loaded.get_synapse(s12).get_source_ptr(), n1);
    assert_eq!(loaded.get_synapse(s12).get_target_ptr(), n2);

    assert_eq!(loaded.get_synapse(s13).get_kind(), 20);
    assert_eq!(loaded.get_synapse(s13).get_source_ptr(), n1);
    assert_eq!(loaded.get_synapse(s13).get_target_ptr(), n3);

    assert_eq!(loaded.get_synapse(s23).get_kind(), 30);
    assert_eq!(loaded.get_synapse(s23).get_source_ptr(), n2);
    assert_eq!(loaded.get_synapse(s23).get_target_ptr(), n3);
}

#[test]
fn outgoing_synapse_chain_preserved() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    let n3 = kernel.insert_node_after(n2, 3).unwrap();

    let s12 = kernel.connect(n1, n2, 10).unwrap();
    let s13 = kernel.connect(n1, n3, 20).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    let node1 = loaded.get_node(n1);
    assert_eq!(node1.get_outgoing_synapse_head(), Some(s12));
    assert_eq!(node1.get_outgoing_synapse_tail(), Some(s13));

    let syn12 = loaded.get_synapse(s12);
    assert_eq!(syn12.get_outgoing_next_ptr(), Some(s13));
    assert!(syn12.get_outgoing_prev_ptr().is_none());

    let syn13 = loaded.get_synapse(s13);
    assert!(syn13.get_outgoing_next_ptr().is_none());
    assert_eq!(syn13.get_outgoing_prev_ptr(), Some(s12));
}

#[test]
fn incoming_synapse_chain_preserved() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    let n3 = kernel.insert_node_after(n2, 3).unwrap();

    let s13 = kernel.connect(n1, n3, 10).unwrap();
    let s23 = kernel.connect(n2, n3, 20).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    let node3 = loaded.get_node(n3);
    assert_eq!(node3.get_incoming_synapse_head(), Some(s13));
    assert_eq!(node3.get_incoming_synapse_tail(), Some(s23));

    let syn13 = loaded.get_synapse(s13);
    assert_eq!(syn13.get_incoming_next_ptr(), Some(s23));

    let syn23 = loaded.get_synapse(s23);
    assert_eq!(syn23.get_incoming_prev_ptr(), Some(s13));
}

// =========================================================
// PHASE 4: Attribute Preservation
// =========================================================

#[test]
fn node_attributes_preserved() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();

    for offset in 0..NODE_ATTR {
        kernel.get_node(n1).attr_write(offset, (offset as i32) * 100 + 7);
    }

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    for offset in 0..NODE_ATTR {
        assert_eq!(
            loaded.get_node(n1).attr_read(offset),
            (offset as i32) * 100 + 7,
            "node attribute mismatch at offset {}",
            offset,
        );
    }
}

#[test]
fn synapse_attributes_preserved() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    let s1 = kernel.connect(n1, n2, 5).unwrap();

    for offset in 0..SYNAPSE_ATTR {
        kernel.get_synapse(s1).attr_write(offset, -(offset as i32) * 50);
    }

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    for offset in 0..SYNAPSE_ATTR {
        assert_eq!(
            loaded.get_synapse(s1).attr_read(offset),
            -(offset as i32) * 50,
            "synapse attribute mismatch at offset {}",
            offset,
        );
    }
}

#[test]
fn negative_and_extreme_attribute_values_preserved() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();

    kernel.get_node(n1).attr_write(0, i32::MIN);
    kernel.get_node(n1).attr_write(1, i32::MAX);
    kernel.get_node(n1).attr_write(2, -1);
    kernel.get_node(n1).attr_write(3, 0);

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.get_node(n1).attr_read(0), i32::MIN);
    assert_eq!(loaded.get_node(n1).attr_read(1), i32::MAX);
    assert_eq!(loaded.get_node(n1).attr_read(2), -1);
    assert_eq!(loaded.get_node(n1).attr_read(3), 0);
}

// =========================================================
// PHASE 5: Metadata Preservation
// =========================================================

#[test]
fn mem_metadata_preserved() {
    let mut kernel = TestKernel::new(config(16));
    kernel.mem_write_meta(0, 12345);

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.mem_read_meta(0), 12345);
}

// =========================================================
// PHASE 6: Node Meta (Topology Metadata) Preservation
// =========================================================

#[test]
fn node_meta_preserved() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();

    for i in 0..NODE_META {
        kernel.get_node(n1).attr_write(i, (i as i32) * 11);
    }

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    for i in 0..NODE_META {
        assert_eq!(
            loaded.get_node(n1).attr_read(i),
            (i as i32) * 11,
            "node meta mismatch at offset {}",
            i,
        );
    }
}

#[test]
fn synapse_meta_preserved() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    let s1 = kernel.connect(n1, n2, 5).unwrap();

    for i in 0..SYNAPSE_META {
        kernel.get_synapse(s1).attr_write(i, (i as i32) * 13);
    }

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    for i in 0..SYNAPSE_META {
        assert_eq!(
            loaded.get_synapse(s1).attr_read(i),
            (i as i32) * 13,
            "synapse meta mismatch at offset {}",
            i,
        );
    }
}

// =========================================================
// PHASE 7: Capacity & Allocator State
// =========================================================

#[test]
fn node_count_preserved() {
    let mut kernel = TestKernel::new(config(16));
    kernel.insert_node(1).unwrap();
    kernel.insert_node(2).unwrap();
    kernel.insert_node(3).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.node_count(), 3);
    assert_eq!(loaded.node_capacity(), 16);
}

#[test]
fn synapse_count_preserved() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    kernel.connect(n1, n2, 1).unwrap();
    kernel.connect(n2, n1, 2).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.synapse_count(), 2);
}

#[test]
fn free_slots_allocatable_after_load() {
    let mut kernel = TestKernel::new(config(4));
    kernel.insert_node(1).unwrap();
    kernel.insert_node(2).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.node_count(), 2);
    loaded.insert_node(3).unwrap();
    loaded.insert_node(4).unwrap();
    assert_eq!(loaded.node_count(), 4);

    assert!(loaded.insert_node(5).is_err());
}

#[test]
fn capacity_at_limit_round_trips() {
    let mut kernel = TestKernel::new(config(4));
    kernel.insert_node(1).unwrap();
    kernel.insert_node(2).unwrap();
    kernel.insert_node(3).unwrap();
    kernel.insert_node(4).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.node_count(), 4);
    assert!(loaded.insert_node(5).is_err());
}

// =========================================================
// PHASE 8: Deferred Frees & Fragmentation
// =========================================================

#[test]
fn deferred_frees_flushed_before_serialize() {
    let mut kernel = TestKernel::new(config(4));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node(2).unwrap();
    kernel.insert_node(3).unwrap();
    kernel.insert_node(4).unwrap();

    kernel.remove_node(n1).unwrap();
    kernel.remove_node(n2).unwrap();
    flush_deferred(&mut kernel);

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.node_count(), 2);
    loaded.insert_node(5).unwrap();
    loaded.insert_node(6).unwrap();
    assert_eq!(loaded.node_count(), 4);
}

#[test]
fn fragmented_free_list_survives_round_trip() {
    let mut kernel = TestKernel::new(config(8));

    let mut slots = Vec::new();
    for i in 0..8 {
        slots.push(kernel.insert_node(i).unwrap());
    }

    kernel.remove_node(slots[1]).unwrap();
    kernel.remove_node(slots[3]).unwrap();
    kernel.remove_node(slots[5]).unwrap();
    flush_deferred(&mut kernel);

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.node_count(), 5);
    assert_eq!(loaded.get_node(slots[0]).get_kind(), 0);
    assert_eq!(loaded.get_node(slots[2]).get_kind(), 2);
    assert_eq!(loaded.get_node(slots[4]).get_kind(), 4);
    assert_eq!(loaded.get_node(slots[6]).get_kind(), 6);
    assert_eq!(loaded.get_node(slots[7]).get_kind(), 7);

    loaded.insert_node(100).unwrap();
    loaded.insert_node(101).unwrap();
    loaded.insert_node(102).unwrap();
    assert_eq!(loaded.node_count(), 8);
}

#[test]
fn disconnect_then_serialize_preserves_remaining_synapses() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    let n3 = kernel.insert_node_after(n2, 3).unwrap();

    let s12 = kernel.connect(n1, n2, 10).unwrap();
    let _s13 = kernel.connect(n1, n3, 20).unwrap();
    let s23 = kernel.connect(n2, n3, 30).unwrap();

    kernel.disconnect_synapse(s12).unwrap();
    flush_deferred(&mut kernel);

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.synapse_count(), 2);
    assert_eq!(loaded.get_synapse(s23).get_kind(), 30);

    let fresh = loaded.connect(n1, n2, 99).unwrap();
    assert_eq!(loaded.get_synapse(fresh).get_kind(), 99);
}

// =========================================================
// PHASE 9: Post-Grow Serialization
// =========================================================

#[test]
fn serialize_after_grow_uses_new_capacity() {
    let mut kernel = TestKernel::new(config(4));
    kernel.insert_node(1).unwrap();
    kernel.insert_node(2).unwrap();

    kernel.grow(config(16)).unwrap();

    let serialized = kernel.serialize();
    assert_eq!(serialized.config.network_config.node_capacity, 16);
    assert_eq!(serialized.config.network_config.synapse_capacity, 16);

    let loaded = TestKernel::load_serialized(serialized);
    assert_eq!(loaded.node_capacity(), 16);
    assert_eq!(loaded.node_count(), 2);

    for i in 3..=16 {
        loaded.insert_node(i as i32).unwrap();
    }
    assert_eq!(loaded.node_count(), 16);
}

#[test]
fn topology_preserved_after_grow_and_serialize() {
    let mut kernel = TestKernel::new(config(4));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    let s1 = kernel.connect(n1, n2, 50).unwrap();

    kernel.get_node(n1).attr_write(0, 1000);
    kernel.get_synapse(s1).attr_write(0, 2000);
    // Verify BOTH planes survive grow+serialize: attr (MEM plane, above) and
    // meta (TB plane, below). Collapsing both to attr would leave the TB
    // serialization path untested.
    kernel.get_node(n1).set_meta(0, 3000);

    kernel.grow(config(16)).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    let head = loaded.get_node(n1);
    assert_eq!(head.get_kind(), 1);
    assert_eq!(head.get_next_ptr(), Some(n2));

    assert_eq!(loaded.get_node(n2).get_kind(), 2);
    assert_eq!(loaded.get_synapse(s1).get_kind(), 50);
    assert_eq!(loaded.get_node(n1).attr_read(0), 1000);
    assert_eq!(loaded.get_synapse(s1).attr_read(0), 2000);
    assert_eq!(loaded.get_node(n1).get_meta(0), 3000);
}

#[test]
fn multiple_grows_then_serialize() {
    let mut kernel = TestKernel::new(config(4));
    let s1 = kernel.insert_node(1).unwrap();

    kernel.grow(config(8)).unwrap();
    kernel.publish();

    let s2 = kernel.insert_node(2).unwrap();
    kernel.grow(config(16)).unwrap();
    kernel.publish();

    let s3 = kernel.insert_node(3).unwrap();
    kernel.grow(config(32)).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.node_capacity(), 32);
    assert_eq!(loaded.node_count(), 3);

    assert_eq!(loaded.get_node(s1).get_kind(), 1);
    assert_eq!(loaded.get_node(s2).get_kind(), 2);
    assert_eq!(loaded.get_node(s3).get_kind(), 3);
}

// =========================================================
// PHASE 10: Asymmetric Config
// =========================================================

#[test]
fn asymmetric_config_round_trips() {
    let mut kernel = TestKernel::new(create_config(32, 8));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    kernel.connect(n1, n2, 5).unwrap();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    assert_eq!(loaded.node_capacity(), 32);
    assert_eq!(loaded.synapse_capacity(), 8);
    assert_eq!(loaded.node_count(), 2);
    assert_eq!(loaded.synapse_count(), 1);
}

// =========================================================
// PHASE 11: Mutability After Load
// =========================================================

#[test]
fn loaded_kernel_supports_full_mutation_cycle() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    kernel.connect(n1, n2, 10).unwrap();

    let serialized = kernel.serialize();
    let mut loaded = TestKernel::load_serialized(serialized);

    let n3 = loaded.insert_node_after(n2, 3).unwrap();
    let s23 = loaded.connect(n2, n3, 20).unwrap();
    loaded.get_node(n3).attr_write(0, 999);
    loaded.get_synapse(s23).attr_write(0, 888);
    loaded.publish();

    assert_eq!(loaded.node_count(), 3);
    assert_eq!(loaded.synapse_count(), 2);
    assert_eq!(loaded.get_node(n3).get_kind(), 3);
    assert_eq!(loaded.get_synapse(s23).get_kind(), 20);
    assert_eq!(loaded.get_node(n3).attr_read(0), 999);
    assert_eq!(loaded.get_synapse(s23).attr_read(0), 888);
}

#[test]
fn loaded_kernel_supports_grow() {
    let mut kernel = TestKernel::new(config(4));
    kernel.insert_node(1).unwrap();
    kernel.insert_node(2).unwrap();

    let serialized = kernel.serialize();
    let mut loaded = TestKernel::load_serialized(serialized);

    loaded.grow(config(16)).unwrap();
    assert_eq!(loaded.node_capacity(), 16);
    assert_eq!(loaded.node_count(), 2);

    for i in 3..=16 {
        loaded.insert_node(i as i32).unwrap();
    }
    assert_eq!(loaded.node_count(), 16);
}

#[test]
fn loaded_kernel_supports_remove_and_realloc() {
    let mut kernel = TestKernel::new(config(4));
    let n1 = kernel.insert_node(1).unwrap();
    kernel.insert_node(2).unwrap();
    kernel.insert_node(3).unwrap();
    kernel.insert_node(4).unwrap();

    let serialized = kernel.serialize();
    let mut loaded = TestKernel::load_serialized(serialized);

    assert!(loaded.insert_node(5).is_err());

    loaded.remove_node(n1).unwrap();
    flush_deferred(&mut loaded);

    let n5 = loaded.insert_node(5).unwrap();
    assert_eq!(loaded.get_node(n5).get_kind(), 5);
}

// =========================================================
// PHASE 12: Double Serialization (Idempotency)
// =========================================================

#[test]
fn double_serialize_preserves_semantic_content() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    kernel.connect(n1, n2, 10).unwrap();
    kernel.get_node(n1).attr_write(0, 999);

    let s1 = kernel.serialize();
    let loaded = TestKernel::load_serialized(s1);

    assert_eq!(loaded.node_count(), 2);
    assert_eq!(loaded.get_node(n1).get_kind(), 1);
    assert_eq!(loaded.get_node(n2).get_kind(), 2);
    assert_eq!(loaded.get_node(n1).attr_read(0), 999);
    assert_eq!(loaded.synapse_count(), 1);
}

#[test]
fn serialize_load_serialize_preserves_semantic_content() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    let s1 = kernel.connect(n1, n2, 10).unwrap();
    kernel.get_node(n1).attr_write(0, 999);
    kernel.get_node(n2).attr_write(5, -42);
    kernel.get_node(n1).attr_write(3, 777);
    kernel.mem_write_meta(0, 12345);

    let first = kernel.serialize();
    let mut loaded = TestKernel::load_serialized(first);
    let second = loaded.serialize();
    let reloaded = TestKernel::load_serialized(second);

    assert_eq!(reloaded.node_count(), 2);
    assert_eq!(reloaded.synapse_count(), 1);
    assert_eq!(reloaded.get_node(n1).get_kind(), 1);
    assert_eq!(reloaded.get_node(n2).get_kind(), 2);
    assert_eq!(reloaded.get_node(n1).get_next_ptr(), Some(n2));
    assert_eq!(reloaded.get_synapse(s1).get_kind(), 10);
    assert_eq!(reloaded.get_node(n1).attr_read(0), 999);
    assert_eq!(reloaded.get_node(n2).attr_read(5), -42);
    assert_eq!(reloaded.get_node(n1).attr_read(3), 777);
    assert_eq!(reloaded.mem_read_meta(0), 12345);
}

// =========================================================
// PHASE 13: Consumer Thread After Load
// =========================================================

#[test]
fn consumer_thread_sees_loaded_state_after_publish_swap() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    let n2 = kernel.insert_node_after(n1, 2).unwrap();
    kernel.connect(n1, n2, 10).unwrap();
    kernel.get_node(n1).attr_write(0, 42);

    let serialized = kernel.serialize();
    let mut loaded = TestKernel::load_serialized(serialized);

    let mut consumer = EpochConsumer::<1, 1, 1>::new(loaded.get_control_plane());
    loaded.publish();
    let graph = consumer.acquire_mirror();

    let head = graph.get_node(n1);
    assert_eq!(head.get_kind(), 1);
    assert_eq!(graph.get_node(n1).attr_read(0), 42);

    let next = graph.get_node(head.get_next_ptr().unwrap());
    assert_eq!(next.get_kind(), 2);

    let syn = graph.get_synapse(SlotId::new(1).unwrap());
    assert_eq!(syn.get_kind(), 10);
}

#[test]
fn mutations_after_load_visible_to_consumer_thread() {
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();

    let serialized = kernel.serialize();
    let mut loaded = TestKernel::load_serialized(serialized);
    let mut consumer = EpochConsumer::<1, 1, 1>::new(loaded.get_control_plane());

    let n2 = loaded.insert_node(2).unwrap();
    loaded.get_node(n2).attr_write(0, 555);
    loaded.publish();

    let graph = consumer.acquire_mirror();

    assert_eq!(graph.get_node(n1).get_kind(), 1);
    assert_eq!(graph.get_node(n2).get_kind(), 2);
    assert_eq!(graph.get_node(n2).attr_read(0), 555);
}

#[test]
fn entry_store_data_survives_serialize_load() {
    let mut kernel = TestKernel::new(config(16));
    let slot = kernel.get_entry_store(EntryStoreId(0)).insert().unwrap();
    kernel
        .get_entry_store(EntryStoreId(0))
        .get(slot)
        .attr_write(0, 12345);
    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);
    assert_eq!(
        loaded
            .get_entry_store(EntryStoreId(0))
            .get(slot)
            .attr_read(0),
        12345
    );
}

#[test]
fn lut_data_preserved_through_serialization() {
    let mut kernel = TestKernel::new(config(16));
    kernel.get_lut(LutId(0)).write(0, 31_415);
    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);
    assert_eq!(loaded.get_lut(LutId(0)).read(0), 31_415);
}

#[test]
fn store_defs_preserved_in_serialized_config() {
    let mut kernel = TestKernel::new(config(16));
    let serialized = kernel.serialize();
    assert_eq!(serialized.config.store_defs[0].id, EntryStoreId(0));
    assert_eq!(serialized.config.store_defs[0].config.capacity, 4);
}

// =========================================================
// PHASE 14: Bind-mid-publish round-trip
// =========================================================
//
// Producer publishes (writer_index advanced, NEW_DATA set on the default
// TB) but no consumer ever swaps. Then serialize -> load_serialized.
// The rebound writer's `TripleBufferWriter::create(bind=true)` path runs
// its `Acquire` load on `state` and sync-from-published step. The freshly-
// loaded kernel should see the most recent published frame, not the
// initial empty buffer.

#[test]
fn bind_after_unconsumed_publish_recovers_published_state() {
    let mut kernel = TestKernel::new(config(16));

    let n1 = kernel.insert_node(7).unwrap();
    let n2 = kernel.insert_node_after(n1, 8).unwrap();
    let s = kernel.connect(n1, n2, 99).unwrap();
    kernel.get_node(n1).set_meta(0, 1234);

    // Publish but do NOT touch the consumer side. The TB has NEW_DATA
    // set; no swap has happened.
    kernel.publish();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    // The rebound kernel must be able to expose the topology that was
    // published just before serialize, which only works if the bind path
    // pulled the latest published buffer into the writer's working slot.
    let head = loaded.get_node(n1);
    assert_eq!(head.get_kind(), 7);
    assert_eq!(head.get_next_ptr(), Some(n2));
    assert_eq!(head.get_meta(0), 1234);

    let tail = loaded.get_node(n2);
    assert_eq!(tail.get_kind(), 8);
    assert_eq!(tail.get_prev_ptr(), Some(n1));

    let syn = loaded.get_synapse(s);
    assert_eq!(syn.get_kind(), 99);
    assert_eq!(syn.get_source_ptr(), n1);
    assert_eq!(syn.get_target_ptr(), n2);
}

#[test]
fn bind_after_unconsumed_publish_supports_further_mutation() {
    // After bind-mid-publish, the loaded kernel must accept new writes
    // and surface them to a fresh consumer. If the bind path produced a
    // half-initialized writer that started from buffer 0, the next
    // publish would land in the wrong slot and the consumer would see
    // stale data.
    let mut kernel = TestKernel::new(config(16));
    let n1 = kernel.insert_node(1).unwrap();
    kernel.publish();

    let serialized = kernel.serialize();
    let mut loaded = TestKernel::load_serialized(serialized);
    let mut consumer = EpochConsumer::<1, 1, 1>::new(loaded.get_control_plane());

    // n1 is preserved. Add n2 after the bind, publish, and read both via
    // the consumer.
    let n2 = loaded.insert_node_after(n1, 2).unwrap();
    loaded.get_node(n2).attr_write(0, 999);
    loaded.publish();

    let mirror = consumer.acquire_mirror();
    assert_eq!(mirror.get_node(n1).get_kind(), 1);
    assert_eq!(mirror.get_node(n2).get_kind(), 2);
    assert_eq!(mirror.get_node(n2).attr_read(0), 999);
    assert_eq!(mirror.get_node(n1).get_next_ptr(), Some(n2));
}

#[test]
fn bind_after_two_unconsumed_publishes_recovers_latest() {
    // Two consecutive publishes without a swap in between: the second
    // publish overwrites the first as the "latest published". The bind
    // path must sync from the most recent published index, not the
    // first one.
    let mut kernel = TestKernel::new(config(16));
    let n = kernel.insert_node(1).unwrap();
    kernel.get_node(n).set_meta(0, 100);
    kernel.publish();

    kernel.get_node(n).set_meta(0, 200);
    kernel.publish();

    let serialized = kernel.serialize();
    let loaded = TestKernel::load_serialized(serialized);

    // Latest meta value (200) must be visible on the loaded kernel.
    assert_eq!(loaded.get_node(n).get_meta(0), 200);
}
