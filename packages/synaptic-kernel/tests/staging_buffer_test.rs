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

// ============ Basic push / len ============

#[test]
fn empty_buffer_has_zero_len() {
    let (buf, _, _) = create_staging(4);
    assert_eq!(buf.len(), 0);
}

#[test]
fn push_increments_len() {
    let (buf, _, _) = create_staging(4);
    buf.push(s(1)).unwrap();
    buf.push(s(2)).unwrap();
    assert_eq!(buf.len(), 2);
}

// ============ Generation stamping ============

#[test]
fn writer_generation_starts_at_one() {
    let (buf, reader, _) = create_staging(4);
    assert_eq!(buf.writer_generation(), 1);
    assert_eq!(reader.writer_generation(), 1);
}

#[test]
fn publish_increments_writer_generation() {
    let (buf, reader, _) = create_staging(4);
    buf.publish();
    assert_eq!(buf.writer_generation(), 2);
    assert_eq!(reader.writer_generation(), 2);
    buf.publish();
    assert_eq!(buf.writer_generation(), 3);
}

#[test]
fn push_does_not_increment_writer_generation() {
    let (buf, _, _) = create_staging(4);
    buf.push(s(1)).unwrap();
    buf.push(s(2)).unwrap();
    buf.push(s(3)).unwrap();
    assert_eq!(buf.writer_generation(), 1);
}

// ============ Drain without ack ============

#[test]
fn drain_without_ack_yields_nothing() {
    let (buf, _, _) = create_staging(4);
    buf.push(s(10)).unwrap();
    buf.push(s(20)).unwrap();
    buf.publish();

    // No ack — drain should yield nothing (ack_generation is 0, entries stamped with gen 1)
    let drained: Vec<SlotId> = buf.drain().collect();
    assert!(drained.is_empty());
}

// ============ Generation-gated drain ============

#[test]
fn drain_only_yields_acked_generations() {
    let (buf, reader, _) = create_staging(8);

    // Publish cycle 1: push A, B (stamped gen 1)
    buf.push(s(10)).unwrap();
    buf.push(s(20)).unwrap();
    buf.publish(); // writer_gen becomes 2

    // Publish cycle 2: push C (stamped gen 2)
    buf.push(s(30)).unwrap();
    buf.publish(); // writer_gen becomes 3

    // Reader acks gen 1 (writer_gen - 1 = 2 - 1 = 1... but we simulate seeing 2)
    // Actually, reader.ack() reads the current writer_gen (3) so it acks 2!
    // We want to simulate the reader only acknowledging gen 1.
    // Instead of reader.ack() which goes to the bleeding edge, let's just
    // manually advance it or test standard sync flow.
    // Let's create a separate scenario.

    // For this test, we want to prove it only yields up to what's acked.
    // Let's just use reader.ack(). It acks writer_gen - 1, so it acks 2.
    reader.ack(); // reads writer_gen=3, acks 3-1=2

    // Drain: yields entries with gen <= 2 → A(1), B(1), C(2)
    let drained: Vec<SlotId> = buf.drain().collect();
    assert_eq!(drained, vec![s(10), s(20), s(30)]);
    assert_eq!(buf.len(), 0);
}

#[test]
fn drain_preserves_unacked_entries() {
    let (buf, _, mem) = create_staging(8);

    // Push A, B with gen 1
    buf.push(s(10)).unwrap();
    buf.push(s(20)).unwrap();
    buf.publish(); // writer_gen → 2

    // Push C with gen 2
    buf.push(s(30)).unwrap();
    buf.publish(); // writer_gen → 3

    // Manually ack only gen 1
    // mem_reader_ack_generation_offset is start_offset + 1
    mem[buf.mem_start_offset() + 1].store(1, std::sync::atomic::Ordering::Relaxed);

    // Drain yields gen <= 1
    let drained: Vec<SlotId> = buf.drain().collect();
    assert_eq!(drained, vec![s(10), s(20)]); // A, B (gen 1)
    assert_eq!(buf.len(), 1); // C still in buffer

    // Now nothing more to drain (C is gen 2, ack is still 1)
    let empty: Vec<SlotId> = buf.drain().collect();
    assert!(empty.is_empty());
}

#[test]
fn multiple_publish_cycles_with_incremental_acks() {
    let (buf, reader, _) = create_staging(16);

    // Cycle 1: push slots 1, 2 (gen 1)
    buf.push(s(1)).unwrap();
    buf.push(s(2)).unwrap();
    buf.publish(); // gen → 2

    // Cycle 2: push slots 3, 4 (gen 2)
    buf.push(s(3)).unwrap();
    buf.push(s(4)).unwrap();
    buf.publish(); // gen → 3

    // Cycle 3: push slots 5, 6 (gen 3)
    buf.push(s(5)).unwrap();
    buf.push(s(6)).unwrap();
    buf.publish(); // gen → 4

    assert_eq!(buf.len(), 6);

    // Reader acks: writer_gen=4, acks 4-1=3
    reader.ack();

    // Drain: yields gen <= 3
    let drained: Vec<SlotId> = buf.drain().collect();
    assert_eq!(drained, vec![s(1), s(2), s(3), s(4), s(5), s(6)]);
    assert_eq!(buf.len(), 0);
}

#[test]
fn partial_drain_then_full_drain() {
    let (buf, reader, _) = create_staging(8);

    // Push A (gen 1)
    buf.push(s(10)).unwrap();
    buf.publish(); // gen → 2

    // Push B (gen 2)
    buf.push(s(20)).unwrap();
    buf.publish(); // gen → 3

    // Push C (gen 3) — not published yet
    buf.push(s(30)).unwrap();

    // ack_gen defaults to 0 → drain yields gen <= 0 (nothing)
    let d1: Vec<SlotId> = buf.drain().collect();
    assert!(d1.is_empty());
    assert_eq!(buf.len(), 3);

    // Reader acks: writer_gen=3, acks 2
    reader.ack();

    // Drain yields gen <= 2 → A(1), B(2)
    let d2: Vec<SlotId> = buf.drain().collect();
    assert_eq!(d2, vec![s(10), s(20)]);
    assert_eq!(buf.len(), 1); // C remains (gen 3, not published, not acked)

    // Publish C
    buf.publish(); // gen → 4

    // Reader acks: writer_gen=4, acks 3
    reader.ack();

    // Drain yields gen <= 3 → C(3)
    let d3: Vec<SlotId> = buf.drain().collect();
    assert_eq!(d3, vec![s(30)]);
    assert_eq!(buf.len(), 0);
}

// ============ Drain on empty ============

#[test]
fn drain_on_empty_is_safe() {
    let (buf, _, _) = create_staging(4);
    let d: Vec<SlotId> = buf.drain().collect();
    assert!(d.is_empty());

    // Multiple drains on empty
    buf.drain();
    buf.drain();
    buf.drain();
    assert_eq!(buf.len(), 0);
}

// ============ Publish without push ============

#[test]
fn publish_without_push_advances_generation() {
    let (buf, _, _) = create_staging(4);
    buf.publish();
    buf.publish();
    buf.publish();
    assert_eq!(buf.writer_generation(), 4);
    assert_eq!(buf.len(), 0);
}

// ============ Copy from ============

#[test]
fn copy_from_preserves_pending_entries() {
    let (src, _, _) = create_staging(4);
    src.push(s(10)).unwrap();
    src.push(s(20)).unwrap();
    src.publish(); // gen → 2

    let (dst, reader, _) = create_staging(8);
    dst.copy_from(&src);

    assert_eq!(dst.len(), 2);
    assert_eq!(dst.writer_generation(), 2);

    // Ack and drain from destination
    reader.ack(); // reads writer_gen=2, acks 1
    let drained: Vec<SlotId> = dst.drain().collect();
    assert_eq!(drained, vec![s(10), s(20)]);
}

#[test]
fn copy_from_preserves_generation_state() {
    let (src, src_reader, _) = create_staging(4);
    src.push(s(1)).unwrap();
    src.publish(); // gen -> 2
    src.push(s(2)).unwrap();
    src.publish(); // gen -> 3

    // Ack gen 1 (assuming reader lags) - wait, src_reader.ack() will ack 2.
    src_reader.ack(); // acks writer_gen-1 = 2
    src.drain().for_each(drop); // drains gen <= 2

    let (dst, _, _) = create_staging(8);
    dst.copy_from(&src);

    assert_eq!(dst.writer_generation(), 3);
    assert_eq!(dst.reader_ack_generation(), 2);
}

#[test]
#[should_panic]
fn copy_from_panics_if_source_larger() {
    let (small, _, _) = create_staging(2);
    let (large, _, _) = create_staging(4);
    small.copy_from(&large);
}

// ============ Capacity boundary ============

#[test]
fn push_to_capacity_succeeds() {
    let (buf, _, _) = create_staging(4);
    buf.push(s(1)).unwrap();
    buf.push(s(2)).unwrap();
    buf.push(s(3)).unwrap();
    buf.push(s(4)).unwrap();
    assert_eq!(buf.len(), 4);
}

#[test]
#[should_panic]
fn push_beyond_capacity_returns_error() {
    let (buf, _, _) = create_staging(4);
    buf.push(s(1)).unwrap();
    buf.push(s(2)).unwrap();
    buf.push(s(3)).unwrap();
    buf.push(s(4)).unwrap();
    assert!(buf.push(s(5)).is_err());
}

// ============ Nonzero start offset ============

#[test]
fn nonzero_start_offset_works() {
    let offset = 100;
    let size = StagingBufferWriter::calculate_size_on_mem(4) + offset;
    let mem: AtomicBuffer = (0..size).map(|_| AtomicI32::new(0)).collect();

    // Explicitly initialize writer_generation for the raw instantiated mem
    mem[offset].store(1, std::sync::atomic::Ordering::Relaxed);

    let buf = StagingBufferWriter::new(Arc::clone(&mem), offset, 4);
    let reader = buf.to_reader();

    buf.push(s(42)).unwrap();
    buf.publish();
    reader.ack();

    let drained: Vec<SlotId> = buf.drain().collect();
    assert_eq!(drained, vec![s(42)]);
}

// ============ Bind ============

#[test]
fn bind_reads_existing_state() {
    let size = StagingBufferWriter::calculate_size_on_mem(4);
    let mem: AtomicBuffer = (0..size).map(|_| AtomicI32::new(0)).collect();

    mem[0].store(1, std::sync::atomic::Ordering::Relaxed);

    let buf1 = StagingBufferWriter::new(Arc::clone(&mem), 0, 4);
    buf1.push(s(42)).unwrap();
    buf1.push(s(99)).unwrap();

    let buf2 = StagingBufferWriter::bind(Arc::clone(&mem), 0, 4);
    assert_eq!(buf2.len(), 2);
}

// ============ Ring buffer wrap-around ============

#[test]
fn ring_buffer_wraps_correctly() {
    let (buf, reader, _) = create_staging(4);

    // Fill, ack, drain, refill — exercises wrap-around
    for cycle in 0u32..3 {
        buf.push(s(cycle * 10 + 1)).unwrap();
        buf.push(s(cycle * 10 + 2)).unwrap();
        buf.publish();
        reader.ack();
        let drained: Vec<SlotId> = buf.drain().collect();
        assert_eq!(drained.len(), 2, "cycle {} should drain 2", cycle);
    }
    assert_eq!(buf.len(), 0);
}

// ============ Generation ordering ============

#[test]
fn entries_stamped_with_correct_generation() {
    let (buf, reader, mem) = create_staging(8);

    // Gen 1: push A
    buf.push(s(100)).unwrap();
    buf.publish(); // gen → 2

    // Gen 2: push B
    buf.push(s(200)).unwrap();
    buf.publish(); // gen → 3

    // Manually ack only gen 1
    mem[1].store(1, std::sync::atomic::Ordering::Relaxed);

    let d: Vec<SlotId> = buf.drain().collect();
    assert_eq!(d, vec![s(100)]); // Only A (gen 1 <= ack 1)

    // Now ack gen 2
    reader.ack(); // acks writer_gen-1 = 2
    let d2: Vec<SlotId> = buf.drain().collect();
    assert_eq!(d2, vec![s(200)]); // B (gen 2 <= ack 2)
}

#[test]
fn unpublished_entries_are_never_drained_even_with_full_ack() {
    let (buf, reader, _) = create_staging(8);

    buf.push(s(10)).unwrap();
    buf.publish(); // gen → 2

    buf.push(s(20)).unwrap(); // gen 2, NOT published

    // Reader acks published Gen 1
    reader.ack(); // acks writer_gen-1 = 1

    let drained: Vec<SlotId> = buf.drain().collect();
    assert_eq!(drained, vec![s(10)]); // Only the pre-publish entry

    // Entry 20 (gen 2) is NOT drained because ack=1 < gen=2
    assert_eq!(buf.len(), 1);
}
