use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::ring_buffer::RingBuffer;
use synaptic_kernel::primitives::types::AtomicBuffer;

const STRIDE: usize = 16;

/// Creates a AtomicBuffer with the given number of AtomicI32 slots.
fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

#[test]
fn read_empty_buffer_returns_none() {
    let mem = create_mem(1024);
    let ring: RingBuffer<4> = RingBuffer::new(mem, 0, 8);

    assert_eq!(ring.read(), None);
}

#[test]
fn write_and_read_single_entry() {
    let mem = create_mem(1024);
    let ring: RingBuffer<4> = RingBuffer::new(mem, 0, 8);

    let data = [1, 2, 3, 4];
    assert!(ring.write(data).is_ok());

    let result = ring.read();
    assert_eq!(result, Some([1, 2, 3, 4]));
}

#[test]
fn fifo_ordering() {
    let mem = create_mem(4096);
    let ring: RingBuffer<2> = RingBuffer::new(mem, 0, 8);

    ring.write([10, 20]).unwrap();
    ring.write([30, 40]).unwrap();
    ring.write([50, 60]).unwrap();

    assert_eq!(ring.read(), Some([10, 20]));
    assert_eq!(ring.read(), Some([30, 40]));
    assert_eq!(ring.read(), Some([50, 60]));
    assert_eq!(ring.read(), None);
}

#[test]
fn pending_count_tracks_entries() {
    let mem = create_mem(1024);
    let ring: RingBuffer<2> = RingBuffer::new(mem, 0, 8);

    assert_eq!(ring.pending_count(), 0);

    ring.write([1, 2]).unwrap();
    assert_eq!(ring.pending_count(), 1);

    ring.write([3, 4]).unwrap();
    assert_eq!(ring.pending_count(), 2);

    ring.read();
    assert_eq!(ring.pending_count(), 1);

    ring.read();
    assert_eq!(ring.pending_count(), 0);
}

#[test]
fn write_full_buffer_returns_error() {
    let mem = create_mem(4096);
    let ring: RingBuffer<2> = RingBuffer::new(mem, 0, 4);

    ring.write([1, 2]).unwrap();
    ring.write([3, 4]).unwrap();
    ring.write([5, 6]).unwrap();
    ring.write([7, 8]).unwrap();

    let result = ring.write([9, 10]);
    assert!(result.is_err());
}

#[test]
fn wrap_around_read_write() {
    let mem = create_mem(4096);
    let ring: RingBuffer<2> = RingBuffer::new(mem, 0, 4);

    // Fill and drain to advance read/write pointers
    for round in 0..3 {
        for i in 0..4 {
            let val = (round * 10 + i) as i32;
            ring.write([val, val + 100]).unwrap();
        }
        for i in 0..4 {
            let val = (round * 10 + i) as i32;
            assert_eq!(ring.read(), Some([val, val + 100]));
        }
    }

    // Buffer should be empty after full drain
    assert_eq!(ring.read(), None);
    assert_eq!(ring.pending_count(), 0);
}

#[test]
fn interleaved_read_write() {
    let mem = create_mem(4096);
    let ring: RingBuffer<3> = RingBuffer::new(mem, 0, 4);

    ring.write([1, 2, 3]).unwrap();
    ring.write([4, 5, 6]).unwrap();

    assert_eq!(ring.read(), Some([1, 2, 3]));

    ring.write([7, 8, 9]).unwrap();

    assert_eq!(ring.read(), Some([4, 5, 6]));
    assert_eq!(ring.read(), Some([7, 8, 9]));
    assert_eq!(ring.read(), None);
}

#[test]
fn mem_end_offset_is_correct() {
    let mem = create_mem(4096);
    let ring: RingBuffer<4> = RingBuffer::new(mem, 0, 8);

    // mem_end_offset = start(0) + header(3) + capacity(8) * STRIDE(4) = 35
    assert_eq!(ring.mem_end_offset(), 35);
}

#[test]
fn works_with_nonzero_start_index() {
    let mem = create_mem(4096);
    let start = 200;
    let ring: RingBuffer<2> = RingBuffer::new(mem, start, 4);

    ring.write([42, 84]).unwrap();
    assert_eq!(ring.read(), Some([42, 84]));
    assert_eq!(ring.pending_count(), 0);
}

#[test]
fn single_slot_size() {
    let mem = create_mem(1024);
    let ring: RingBuffer<1> = RingBuffer::new(mem, 0, 4);

    ring.write([99]).unwrap();
    assert_eq!(ring.read(), Some([99]));
}

// ============ Edge Cases ============

#[test]
fn exact_capacity_boundary() {
    let mem = create_mem(4096);
    let ring: RingBuffer<2> = RingBuffer::new(mem, 0, 4);

    // Fill to exact capacity
    for i in 0..4 {
        ring.write([i, i + 10]).unwrap();
    }
    assert_eq!(ring.pending_count(), 4);

    // Next write must fail
    assert!(ring.write([99, 99]).is_err());

    // Read one, freeing exactly one slot
    assert_eq!(ring.read(), Some([0, 10]));
    assert_eq!(ring.pending_count(), 3);

    // Now one write should succeed
    ring.write([99, 99]).unwrap();
    assert_eq!(ring.pending_count(), 4);

    // Next write must fail again
    assert!(ring.write([100, 100]).is_err());
}

#[test]
fn multiple_full_drain_cycles() {
    let mem = create_mem(4096);
    let ring: RingBuffer<2> = RingBuffer::new(mem, 0, 4);

    for round in 0..10 {
        // Fill completely
        for i in 0..4 {
            let val = (round * 100 + i) as i32;
            ring.write([val, val + 1]).unwrap();
        }
        assert_eq!(ring.pending_count(), 4);
        assert!(ring.write([0, 0]).is_err());

        // Drain completely
        for i in 0..4 {
            let val = (round * 100 + i) as i32;
            assert_eq!(ring.read(), Some([val, val + 1]));
        }
        assert_eq!(ring.pending_count(), 0);
        assert_eq!(ring.read(), None);
    }
}

#[test]
fn i32_extreme_values_in_ring() {
    let mem = create_mem(4096);
    let ring: RingBuffer<4> = RingBuffer::new(mem, 0, 4);

    ring.write([i32::MAX, i32::MIN, 0, -1]).unwrap();
    assert_eq!(ring.read(), Some([i32::MAX, i32::MIN, 0, -1]));
}

#[test]
fn zero_values_not_confused_with_empty() {
    let mem = create_mem(4096);
    let ring: RingBuffer<3> = RingBuffer::new(mem, 0, 4);

    ring.write([0, 0, 0]).unwrap();
    assert_eq!(ring.pending_count(), 1);
    assert_eq!(ring.read(), Some([0, 0, 0]));
    assert_eq!(ring.pending_count(), 0);
}

#[test]
fn capacity_of_one() {
    let mem = create_mem(1024);
    let ring: RingBuffer<2> = RingBuffer::new(mem, 0, 1);

    ring.write([42, 84]).unwrap();
    assert!(ring.write([1, 2]).is_err()); // full at 1

    assert_eq!(ring.read(), Some([42, 84]));
    assert_eq!(ring.read(), None);

    // Reuse the single slot
    ring.write([99, 100]).unwrap();
    assert_eq!(ring.read(), Some([99, 100]));
}

#[test]
fn large_slot_size() {
    let mem = create_mem(4096);
    let ring: RingBuffer<STRIDE> = RingBuffer::new(mem, 0, 4);

    let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    ring.write(data).unwrap();
    assert_eq!(ring.read(), Some(data));
}

#[test]
fn read_after_full_error_still_works() {
    let mem = create_mem(4096);
    let ring: RingBuffer<2> = RingBuffer::new(mem, 0, 2);

    ring.write([1, 2]).unwrap();
    ring.write([3, 4]).unwrap();

    // Buffer full — write fails
    assert!(ring.write([5, 6]).is_err());

    // Reads should still work normally
    assert_eq!(ring.read(), Some([1, 2]));
    assert_eq!(ring.read(), Some([3, 4]));
    assert_eq!(ring.read(), None);
}
