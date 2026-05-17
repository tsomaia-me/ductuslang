use std::sync::atomic::{AtomicI32, Ordering};
use synaptic_kernel::primitives::mem_zone_reader::MemZoneReader;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

#[test]
fn new_creates_reader_at_start_index() {
    let mem = create_mem(128);
    let view = MemZoneReader::new(&mem, 16, 0);

    for i in 0..16 {
        assert_eq!(view.read(i), 0);
    }
}

#[test]
fn reads_reflect_backing_buffer_state() {
    let mem = create_mem(128);
    // Directly poke values into the backing buffer at offset 10..10+16.
    for (i, v) in [
        500, -42, 0, 1, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47,
    ]
    .into_iter()
    .enumerate()
    {
        mem[10 + i].store(v, Ordering::Relaxed);
    }
    let view = MemZoneReader::new(&mem, 16, 10);

    assert_eq!(view.read(0), 500);
    assert_eq!(view.read(1), -42);
    assert_eq!(view.read(15), 47);
}

#[test]
fn read_all_matches_individual_reads() {
    let mem = create_mem(128);
    for i in 0..16 {
        mem[i].store((i as i32) * 7 - 3, Ordering::Relaxed);
    }
    let view = MemZoneReader::new(&mem, 16, 0);

    let mut all = [0i32; 16];
    view.read_all(&mut all);
    for i in 0..16 {
        assert_eq!(all[i], view.read(i));
    }
}

#[test]
fn fields_do_not_bleed_across_offsets() {
    let mem = create_mem(128);
    mem[0].store(i32::MAX, Ordering::Relaxed);
    mem[2].store(i32::MIN, Ordering::Relaxed);
    let view = MemZoneReader::new(&mem, 16, 0);

    assert_eq!(view.read(0), i32::MAX);
    assert_eq!(view.read(1), 0);
    assert_eq!(view.read(2), i32::MIN);
    assert_eq!(view.read(3), 0);
}

#[test]
fn two_readers_share_same_backing_see_identical_values() {
    let mem = create_mem(128);
    mem[10].store(999, Ordering::Relaxed);
    let view_a = MemZoneReader::new(&mem, 16, 10);
    let view_b = MemZoneReader::new(&mem, 16, 10);

    assert_eq!(view_a.read(0), 999);
    assert_eq!(view_b.read(0), 999);
}

#[test]
fn independent_offsets_independent_data() {
    let mem = create_mem(128);
    mem[0].store(100, Ordering::Relaxed);
    mem[16].store(200, Ordering::Relaxed);
    let view_a = MemZoneReader::new(&mem, 16, 0);
    let view_b = MemZoneReader::new(&mem, 16, 16);

    assert_eq!(view_a.read(0), 100);
    assert_eq!(view_b.read(0), 200);
}

#[test]
fn works_with_different_stride_sizes() {
    let mem = create_mem(128);
    mem[0].store(111, Ordering::Relaxed);
    mem[64].store(222, Ordering::Relaxed);
    let view_8 = MemZoneReader::new(&mem, 8, 0);
    let view_16 = MemZoneReader::new(&mem, 16, 64);

    assert_eq!(view_8.read(0), 111);
    assert_eq!(view_16.read(0), 222);
}

#[test]
fn reader_observes_late_writer_mutations() {
    // MemZoneReader is a view over live atomic memory — a writer touching the
    // same offsets after reader construction must be observable on subsequent
    // reads, with no stale caching in the reader.
    let mem = create_mem(128);
    let view = MemZoneReader::new(&mem, 16, 0);
    assert_eq!(view.read(5), 0);

    mem[5].store(12345, Ordering::Relaxed);
    assert_eq!(view.read(5), 12345);
}

#[test]
#[should_panic(expected = "MemZoneReader::new | range")]
fn new_panics_if_out_of_bounds() {
    let mem = create_mem(10);
    let _view = MemZoneReader::new(&mem, 16, 0);
}

#[test]
#[should_panic(expected = "MemZoneReader::new | range")]
fn new_panics_if_start_index_crosses_bounds() {
    let mem = create_mem(32);
    let _view = MemZoneReader::new(&mem, 16, 20);
}

#[test]
#[should_panic(expected = "MemZoneReader.read | offset")]
fn read_panics_on_out_of_bounds_offset() {
    let mem = create_mem(32);
    let view = MemZoneReader::new(&mem, 16, 0);
    let _ = view.read(16);
}
