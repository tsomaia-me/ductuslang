use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;
use synaptic_kernel::topology::network::network_config::NetworkConfig;
use synaptic_kernel::topology::network::network_reader::NetworkReader;
use synaptic_kernel::topology::network::network_writer::NetworkWriter;

const NODE_META: usize = 8;
const NODE_ATTR: usize = 16;
const SYNAPSE_META: usize = 8;
const SYNAPSE_ATTR: usize = 16;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

const MEM_SIZE: usize = 65536;
const TB_START: usize = 0;
const TB_BUF_CAP: u32 = 16384;
const NODE_CAPACITY: u32 = 16;
const SYNAPSE_CAPACITY: u32 = 32;
const NODE_START_OFFSET: usize = 0;
const NODE_FL_START: usize = 50000;

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
type TestNetworkReader = NetworkReader;

struct TestHarness {
    _mem: AtomicBuffer,
    writer: synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter,
    reader: synaptic_kernel::primitives::triple_buffer_reader::TripleBufferReader,
    /// `node_chain` and `synapse_chain` are clones of the same underlying
    /// `NetworkWriter`. See the note on `network_test.rs` for rationale.
    node_chain: TestNetwork,
    synapse_chain: TestNetwork,
    synapse_chain_r: TestNetworkReader,
}

fn setup() -> TestHarness {
    let mem = create_mem(MEM_SIZE);
    let writer = TripleBufferWriter::new(Arc::clone(&mem), TB_START, TB_BUF_CAP);
    let reader = writer.to_reader();
    let network = NetworkWriter::new(
        Arc::clone(&mem),
        writer.clone(),
        net_config(),
        NODE_FL_START,
        NODE_START_OFFSET,
    );
    let synapse_chain_r = network.to_reader();
    let node_chain = network.clone();
    let synapse_chain = network;
    TestHarness {
        _mem: mem,
        writer,
        reader,
        node_chain,
        synapse_chain,
        synapse_chain_r,
    }
}

#[test]
fn reader_sees_all_synapse_fields_after_publish() {
    let h = setup();

    let (src, tgt, syn) = {
        let node_chain = h.node_chain;
        let synapse_chain = h.synapse_chain;

        let src = node_chain.insert_node(1).unwrap();
        let tgt = node_chain.insert_node(2).unwrap();
        let syn = synapse_chain.connect(src, tgt, 42).unwrap();
        (src, tgt, syn)
    };
    h.writer.publish();
    h.reader.swap();

    let synapse_chain_r = h.synapse_chain_r;
    let s = synapse_chain_r.get_synapse(syn);
    assert_eq!(s.get_kind(), 42);
    assert_eq!(s.get_source_ptr(), src);
    assert_eq!(s.get_target_ptr(), tgt);
    assert!(s.get_outgoing_next_ptr().is_none());
    assert!(s.get_outgoing_prev_ptr().is_none());
    assert!(s.get_incoming_next_ptr().is_none());
    assert!(s.get_incoming_prev_ptr().is_none());
}

#[test]
fn reader_sees_chain_pointers_with_multiple_synapses() {
    let h = setup();

    let (src, s1, s2, s3) = {
        let node_chain = h.node_chain;
        let synapse_chain = h.synapse_chain;

        let src = node_chain.insert_node(1).unwrap();
        let tgt1 = node_chain.insert_node(2).unwrap();
        let tgt2 = node_chain.insert_node(3).unwrap();
        let tgt3 = node_chain.insert_node(4).unwrap();

        let s1 = synapse_chain.connect(src, tgt1, 10).unwrap();
        let s2 = synapse_chain.connect(src, tgt2, 20).unwrap();
        let s3 = synapse_chain.connect(src, tgt3, 30).unwrap();
        (src, s1, s2, s3)
    };
    h.writer.publish();
    h.reader.swap();

    let reader = h.synapse_chain_r;

    // verify outgoing chain traversal via reader: s1 -> s2 -> s3
    let r1 = reader.get_synapse(s1);
    assert_eq!(r1.get_kind(), 10);
    assert_eq!(r1.get_source_ptr(), src);
    assert!(r1.get_outgoing_prev_ptr().is_none(), "s1 is head");
    assert_eq!(r1.get_outgoing_next_ptr(), Some(s2));

    let r2 = reader.get_synapse(s2);
    assert_eq!(r2.get_kind(), 20);
    assert_eq!(r2.get_outgoing_prev_ptr(), Some(s1));
    assert_eq!(r2.get_outgoing_next_ptr(), Some(s3));

    let r3 = reader.get_synapse(s3);
    assert_eq!(r3.get_kind(), 30);
    assert_eq!(r3.get_outgoing_prev_ptr(), Some(s2));
    assert!(r3.get_outgoing_next_ptr().is_none(), "s3 is tail");
}

#[test]
fn reader_does_not_see_unpublished_changes() {
    let h = setup();

    // publish initial state with one synapse
    let (src, tgt, s1) = {
        let node_chain = h.node_chain;
        let synapse_chain = h.synapse_chain.clone();

        let src = node_chain.insert_node(1).unwrap();
        let tgt = node_chain.insert_node(2).unwrap();
        let s1 = synapse_chain.connect(src, tgt, 10).unwrap();
        (src, tgt, s1)
    };
    h.writer.publish();
    h.reader.swap();

    // add second synapse but DON'T publish
    {
        let synapse_chain = h.synapse_chain.clone();
        synapse_chain.connect(src, tgt, 20).unwrap();
    }
    // no publish, no swap

    // reader still sees old snapshot
    let reader = h.synapse_chain_r;
    let r1 = reader.get_synapse(s1);
    assert_eq!(r1.get_kind(), 10);
    assert!(
        r1.get_outgoing_next_ptr().is_none(),
        "reader still sees s1 as tail"
    );
}

#[test]
fn reader_sees_disconnect_after_publish() {
    let h = setup();

    let s2 = {
        let node_chain = h.node_chain;
        let synapse_chain = h.synapse_chain;

        let src = node_chain.insert_node(1).unwrap();
        let tgt1 = node_chain.insert_node(2).unwrap();
        let tgt2 = node_chain.insert_node(3).unwrap();

        let s1 = synapse_chain.connect(src, tgt1, 10).unwrap();
        let s2 = synapse_chain.connect(src, tgt2, 20).unwrap();
        // outgoing: s1 -> s2

        synapse_chain.disconnect_synapse(s1).unwrap();
        // outgoing: s2
        s2
    };
    h.writer.publish();
    h.reader.swap();

    let reader = h.synapse_chain_r;
    let r2 = reader.get_synapse(s2);
    assert_eq!(r2.get_kind(), 20);
    assert!(
        r2.get_outgoing_prev_ptr().is_none(),
        "s2 is now head after disconnect"
    );
    assert!(r2.get_outgoing_next_ptr().is_none(), "s2 is also tail");
}
