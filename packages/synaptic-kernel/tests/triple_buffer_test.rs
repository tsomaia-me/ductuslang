use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

// ============ Happy Paths ============

#[test]
fn new_creates_writer_and_reader() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 10);
    let reader = writer.to_reader();
    assert_eq!(writer.buffer_capacity(), 10);
    assert_eq!(reader.buffer_capacity(), 10);
}

#[test]
fn writer_can_write_and_publish() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);

    let base = writer.mem_writer_base();
    mem[base].store(42, Ordering::Relaxed);
    mem[base + 1].store(99, Ordering::Relaxed);
    writer.publish();
}

#[test]
fn reader_sees_published_data() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Write data to writer buffer
    let base = writer.mem_writer_base();
    mem[base].store(100, Ordering::Relaxed);
    mem[base + 1].store(200, Ordering::Relaxed);
    mem[base + 2].store(300, Ordering::Relaxed);
    mem[base + 3].store(400, Ordering::Relaxed);

    // Publish
    writer.publish();

    // Reader swaps — should get published data
    assert!(reader.swap());
    let rbase = reader.mem_reader_base();
    assert_eq!(mem[rbase].load(Ordering::Relaxed), 100);
    assert_eq!(mem[rbase + 1].load(Ordering::Relaxed), 200);
    assert_eq!(mem[rbase + 2].load(Ordering::Relaxed), 300);
    assert_eq!(mem[rbase + 3].load(Ordering::Relaxed), 400);
}

#[test]
fn reader_swap_returns_false_when_no_new_data() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 4);
    let reader = writer.to_reader();

    // No publish happened — reader should get false
    assert!(!reader.swap());
}

#[test]
fn reader_swap_returns_true_when_new_data() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 4);
    let reader = writer.to_reader();

    writer.publish();
    assert!(reader.swap());
}

#[test]
fn multiple_publish_reader_gets_latest() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Write #1
    let base1 = writer.mem_writer_base();
    mem[base1].store(111, Ordering::Relaxed);
    writer.publish();

    // Write #2 (overwrites shared before reader consumes)
    let base2 = writer.mem_writer_base();
    mem[base2].store(222, Ordering::Relaxed);
    writer.publish();

    // Reader should see 222 (latest), not 111
    assert!(reader.swap());
    let rbase = reader.mem_reader_base();
    assert_eq!(mem[rbase].load(Ordering::Relaxed), 222);
}

#[test]
fn reader_keeps_old_data_when_no_new_publish() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Publish once
    let base = writer.mem_writer_base();
    mem[base].store(42, Ordering::Relaxed);
    writer.publish();

    // Reader swaps
    assert!(reader.swap());
    let rbase = reader.mem_reader_base();
    assert_eq!(mem[rbase].load(Ordering::Relaxed), 42);

    // No new publish — reader swap returns false, keeps same data
    assert!(!reader.swap());
    let rbase2 = reader.mem_reader_base();
    assert_eq!(rbase2, rbase); // same buffer
    assert_eq!(mem[rbase2].load(Ordering::Relaxed), 42);
}

#[test]
fn full_cycle_writer_reader_writer() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Cycle 1
    let b1 = writer.mem_writer_base();
    mem[b1].store(10, Ordering::Relaxed);
    writer.publish();
    assert!(reader.swap());
    let rbase1 = reader.mem_reader_base();
    assert_eq!(mem[rbase1].load(Ordering::Relaxed), 10);

    // Cycle 2
    let b2 = writer.mem_writer_base();
    assert_ne!(b1, b2, "Writer should get a new buffer");
    mem[b2].store(20, Ordering::Relaxed);
    writer.publish();
    assert!(reader.swap());
    let rbase2 = reader.mem_reader_base();
    assert_ne!(rbase1, rbase2, "Reader should get a new buffer");
    assert_eq!(mem[rbase2].load(Ordering::Relaxed), 20);

    // Cycle 3
    let b3 = writer.mem_writer_base();
    assert_ne!(b2, b3, "Writer should get a new buffer");
    assert_ne!(b1, b3, "Writer should get a new buffer");
    mem[b3].store(30, Ordering::Relaxed);
    writer.publish();
    assert!(reader.swap());
    let rbase3 = reader.mem_reader_base();
    assert_ne!(rbase2, rbase3, "Reader should get a new buffer");
    assert_ne!(rbase1, rbase3, "Reader should get a new buffer");
    assert_eq!(mem[rbase3].load(Ordering::Relaxed), 30);
}

#[test]
fn mem_end_offset_is_correct() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 10);
    let reader = writer.to_reader();

    // Layout: 4 metadata slots + 3 × 10 buffer slots = 34
    assert_eq!(writer.mem_end_offset(), 4 + 3 * 10);
    assert_eq!(reader.mem_end_offset(), 4 + 3 * 10);
}

#[test]
fn nonzero_start_index() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 100, 8);
    let reader = writer.to_reader();

    assert_eq!(writer.mem_end_offset(), 100 + 4 + 3 * 8);
    assert_eq!(reader.mem_end_offset(), 100 + 4 + 3 * 8);

    let base = writer.mem_writer_base();
    assert!(base >= 104); // at least after metadata
    mem[base].store(55, Ordering::Relaxed);
    writer.publish();

    assert!(reader.swap());
    let rbase = reader.mem_reader_base();
    assert_eq!(mem[rbase].load(Ordering::Relaxed), 55);
}

// ============ Edge Cases ============

#[test]
fn buffer_size_of_one() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 1);
    let reader = writer.to_reader();

    let base = writer.mem_writer_base();
    mem[base].store(777, Ordering::Relaxed);
    writer.publish();

    assert!(reader.swap());
    let rbase = reader.mem_reader_base();
    assert_eq!(mem[rbase].load(Ordering::Relaxed), 777);
}

#[test]
fn writer_publishes_many_times_without_reader_consuming() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Writer publishes 10 times without reader ever swapping
    for i in 0..10 {
        let base = writer.mem_writer_base();
        mem[base].store(i * 100, Ordering::Relaxed);
        writer.publish();
    }

    // Reader should see latest (900)
    assert!(reader.swap());
    let rbase = reader.mem_reader_base();
    assert_eq!(mem[rbase].load(Ordering::Relaxed), 900);
}

#[test]
fn reader_swaps_many_times_without_new_publishes() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Publish once
    let base = writer.mem_writer_base();
    mem[base].store(42, Ordering::Relaxed);
    writer.publish();

    // Reader swaps first time — gets data
    assert!(reader.swap());

    // Reader swaps 10 more times — all return false
    for _ in 0..10 {
        assert!(!reader.swap());
    }
}

#[test]
fn writer_reader_alternating_rapidly() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 2);
    let reader = writer.to_reader();

    for i in 0..100 {
        let base = writer.mem_writer_base();
        mem[base].store(i, Ordering::Relaxed);
        mem[base + 1].store(i * 10, Ordering::Relaxed);
        writer.publish();

        assert!(reader.swap());
        let rbase = reader.mem_reader_base();
        assert_eq!(mem[rbase].load(Ordering::Relaxed), i);
        assert_eq!(mem[rbase + 1].load(Ordering::Relaxed), i * 10);
    }
}

#[test]
fn all_three_buffers_are_distinct() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    let mut seen_bases = std::collections::HashSet::new();

    // Init: writer=0, shared=1, reader=2
    // Writer starts with buffer 0
    seen_bases.insert(writer.mem_writer_base());

    // Publish: writer gives 0 to shared, takes 1. Now writer=1, shared=0, reader=2
    writer.publish();
    seen_bases.insert(writer.mem_writer_base());

    // Reader swaps: reader gives 2 to shared, takes 0. Now writer=1, shared=2, reader=0
    reader.swap();
    seen_bases.insert(reader.mem_reader_base());

    // Publish: writer gives 1 to shared, takes 2. Now writer=2, shared=1, reader=0
    writer.publish();
    seen_bases.insert(writer.mem_writer_base());

    // All three buffers should have been observed
    assert_eq!(
        seen_bases.len(),
        3,
        "expected 3 distinct buffer bases, got {:?}",
        seen_bases
    );
}

#[test]
fn writer_buffer_sync_after_publish() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);

    // Write data to writer buffer
    let base1 = writer.mem_writer_base();
    mem[base1].store(111, Ordering::Relaxed);
    mem[base1 + 1].store(222, Ordering::Relaxed);
    mem[base1 + 2].store(333, Ordering::Relaxed);
    mem[base1 + 3].store(444, Ordering::Relaxed);

    // Publish — the sync inside publish() should copy data to the new writer buffer
    writer.publish();

    let base2 = writer.mem_writer_base();
    assert_ne!(base1, base2); // different buffer

    // New writer buffer should have the synced data
    assert_eq!(mem[base2].load(Ordering::Relaxed), 111);
    assert_eq!(mem[base2 + 1].load(Ordering::Relaxed), 222);
    assert_eq!(mem[base2 + 2].load(Ordering::Relaxed), 333);
    assert_eq!(mem[base2 + 3].load(Ordering::Relaxed), 444);
}

#[test]
fn sync_correctness_across_multiple_publishes() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Write field 0 = 10
    let base = writer.mem_writer_base();
    mem[base].store(10, Ordering::Relaxed);
    writer.publish();

    // After sync, writer's new buffer should have field 0 = 10
    let base = writer.mem_writer_base();
    assert_eq!(mem[base].load(Ordering::Relaxed), 10);

    // Now update field 1 = 20, keep field 0 as synced
    mem[base + 1].store(20, Ordering::Relaxed);
    writer.publish();

    // Both fields should persist in new writer
    let base = writer.mem_writer_base();
    assert_eq!(mem[base].load(Ordering::Relaxed), 10);
    assert_eq!(mem[base + 1].load(Ordering::Relaxed), 20);

    // Reader should see both fields
    reader.swap();
    let rbase = reader.mem_reader_base();
    assert_eq!(mem[rbase].load(Ordering::Relaxed), 10);
    assert_eq!(mem[rbase + 1].load(Ordering::Relaxed), 20);
}

#[test]
fn incremental_writes_dont_lose_prior_data() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 8);
    let reader = writer.to_reader();

    // Write all 8 fields
    let base = writer.mem_writer_base();
    for i in 0..8 {
        mem[base + i].store((i + 1) as i32 * 100, Ordering::Relaxed);
    }
    writer.publish();
    reader.swap();

    // Reader sees all 8
    let rbase = reader.mem_reader_base();
    for i in 0..8 {
        assert_eq!(mem[rbase + i].load(Ordering::Relaxed), (i + 1) as i32 * 100);
    }

    // Now only update field 3
    let base = writer.mem_writer_base();
    mem[base + 3].store(999, Ordering::Relaxed);
    writer.publish();
    reader.swap();

    // Reader should see field 3 changed, others preserved
    let rbase = reader.mem_reader_base();
    assert_eq!(mem[rbase].load(Ordering::Relaxed), 100);
    assert_eq!(mem[rbase + 1].load(Ordering::Relaxed), 200);
    assert_eq!(mem[rbase + 2].load(Ordering::Relaxed), 300);
    assert_eq!(mem[rbase + 3].load(Ordering::Relaxed), 999); // updated
    assert_eq!(mem[rbase + 4].load(Ordering::Relaxed), 500);
    assert_eq!(mem[rbase + 5].load(Ordering::Relaxed), 600);
    assert_eq!(mem[rbase + 6].load(Ordering::Relaxed), 700);
    assert_eq!(mem[rbase + 7].load(Ordering::Relaxed), 800);
}

// ============ Bind (Reconnect to Existing AtomicBuffer) ============

#[test]
fn bind_reconnects_to_existing_state() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);

    // Write and publish
    let base = writer.mem_writer_base();
    mem[base].store(42, Ordering::Relaxed);
    writer.publish();

    // Bind new reader to same AtomicBuffer region
    let writer_bind = TripleBufferWriter::bind(mem.clone(), 0, 4);
    let reader2 = writer_bind.to_reader();

    // Reader2 should see the published data
    assert!(reader2.swap());
    let rbase = reader2.mem_reader_base();
    assert_eq!(mem[rbase].load(Ordering::Relaxed), 42);
}

#[test]
fn bind_does_not_reinitialize_state() {
    let mem = create_mem(4096);
    let state_slot = 0;

    // Initialize
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let base = writer.mem_writer_base();
    mem[base].store(99, Ordering::Relaxed);
    writer.publish();

    // Capture state before bind
    let state_before = mem[state_slot].load(Ordering::Relaxed);

    // Bind — should NOT overwrite state
    let _writer2 = TripleBufferWriter::bind(mem.clone(), 0, 4);
    let _reader2 = _writer2.to_reader();
    let state_after = mem[state_slot].load(Ordering::Relaxed);

    assert_eq!(state_before, state_after);
}

// ============ Super Edge Cases ============

#[test]
fn dropped_frames_preserve_cumulative_state() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Frame 1: set field 0 = 10
    let base = writer.mem_writer_base();
    mem[base].store(10, Ordering::Relaxed);
    writer.publish();

    // Frame 2: set field 1 = 20 (field 0 synced from previous)
    let base = writer.mem_writer_base();
    mem[base + 1].store(20, Ordering::Relaxed);
    writer.publish();

    // Frame 3: set field 2 = 30
    let base = writer.mem_writer_base();
    mem[base + 2].store(30, Ordering::Relaxed);
    writer.publish();

    // Reader only swaps ONCE — should see cumulative state
    assert!(reader.swap());
    let rbase = reader.mem_reader_base();
    assert_eq!(mem[rbase].load(Ordering::Relaxed), 10);
    assert_eq!(mem[rbase + 1].load(Ordering::Relaxed), 20);
    assert_eq!(mem[rbase + 2].load(Ordering::Relaxed), 30);
}

#[test]
fn writer_and_reader_never_see_same_buffer() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    for _ in 0..50 {
        // Writer writes and publishes
        writer.publish();
        reader.swap();

        // Writer and reader should NEVER have the same base
        assert_ne!(
            writer.mem_writer_base(),
            reader.mem_reader_base(),
            "writer and reader must never share a buffer"
        );
    }
}

#[test]
fn published_slot_index_tracks_latest_publish() {
    let mem = create_mem(4096);
    let published_slot = 2; // mem_start_offset + 2

    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Initially 0
    assert_eq!(
        mem[published_slot].load(Ordering::Relaxed),
        0,
        "initially 0"
    );

    // After 1st publish (writer had buffer 0)
    writer.publish();
    assert_eq!(
        mem[published_slot].load(Ordering::Relaxed),
        0,
        "published 0"
    );

    // Let reader swap so writer gets a new buffer
    reader.swap();

    // After 2nd publish (writer had buffer 1)
    writer.publish();
    assert_eq!(
        mem[published_slot].load(Ordering::Relaxed),
        1,
        "published 1"
    );

    // Let reader swap
    reader.swap();

    // After 3rd publish (writer had buffer 2)
    writer.publish();
    assert_eq!(
        mem[published_slot].load(Ordering::Relaxed),
        2,
        "published 2"
    );
}

#[test]
fn state_encoding_new_data_flag() {
    let mem = create_mem(4096);
    let state_slot = 0;

    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Initial state: NEW_DATA bit should be set (0b100) because initial state is 0b001
    // Actually initial state: state = 0b001 (shared=1, no new data — wait, the init sets 0b001)
    let state = mem[state_slot].load(Ordering::Relaxed);
    // Init: state = 0b001 → shared_idx=1, NEW_DATA=0
    assert_eq!(state & 0b100, 0, "no new data initially");

    // After publish: NEW_DATA should be set
    writer.publish();
    let state = mem[state_slot].load(Ordering::Relaxed);
    assert_ne!(state & 0b100, 0, "NEW_DATA set after publish");

    // After reader swap: NEW_DATA should be cleared
    reader.swap();
    let state = mem[state_slot].load(Ordering::Relaxed);
    assert_eq!(state & 0b100, 0, "NEW_DATA cleared after reader swap");
}

#[test]
fn large_buffer_data_integrity() {
    let buffer_size: u32 = 1024;
    let mem = create_mem(4 + buffer_size as usize * 3 + 100);
    let writer = TripleBufferWriter::new(mem.clone(), 0, buffer_size);
    let reader = writer.to_reader();

    // Write a pattern to all slots
    let base = writer.mem_writer_base();
    for i in 0..buffer_size as usize {
        mem[base + i].store((i as i32) * 7 + 3, Ordering::Relaxed);
    }
    writer.publish();

    // Reader should see the exact pattern
    assert!(reader.swap());
    let rbase = reader.mem_reader_base();
    for i in 0..buffer_size as usize {
        assert_eq!(
            mem[rbase + i].load(Ordering::Relaxed),
            (i as i32) * 7 + 3,
            "mismatch at index {i}"
        );
    }
}

#[test]
fn sync_preserves_data_through_all_three_buffers() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Write data, publish, reader consumes — exercising all 3 buffers
    let mut expected = [0i32; 4];

    for round in 0..6 {
        let base = writer.mem_writer_base();

        // Each round, update one field
        let field = round % 4;
        expected[field] = (round + 1) as i32 * 100;
        mem[base + field].store(expected[field], Ordering::Relaxed);

        writer.publish();
        assert!(reader.swap());

        let rbase = reader.mem_reader_base();
        for f in 0..4 {
            assert_eq!(
                mem[rbase + f].load(Ordering::Relaxed),
                expected[f],
                "round {round}, field {f}: expected {}, got {}",
                expected[f],
                mem[rbase + f].load(Ordering::Relaxed)
            );
        }
    }
}

#[test]
fn concurrent_writer_reader_stress() {
    let buffer_size: u32 = 64;
    let mem = create_mem(4 + buffer_size as usize * 3 + 100);

    let writer = TripleBufferWriter::new(mem.clone(), 0, buffer_size);
    let reader = writer.to_reader();

    let mem_writer = mem.clone();
    let mem_reader = mem.clone();
    let iterations = 10_000;

    // Writer thread: write incrementing values, publish
    let writer_handle = thread::spawn(move || {
        for i in 0..iterations {
            let base = writer.mem_writer_base();
            // Write a sentinel pattern: [i, i, i, ..., i]
            for j in 0..buffer_size as usize {
                mem_writer[base + j].store(i as i32, Ordering::Relaxed);
            }
            writer.publish();
        }
    });

    // Reader thread: swap and verify consistency within each frame
    let reader_handle = thread::spawn(move || {
        let mut frames_read = 0u64;
        let mut last_val = -1i32;

        for _ in 0..iterations * 2 {
            if reader.swap() {
                let rbase = reader.mem_reader_base();
                let first = mem_reader[rbase].load(Ordering::Relaxed);

                // Data is written in monotonically increasing order. The protocol
                // guarantees the reader only gets newer frames (or the same frame),
                // so it must never see a value from the past.
                assert!(
                    first >= last_val,
                    "reader went backwards in time: got {first}, expected >= {last_val}"
                );
                last_val = first;

                // All values in the frame should be the same (consistency)
                for j in 1..buffer_size as usize {
                    let val = mem_reader[rbase + j].load(Ordering::Relaxed);
                    assert_eq!(
                        val, first,
                        "torn frame at index {j}: got {val}, expected {first}"
                    );
                }

                frames_read += 1;
            }
            // Small yield to allow interleaving
            thread::yield_now();
        }

        assert!(
            frames_read > 0,
            "reader should have consumed at least one frame"
        );
        frames_read
    });

    writer_handle.join().expect("writer panicked");
    let frames = reader_handle.join().expect("reader panicked");
    assert!(frames > 0);
}

#[test]
fn concurrent_high_frequency_publish() {
    let buffer_size: u32 = 8;
    let mem = create_mem(4 + buffer_size as usize * 3 + 100);
    let writer = TripleBufferWriter::new(mem.clone(), 0, buffer_size);
    let reader = writer.to_reader();

    let mem_w = mem.clone();
    let mem_r = mem.clone();

    // Writer publishes as fast as possible
    let writer_handle = thread::spawn(move || {
        for i in 0..50_000i32 {
            let base = writer.mem_writer_base();
            mem_w[base].store(i, Ordering::Relaxed);
            writer.publish();
        }
    });

    // Reader swaps periodically
    let reader_handle = thread::spawn(move || {
        let mut reads = 0u64;
        for _ in 0..5_000 {
            if reader.swap() {
                let rbase = reader.mem_reader_base();
                let _val = mem_r[rbase].load(Ordering::Relaxed);
                reads += 1;
            }
            thread::yield_now();
        }
        reads
    });

    writer_handle.join().expect("writer panicked");
    let reads = reader_handle.join().expect("reader panicked");
    // Reader should see some frames (dropped frames are expected)
    assert!(reads > 0, "reader saw zero frames");
}

#[test]
fn writer_gets_back_coherent_buffer_after_reader_consumes() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Round 1: writer writes [1, 2, 3, 4], publishes, reader consumes
    let base = writer.mem_writer_base();
    for i in 0..4 {
        mem[base + i].store((i + 1) as i32, Ordering::Relaxed);
    }
    writer.publish();
    reader.swap();

    // Round 2: writer writes [5, 6, 7, 8], publishes, reader consumes
    let base = writer.mem_writer_base();
    for i in 0..4 {
        mem[base + i].store((i + 5) as i32, Ordering::Relaxed);
    }
    writer.publish();
    reader.swap();

    // Round 3: writer's buffer should be synced (should have [5, 6, 7, 8])
    // because publish() syncs the stale writer from the published buffer
    let base = writer.mem_writer_base();
    assert_eq!(mem[base].load(Ordering::Relaxed), 5);
    assert_eq!(mem[base + 1].load(Ordering::Relaxed), 6);
    assert_eq!(mem[base + 2].load(Ordering::Relaxed), 7);
    assert_eq!(mem[base + 3].load(Ordering::Relaxed), 8);
}

#[test]
fn two_triple_buffers_on_same_mem() {
    let mem = create_mem(8192);

    // First triple buffer at offset 0, size 10
    let w1 = TripleBufferWriter::new(mem.clone(), 0, 10);
    let r1 = w1.to_reader();
    // Second triple buffer at offset after first one ends
    let offset2 = w1.mem_end_offset();
    let w2 = TripleBufferWriter::new(mem.clone(), offset2, 8);
    let r2 = w2.to_reader();

    // Write different data to each
    let b1 = w1.mem_writer_base();
    mem[b1].store(1111, Ordering::Relaxed);
    w1.publish();

    let b2 = w2.mem_writer_base();
    mem[b2].store(2222, Ordering::Relaxed);
    w2.publish();

    // Each reader sees its own data
    r1.swap();
    r2.swap();

    let rb1 = r1.mem_reader_base();
    let rb2 = r2.mem_reader_base();
    assert_eq!(mem[rb1].load(Ordering::Relaxed), 1111);
    assert_eq!(mem[rb2].load(Ordering::Relaxed), 2222);
}

#[test]
fn reader_stability_during_no_publishes() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    // Publish once
    let base = writer.mem_writer_base();
    mem[base].store(42, Ordering::Relaxed);
    writer.publish();
    reader.swap();

    let rbase = reader.mem_reader_base();

    // Reader swaps 100 times with no publishes — buffer should be stable
    for _ in 0..100 {
        assert!(!reader.swap());
        assert_eq!(reader.mem_reader_base(), rbase);
        assert_eq!(mem[rbase].load(Ordering::Relaxed), 42);
    }
}

#[test]
fn publish_swap_publish_swap_never_corrupts() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    let reader = writer.to_reader();

    let mut accumulated = [0i32; 4];

    for round in 0..20 {
        let base = writer.mem_writer_base();
        accumulated[round % 4] += 1;

        for i in 0..4 {
            mem[base + i].store(accumulated[i], Ordering::Relaxed);
        }

        writer.publish();
        assert!(reader.swap());

        let rbase = reader.mem_reader_base();
        for i in 0..4 {
            assert_eq!(
                mem[rbase + i].load(Ordering::Relaxed),
                accumulated[i],
                "corruption at round {round}, field {i}"
            );
        }
    }
}

#[test]
fn publish_does_not_corrupt_surrounding_mem_memory() {
    let padding: usize = 100;
    let buffer_size: u32 = 64;
    // Layout: padding + 4 metadata slots + 3*64 buffer slots + padding
    let mem = create_mem(padding + 4 + buffer_size as usize * 3 + padding);

    // Fill the ENTIRE AtomicBuffer with a sentinel value
    for i in 0..mem.len() {
        mem[i].store(7777, Ordering::Relaxed);
    }

    let mem_start_offset = padding;
    let writer = TripleBufferWriter::new(mem.clone(), mem_start_offset, buffer_size);
    let reader = writer.to_reader();

    // Perform bulk writes and rapid publishes to heavily exercise the memcpy loop
    for round in 0..10_000 {
        let base = writer.mem_writer_base();

        // Write pattern into the writer buffer
        for i in 0..buffer_size as usize {
            mem[base + i].store((round as i32) * 100 + (i as i32), Ordering::Relaxed);
        }

        writer.publish();

        // Let the reader swap occasionally to force buffer rotation
        if round % 3 == 0 {
            reader.swap();
        }
    }

    // 1. Verify leading memory is completely untouched
    for i in 0..padding {
        assert_eq!(
            mem[i].load(Ordering::Relaxed),
            7777,
            "Memory corruption (underflow) before mem_start_offset at mem index {i}"
        );
    }

    // 2. Verify trailing memory is completely untouched
    let mem_end_offset = writer.mem_end_offset();
    for i in mem_end_offset..mem.len() {
        assert_eq!(
            mem[i].load(Ordering::Relaxed),
            7777,
            "Memory corruption (overflow) after mem_end_offset at mem index {i}"
        );
    }

    // 3. Verify metadata slots did not get swept up in an overflowing memcpy
    let state = mem[mem_start_offset].load(Ordering::Relaxed);
    assert!((0..=7).contains(&state), "State slot corrupted: {state}");

    let writer_id = mem[mem_start_offset + 1].load(Ordering::Relaxed);
    assert!(
        (0..=2).contains(&writer_id),
        "Writer ID slot corrupted: {writer_id}"
    );

    let published_id = mem[mem_start_offset + 2].load(Ordering::Relaxed);
    assert!(
        (0..=2).contains(&published_id),
        "Published ID slot corrupted: {published_id}"
    );

    let reader_id = mem[mem_start_offset + 3].load(Ordering::Relaxed);
    assert!(
        (0..=2).contains(&reader_id),
        "Reader ID slot corrupted: {reader_id}"
    );
}

// ============================================================
// TripleBufferWriter::write / read (offset-based accessors)
// ============================================================

#[test]
fn writer_write_read_offset_zero() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 4);
    writer.write(0, 42);
    assert_eq!(writer.read(0), 42);
}

#[test]
fn writer_write_read_offset_middle() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 8);
    writer.write(4, 77);
    assert_eq!(writer.read(4), 77);
}

#[test]
fn writer_write_read_offset_last() {
    let mem = create_mem(4096);
    let capacity: u32 = 10;
    let writer = TripleBufferWriter::new(mem, 0, capacity);
    writer.write(capacity as usize - 1, -123);
    assert_eq!(writer.read(capacity as usize - 1), -123);
}

#[test]
fn writer_write_read_roundtrip_every_index() {
    let mem = create_mem(4096);
    let capacity: u32 = 16;
    let writer = TripleBufferWriter::new(mem, 0, capacity);
    for i in 0..capacity as usize {
        writer.write(i, (i as i32) * 3 - 7);
    }
    for i in 0..capacity as usize {
        assert_eq!(writer.read(i), (i as i32) * 3 - 7);
    }
}

#[test]
fn writer_read_returns_synced_values_after_publish() {
    let mem = create_mem(4096);
    let capacity = 4;
    let writer = TripleBufferWriter::new(mem, 0, capacity);
    writer.write(0, 10);
    writer.write(1, 20);
    writer.write(2, 30);
    writer.write(3, 40);
    writer.publish();
    // After publish, writer's new buffer must be synced from the published buffer.
    assert_eq!(writer.read(0), 10);
    assert_eq!(writer.read(1), 20);
    assert_eq!(writer.read(2), 30);
    assert_eq!(writer.read(3), 40);
}

// ============================================================
// TripleBufferReader::read (offset-based accessor)
// ============================================================

#[test]
fn reader_read_returns_published_at_every_offset() {
    let mem = create_mem(4096);
    let capacity: u32 = 8;
    let writer = TripleBufferWriter::new(mem, 0, capacity);
    let reader = writer.to_reader();
    for i in 0..capacity as usize {
        writer.write(i, (i as i32 + 1) * 5);
    }
    writer.publish();
    assert!(reader.swap());
    for i in 0..capacity as usize {
        assert_eq!(reader.read(i), (i as i32 + 1) * 5);
    }
}

#[test]
fn reader_read_stable_when_swap_returns_false() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 4);
    let reader = writer.to_reader();
    writer.write(0, 11);
    writer.write(1, 22);
    writer.write(2, 33);
    writer.write(3, 44);
    writer.publish();
    assert!(reader.swap());
    // No new publish; swap continues to return false and reader.read
    // continues to return the previously-seen values.
    for _ in 0..50 {
        assert!(!reader.swap());
        assert_eq!(reader.read(0), 11);
        assert_eq!(reader.read(1), 22);
        assert_eq!(reader.read(2), 33);
        assert_eq!(reader.read(3), 44);
    }
}

// ============================================================
// TripleBufferWriter::write_batch / read_batch
// ============================================================

#[test]
fn writer_write_batch_read_batch_zero_offset() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 6);
    writer.write_batch(0, &[10, 20, 30]);
    let mut out = [0i32; 3];
    writer.read_batch(0, &mut out);
    assert_eq!(out, [10, 20, 30]);
}

#[test]
fn writer_write_batch_nonzero_offset_regression() {
    // read_batch(offset, out) must read starting at `offset`, not at buffer start.
    // Using offset 5 so the first 5 slots remain zero and read_batch at offset
    // 0 can distinguish "bug: reads from start" from "correct: reads at offset".
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 10);
    writer.write_batch(5, &[111, 222, 333]);
    let mut out = [0i32; 3];
    writer.read_batch(5, &mut out);
    assert_eq!(out, [111, 222, 333]);
    // The batch values must NOT leak to offsets outside the batch.
    assert_eq!(writer.read(0), 0);
    assert_eq!(writer.read(4), 0);
    assert_eq!(writer.read(8), 0);
    assert_eq!(writer.read(9), 0);
    // If read_batch mistakenly ignored the offset, this would return
    // [111, 222, 333] instead of the (correct) [0, 0, 0].
    let mut z = [0i32; 3];
    writer.read_batch(0, &mut z);
    assert_eq!(z, [0, 0, 0]);
}

#[test]
fn writer_write_batch_offset_matrix() {
    let mem = create_mem(4096);
    let capacity: u32 = 16;
    let writer = TripleBufferWriter::new(mem, 0, capacity);
    let t = 4usize;
    let cap = capacity as usize;

    for &offset in &[0usize, 1, cap / 2, cap - t] {
        // Reset buffer to zero before each iteration.
        for i in 0..cap {
            writer.write(i, 0);
        }
        let batch = [
            offset as i32,
            offset as i32 + 1,
            offset as i32 + 2,
            offset as i32 + 3,
        ];
        writer.write_batch(offset, &batch);
        let mut rb = [0i32; 4];
        writer.read_batch(offset, &mut rb);
        assert_eq!(rb, batch, "roundtrip failed at offset {offset}");
        for i in 0..cap {
            if i >= offset && i < offset + t {
                continue;
            }
            assert_eq!(
                writer.read(i),
                0,
                "leakage at index {i} while writing batch at offset {offset}"
            );
        }
    }
}

#[test]
fn writer_write_batch_fills_exact_remainder() {
    // offset + T == capacity.
    let mem = create_mem(4096);
    let capacity: u32 = 10;
    let cap = capacity as usize;
    let writer = TripleBufferWriter::new(mem, 0, capacity);
    writer.write_batch(cap - 4, &[1, 2, 3, 4]);
    let mut out = [0i32; 4];
    writer.read_batch(cap - 4, &mut out);
    assert_eq!(out, [1, 2, 3, 4]);
    assert_eq!(writer.read(cap - 1), 4);
}

#[test]
fn writer_write_batch_t_one() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 5);
    writer.write_batch(2, &[99]);
    let mut one = [0i32; 1];
    writer.read_batch(2, &mut one);
    assert_eq!(one, [99]);
    assert_eq!(writer.read(2), 99);
}

#[test]
fn writer_write_batch_entire_buffer() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 8);
    let data = [10, 20, 30, 40, 50, 60, 70, 80];
    writer.write_batch(0, &data);
    let mut out = [0i32; 8];
    writer.read_batch(0, &mut out);
    assert_eq!(out, data);
}

#[test]
fn writer_write_batch_then_individual_read() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 8);
    writer.write_batch(2, &[7, 8, 9, 10]);
    assert_eq!(writer.read(2), 7);
    assert_eq!(writer.read(3), 8);
    assert_eq!(writer.read(4), 9);
    assert_eq!(writer.read(5), 10);
}

#[test]
fn writer_individual_write_then_read_batch() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 8);
    writer.write(3, 100);
    writer.write(4, 200);
    writer.write(5, 300);
    let mut out = [0i32; 3];
    writer.read_batch(3, &mut out);
    assert_eq!(out, [100, 200, 300]);
}

// ============================================================
// TripleBufferReader::read_batch
// ============================================================

#[test]
fn reader_read_batch_nonzero_offset_regression() {
    // read_batch(offset, out) must read starting at `offset`, not at buffer start.
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 10);
    let reader = writer.to_reader();
    writer.write_batch(4, &[500, 600, 700]);
    writer.publish();
    assert!(reader.swap());
    let mut out = [0i32; 3];
    reader.read_batch(4, &mut out);
    assert_eq!(out, [500, 600, 700]);
    // A batch read from offset 0 must NOT return the values written at offset 4.
    let mut z = [0i32; 3];
    reader.read_batch(0, &mut z);
    assert_eq!(z, [0, 0, 0]);
}

#[test]
fn reader_read_batch_offset_matrix() {
    let mem = create_mem(4096);
    let capacity: u32 = 16;
    let cap = capacity as usize;
    let t = 4usize;
    let writer = TripleBufferWriter::new(mem, 0, capacity);
    let reader = writer.to_reader();

    for &offset in &[0usize, 1, cap / 2, cap - t] {
        for i in 0..cap {
            writer.write(i, 0);
        }
        writer.write_batch(offset, &[1, 2, 3, 4]);
        writer.publish();
        assert!(reader.swap(), "no new data for offset {offset}");
        let mut rb = [0i32; 4];
        reader.read_batch(offset, &mut rb);
        assert_eq!(rb, [1, 2, 3, 4], "mismatch at offset {offset}");
    }
}

#[test]
fn reader_read_batch_fills_exact_remainder() {
    let mem = create_mem(4096);
    let capacity: u32 = 10;
    let cap = capacity as usize;
    let writer = TripleBufferWriter::new(mem, 0, capacity);
    let reader = writer.to_reader();
    writer.write_batch(cap - 4, &[21, 22, 23, 24]);
    writer.publish();
    assert!(reader.swap());
    let mut out = [0i32; 4];
    reader.read_batch(cap - 4, &mut out);
    assert_eq!(out, [21, 22, 23, 24]);
}

// ============================================================
// publish sync semantics (writer-side invariants)
// ============================================================

#[test]
fn publish_sync_single_offset_persists_on_writer() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 4);
    writer.write(2, 77);
    writer.publish();
    // After publish, writer.read(2) must reflect the value we wrote,
    // because the new writer buffer was synced from the published buffer.
    assert_eq!(writer.read(2), 77);
}

#[test]
fn publish_sync_preserves_values_across_two_publishes() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 4);
    writer.write(0, 100);
    writer.publish();
    assert_eq!(writer.read(0), 100);
    writer.write(2, 300);
    writer.publish();
    // After the second sync, BOTH offsets must be present in the writer buffer.
    assert_eq!(writer.read(0), 100);
    assert_eq!(writer.read(2), 300);
}

#[test]
fn publish_read_coherent_after_writer_cycles_all_buffers() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 4);
    let reader = writer.to_reader();
    writer.write(0, 1);
    writer.publish();
    assert!(reader.swap());
    writer.write(1, 2);
    writer.publish();
    assert!(reader.swap());
    writer.write(2, 3);
    writer.publish();
    assert!(reader.swap());
    // After three full rotations, the writer buffer must still hold
    // a coherent view of every field that was ever written.
    assert_eq!(writer.read(0), 1);
    assert_eq!(writer.read(1), 2);
    assert_eq!(writer.read(2), 3);
}

// ============================================================
// TripleBufferWriter::copy_metadata_from
// ============================================================

#[test]
fn copy_metadata_from_mirrors_all_four_slots() {
    let mem_a = create_mem(4096);
    let mem_b = create_mem(4096);
    let writer_a = TripleBufferWriter::new(mem_a.clone(), 0, 4);
    let reader_a = writer_a.to_reader();

    // Drive A into a non-initial state with publish + swap + publish.
    writer_a.write(0, 123);
    writer_a.publish();
    assert!(reader_a.swap());
    writer_a.write(0, 456);
    writer_a.publish();

    let writer_b = TripleBufferWriter::new(mem_b.clone(), 0, 4);

    let a_state = mem_a[0].load(Ordering::Relaxed);
    let a_writer = mem_a[1].load(Ordering::Relaxed);
    let a_published = mem_a[2].load(Ordering::Relaxed);
    let a_reader = mem_a[3].load(Ordering::Relaxed);

    writer_b.copy_metadata_from(&writer_a);

    assert_eq!(mem_b[0].load(Ordering::Relaxed), a_state, "state");
    assert_eq!(mem_b[1].load(Ordering::Relaxed), a_writer, "writer_id");
    assert_eq!(
        mem_b[2].load(Ordering::Relaxed),
        a_published,
        "published_id"
    );
    assert_eq!(mem_b[3].load(Ordering::Relaxed), a_reader, "reader_id");
}

#[test]
fn copy_metadata_from_does_not_touch_buffer_region() {
    let mem_a = create_mem(4096);
    let mem_b = create_mem(4096);
    let writer_a = TripleBufferWriter::new(mem_a.clone(), 0, 4);
    writer_a.write(0, 123);
    writer_a.publish();
    let writer_b = TripleBufferWriter::new(mem_b.clone(), 0, 4);

    // Pre-fill B's buffer region (slots 4..=15) with a sentinel.
    for i in 4..(4 + 3 * 4) {
        mem_b[i].store(7777, Ordering::Relaxed);
    }
    writer_b.copy_metadata_from(&writer_a);
    for i in 4..(4 + 3 * 4) {
        assert_eq!(
            mem_b[i].load(Ordering::Relaxed),
            7777,
            "buffer slot {i} was touched by copy_metadata_from"
        );
    }
}

#[test]
fn copy_metadata_from_panics_when_source_capacity_larger() {
    let mem_a = create_mem(4096);
    let mem_b = create_mem(4096);
    let writer_a = TripleBufferWriter::new(mem_a.clone(), 0, 8);
    let writer_b = TripleBufferWriter::new(mem_b.clone(), 0, 4);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        writer_b.copy_metadata_from(&writer_a);
    }));
    assert!(
        result.is_err(),
        "expected debug_assert to fire when source capacity exceeds destination"
    );
}

// ============================================================
// TripleBufferWriter::copy_region_from
// ============================================================

#[test]
fn copy_region_from_copies_all_three_buffer_planes() {
    let capacity: u32 = 8;
    let cap = capacity as usize;
    let a_start = 0usize;
    let a_end = 4 + cap * 3;
    let b_start = a_end;
    let total = b_start + 4 + cap * 3;
    let mem = create_mem(total);

    let writer_a = TripleBufferWriter::new(mem.clone(), a_start, capacity);
    let writer_b = TripleBufferWriter::new(mem.clone(), b_start, capacity);

    // Fill each of A's three buffer planes with a distinct pattern.
    // Layout: A's buffer_bases = [a_start+4, a_start+4+cap, a_start+4+2*cap]
    for plane in 0..3 {
        let base = a_start + 4 + plane * cap;
        for i in 0..cap {
            mem[base + i].store(1000 + (plane as i32) * 100 + (i as i32), Ordering::Relaxed);
        }
    }

    // Pre-fill ALL of B's buffer region with a sentinel 7777.
    for i in (b_start + 4)..(b_start + 4 + cap * 3) {
        mem[i].store(7777, Ordering::Relaxed);
    }

    let source_offset = 2usize;
    let destination_offset = 3usize;
    let count = 4usize;

    writer_b.copy_region_from(&writer_a, source_offset, destination_offset, count);

    for plane in 0..3 {
        let a_plane_base = a_start + 4 + plane * cap;
        let b_plane_base = b_start + 4 + plane * cap;

        // Copied range: values match A.
        for k in 0..count {
            let expected = mem[a_plane_base + source_offset + k].load(Ordering::Relaxed);
            let got = mem[b_plane_base + destination_offset + k].load(Ordering::Relaxed);
            assert_eq!(
                got, expected,
                "plane {plane}, k {k}: expected {expected}, got {got}"
            );
        }

        // Outside the destination range, B must still hold the sentinel.
        for i in 0..cap {
            if i >= destination_offset && i < destination_offset + count {
                continue;
            }
            assert_eq!(
                mem[b_plane_base + i].load(Ordering::Relaxed),
                7777,
                "plane {plane}, slot {i} was modified outside the copied region"
            );
        }
    }
}

#[test]
fn copy_region_from_source_offset_zero_destination_offset_zero_full_capacity() {
    let capacity: u32 = 8;
    let cap = capacity as usize;
    let a_start = 0usize;
    let a_end = 4 + cap * 3;
    let b_start = a_end;
    let total = b_start + 4 + cap * 3;
    let mem = create_mem(total);

    let writer_a = TripleBufferWriter::new(mem.clone(), a_start, capacity);
    let writer_b = TripleBufferWriter::new(mem.clone(), b_start, capacity);

    for plane in 0..3 {
        let base = a_start + 4 + plane * cap;
        for i in 0..cap {
            mem[base + i].store((plane as i32) * 10 + (i as i32) + 1, Ordering::Relaxed);
        }
    }

    writer_b.copy_region_from(&writer_a, 0, 0, cap);

    for plane in 0..3 {
        let a_plane_base = a_start + 4 + plane * cap;
        let b_plane_base = b_start + 4 + plane * cap;
        for i in 0..cap {
            assert_eq!(
                mem[b_plane_base + i].load(Ordering::Relaxed),
                mem[a_plane_base + i].load(Ordering::Relaxed),
                "plane {plane} slot {i} mismatch"
            );
        }
    }
}

#[test]
fn copy_region_from_panics_on_destination_out_of_bounds() {
    let capacity: u32 = 8;
    let mem = create_mem(4096);
    let writer_a = TripleBufferWriter::new(mem.clone(), 0, capacity);
    let writer_b = TripleBufferWriter::new(mem.clone(), 100, capacity);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // destination_offset + count = 5 + 5 = 10 > capacity (8)
        writer_b.copy_region_from(&writer_a, 0, 5, 5);
    }));
    assert!(result.is_err());
}

#[test]
fn copy_region_from_panics_on_source_out_of_bounds() {
    let capacity: u32 = 8;
    let mem = create_mem(4096);
    let writer_a = TripleBufferWriter::new(mem.clone(), 0, capacity);
    let writer_b = TripleBufferWriter::new(mem.clone(), 100, capacity);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // source_offset + count = 5 + 5 = 10 > source.capacity (8)
        writer_b.copy_region_from(&writer_a, 5, 0, 5);
    }));
    assert!(result.is_err());
}

// ============================================================
// TripleBufferWriter::bind — sync semantics
// ============================================================

#[test]
fn bind_sync_is_noop_when_writer_index_equals_published_index() {
    // After new(), writer_index == 0 == published_index, so bind's internal
    // sync must be a no-op and must not mutate any buffer contents.
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);

    // Distinctive pattern in the writer's current buffer (buffer 0).
    writer.write(0, 100);
    writer.write(1, 200);
    writer.write(2, 300);
    writer.write(3, 400);

    // Sentinels in the other two buffer planes to prove they're untouched.
    for plane in 1..3 {
        for i in 0..4 {
            mem[4 + plane * 4 + i].store(9999, Ordering::Relaxed);
        }
    }

    let bound = TripleBufferWriter::bind(mem.clone(), 0, 4);

    // Writer buffer unchanged.
    assert_eq!(bound.read(0), 100);
    assert_eq!(bound.read(1), 200);
    assert_eq!(bound.read(2), 300);
    assert_eq!(bound.read(3), 400);

    // Non-writer buffers unchanged (no sync copy happened).
    for plane in 1..3 {
        for i in 0..4 {
            assert_eq!(
                mem[4 + plane * 4 + i].load(Ordering::Relaxed),
                9999,
                "plane {plane} slot {i} was modified during bind's sync"
            );
        }
    }
}

#[test]
fn bind_syncs_writer_buffer_from_published_when_indices_differ() {
    // After publish: writer_index != published_index. On bind, sync must
    // copy the published buffer's contents into the writer's current buffer.
    //
    // NOTE: The SPSC contract forbids using both the original writer and the
    // bound writer concurrently; this test merely observes bind's one-time
    // sync behavior.
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    writer.write(0, 10);
    writer.write(1, 20);
    writer.write(2, 30);
    writer.write(3, 40);
    writer.publish();

    // Deliberately zero out the writer's current (post-publish) buffer so
    // we can detect whether bind re-populates it from the published buffer.
    for i in 0..4 {
        writer.write(i, 0);
    }

    let bound = TripleBufferWriter::bind(mem.clone(), 0, 4);

    assert_eq!(bound.read(0), 10);
    assert_eq!(bound.read(1), 20);
    assert_eq!(bound.read(2), 30);
    assert_eq!(bound.read(3), 40);
}

#[test]
fn bind_bound_writer_to_reader_sees_last_published_data() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem.clone(), 0, 4);
    writer.write(0, 901);
    writer.write(1, 902);
    writer.write(2, 903);
    writer.write(3, 904);
    writer.publish();

    let bound = TripleBufferWriter::bind(mem.clone(), 0, 4);
    let reader = bound.to_reader();
    assert!(reader.swap());
    assert_eq!(reader.read(0), 901);
    assert_eq!(reader.read(1), 902);
    assert_eq!(reader.read(2), 903);
    assert_eq!(reader.read(3), 904);
}

// ============================================================
// Getters (buffer_capacity / mem_start_offset / mem_end_offset)
// ============================================================

#[test]
fn getters_match_construction_parameters() {
    let mem = create_mem(8192);

    let w1 = TripleBufferWriter::new(mem.clone(), 0, 10);
    assert_eq!(w1.buffer_capacity(), 10);
    assert_eq!(w1.mem_start_offset(), 0);
    assert_eq!(w1.mem_end_offset(), 4 + 3 * 10);

    let w2 = TripleBufferWriter::new(mem.clone(), 50, 1);
    assert_eq!(w2.buffer_capacity(), 1);
    assert_eq!(w2.mem_start_offset(), 50);
    assert_eq!(w2.mem_end_offset(), 50 + 4 + 3 * 1);

    let w3 = TripleBufferWriter::new(mem.clone(), 100, 1024);
    assert_eq!(w3.buffer_capacity(), 1024);
    assert_eq!(w3.mem_start_offset(), 100);
    assert_eq!(w3.mem_end_offset(), 100 + 4 + 3 * 1024);

    let r1 = w1.to_reader();
    assert_eq!(r1.buffer_capacity(), 10);
    assert_eq!(r1.mem_start_offset(), 0);
    assert_eq!(r1.mem_end_offset(), 4 + 3 * 10);
}

// ============================================================
// calculate_size_on_mem
// ============================================================

#[test]
fn calculate_size_on_mem_matches_contract() {
    assert_eq!(TripleBufferWriter::calculate_size_on_mem(1), 4 + 1 * 3);
    assert_eq!(TripleBufferWriter::calculate_size_on_mem(8), 4 + 8 * 3);
    assert_eq!(
        TripleBufferWriter::calculate_size_on_mem(1024),
        4 + 1024 * 3
    );
    assert_eq!(
        TripleBufferWriter::calculate_size_on_mem(65_536),
        4 + 65_536 * 3
    );
}
