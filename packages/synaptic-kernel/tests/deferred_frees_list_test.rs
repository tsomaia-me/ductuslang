use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::staging_buffer_reader::StagingBufferReader;
use synaptic_kernel::primitives::staging_buffer_writer::StagingBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_list(capacity: u32) -> (StagingBufferWriter, StagingBufferReader, AtomicBuffer) {
    let size = StagingBufferWriter::calculate_size_on_mem(capacity as usize);
    let mem: AtomicBuffer = (0..size).map(|_| AtomicI32::new(0)).collect();
    let mem_arc = mem;
    let list = StagingBufferWriter::new(Arc::clone(&mem_arc), 0, capacity);
    let reader = list.to_reader();
    (list, reader, mem_arc)
}

fn s(n: u32) -> SlotId {
    SlotId::new(n).unwrap()
}

#[test]
fn push_and_generation_gated_drain() {
    let (list, reader, _mem) = create_list(16);

    list.push(s(10)).unwrap();
    list.push(s(20)).unwrap();
    list.publish(); // gen → 2

    // Default ack=0, entries stamped gen=1. Drain yields gen <= 0, so nothing drains.
    let drained: Vec<SlotId> = list.drain().collect();
    assert!(drained.is_empty());

    // Push more, publish
    list.push(s(30)).unwrap();
    list.publish(); // gen → 3

    // Ack gen 2 (reader sees writer_gen=3, acks 2)
    reader.ack();
    let drained2: Vec<SlotId> = list.drain().collect();
    assert_eq!(drained2, vec![s(10), s(20), s(30)]);
}

#[test]
fn drain_clears_entries() {
    let (list, reader, _mem) = create_list(16);

    list.push(s(5)).unwrap();
    list.publish();
    reader.ack();
    list.drain().for_each(drop);

    // Should be empty
    let mut iter = list.drain();
    assert!(iter.next().is_none());
}

#[test]
fn copy_from_preserves_state_and_resizes() {
    let (small, small_reader, _mem_small) = create_list(16);
    small.push(s(5)).unwrap();
    small.push(s(10)).unwrap();
    small.publish(); // gen → 1
    small_reader.ack(); // acks 0
    small.drain().for_each(drop); // drain gen <= 0

    small.push(s(15)).unwrap();
    small.publish(); // gen → 2

    let (large, large_reader, _mem_large) = create_list(32);
    large.copy_from(&small);

    // Ack gen 1 on destination
    large_reader.ack(); // acks writer_gen-1 = 1
    let drained: Vec<SlotId> = large.drain().collect();
    assert_eq!(drained, vec![s(15)]);
    assert_eq!(large.len(), 0);
}

#[test]
#[should_panic]
fn copy_from_panics_if_source_larger() {
    let (small, _, _mem_small) = create_list(16);
    let (large, _, _mem_large) = create_list(32);
    small.copy_from(&large);
}
