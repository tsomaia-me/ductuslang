use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::staging_buffer_reader::StagingBufferReader;
use synaptic_kernel::primitives::staging_buffer_writer::StagingBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_staging(capacity: u32) -> (StagingBufferWriter, StagingBufferReader, AtomicBuffer) {
    let size = StagingBufferWriter::calculate_size_on_mem(capacity as usize);
    let mem: AtomicBuffer = (0..size).map(|_| AtomicI32::new(0)).collect();
    let buffer = StagingBufferWriter::new(Arc::clone(&mem), 0, capacity);
    let reader = buffer.to_reader();
    (buffer, reader, mem)
}

fn s(n: u32) -> SlotId {
    SlotId::new(n).unwrap()
}

// ============ copy_from: generation-aware ============

#[test]
fn copy_from_preserves_pending_entries_and_generations() {
    let (src, _, _) = create_staging(4);

    // Push A, B with gen 1
    src.push(s(10)).unwrap();
    src.push(s(20)).unwrap();
    src.publish(); // gen → 2

    // Push C with gen 2
    src.push(s(30)).unwrap();

    let (dst, reader, _) = create_staging(8);
    dst.copy_from(&src);

    assert_eq!(dst.len(), 3);
    assert_eq!(dst.writer_generation(), 2); // copied from src

    // Ack gen 1 → drain A, B
    reader.ack(); // acks writer_gen-1 = 1
    let d1: Vec<SlotId> = dst.drain().collect();
    assert_eq!(d1, vec![s(10), s(20)]);

    // C (gen 2) still in buffer
    assert_eq!(dst.len(), 1);
}

#[test]
fn copy_from_to_larger_capacity() {
    let (src, src_reader, _) = create_staging(4);

    src.push(s(1)).unwrap();
    src.push(s(2)).unwrap();
    src.publish(); // gen → 2
    src_reader.ack(); // acks 1

    // Drain gen 1
    let d: Vec<SlotId> = src.drain().collect();
    assert_eq!(d, vec![s(1), s(2)]);

    // Push more
    src.push(s(3)).unwrap();
    src.push(s(4)).unwrap();
    src.publish(); // gen → 3

    let (dst, dst_reader, _) = create_staging(8);
    dst.copy_from(&src);

    assert_eq!(dst.len(), 2); // 3, 4
    assert_eq!(dst.writer_generation(), 3);
    assert_eq!(dst.reader_ack_generation(), 1); // copied ack

    // Ack up to gen 2
    dst_reader.ack(); // acks writer_gen-1 = 2
    let d2: Vec<SlotId> = dst.drain().collect();
    assert_eq!(d2, vec![s(3), s(4)]);
}

// ============ Multi-cycle ordering ============

#[test]
fn drain_preserves_insertion_order_across_generations() {
    let (buf, reader, _) = create_staging(16);

    buf.push(s(1)).unwrap();
    buf.push(s(2)).unwrap();
    buf.publish(); // gen → 2

    buf.push(s(3)).unwrap();
    buf.push(s(4)).unwrap();
    buf.publish(); // gen → 3

    // Ack all
    reader.ack(); // acks 2

    let drained: Vec<SlotId> = buf.drain().collect();
    assert_eq!(drained, vec![s(1), s(2), s(3), s(4)]); // FIFO order preserved
}

#[test]
fn interleaved_push_publish_ack_drain() {
    let (buf, reader, _) = create_staging(8);

    // Cycle 1
    buf.push(s(10)).unwrap();
    buf.publish(); // gen → 2

    // Ack gen 1, drain
    reader.ack(); // acks 1
    let d1: Vec<SlotId> = buf.drain().collect();
    assert_eq!(d1, vec![s(10)]);

    // Cycle 2
    buf.push(s(20)).unwrap();
    buf.push(s(30)).unwrap();
    buf.publish(); // gen → 3

    // Ack gen 2, drain
    reader.ack(); // acks 2
    let d2: Vec<SlotId> = buf.drain().collect();
    assert_eq!(d2, vec![s(20), s(30)]);

    // Cycle 3: empty publish
    buf.publish(); // gen → 4
    reader.ack(); // acks 3
    let d3: Vec<SlotId> = buf.drain().collect();
    assert!(d3.is_empty());

    assert_eq!(buf.len(), 0);
}

// ============ Bind preserves generation state ============

#[test]
fn bind_preserves_generation_state() {
    let size = StagingBufferWriter::calculate_size_on_mem(4);
    let mem: AtomicBuffer = (0..size).map(|_| AtomicI32::new(0)).collect();

    let buf1 = StagingBufferWriter::new(Arc::clone(&mem), 0, 4);
    buf1.push(s(42)).unwrap();
    buf1.publish();
    buf1.push(s(99)).unwrap();
    buf1.publish();

    let buf2 = StagingBufferWriter::bind(Arc::clone(&mem), 0, 4);
    assert_eq!(buf2.len(), 2);
    assert_eq!(buf2.writer_generation(), 3);
}

// ============ Stress: many cycles ============

#[test]
fn many_push_publish_ack_drain_cycles() {
    let (buf, reader, _) = create_staging(16);

    // SlotId is 1-based (NonZeroU32), so iterate from 1.
    for i in 1u32..=50 {
        buf.push(s(i)).unwrap();
        buf.publish();
        reader.ack();
        let drained: Vec<SlotId> = buf.drain().collect();
        assert_eq!(drained, vec![s(i)], "cycle {}", i);
    }

    assert_eq!(buf.len(), 0);
    assert_eq!(buf.writer_generation(), 51);
}

#[test]
fn batch_push_then_batch_drain() {
    let (buf, reader, _) = create_staging(16);

    // Push 8 items across 4 publish cycles. Slots are 1-based (NonZeroU32),
    // so use a 1-based scheme: i*2+1 and i*2+2 produces 1,2,3,4,5,6,7,8.
    for i in 0u32..4 {
        buf.push(s(i * 2 + 1)).unwrap();
        buf.push(s(i * 2 + 2)).unwrap();
        buf.publish();
    }

    assert_eq!(buf.len(), 8);

    // Ack up to gen 4 (all published)
    reader.ack(); // acks writer_gen-1 = 4

    let drained: Vec<SlotId> = buf.drain().collect();
    assert_eq!(
        drained,
        vec![s(1), s(2), s(3), s(4), s(5), s(6), s(7), s(8)]
    );
    assert_eq!(buf.len(), 0);
}

// ============ Edge: ack before any publish ============

#[test]
fn ack_before_any_publish_is_noop() {
    let (buf, reader, _) = create_staging(4);

    buf.push(s(10)).unwrap();

    // Writer gen is 1, reader.ack() → acks 0. Gen 1 > Gen 0 -> noop
    reader.ack();

    // Drain yields nothing
    let drained: Vec<SlotId> = buf.drain().collect();
    assert!(drained.is_empty());
}

// ============ Edge: publish without push between acks ============

#[test]
fn empty_publish_cycles_dont_break_generation_tracking() {
    let (buf, reader, _) = create_staging(8);

    buf.push(s(1)).unwrap();
    buf.publish(); // gen → 2

    // Empty publishes
    buf.publish(); // gen → 3
    buf.publish(); // gen → 4

    buf.push(s(2)).unwrap(); // stamped gen 4
    buf.publish(); // gen → 5

    // Ack gen 4
    reader.ack(); // acks writer_gen-1 = 4

    let drained: Vec<SlotId> = buf.drain().collect();
    assert_eq!(drained, vec![s(1), s(2)]); // both acked (gen 1 <= 4, gen 4 <= 4)
}
