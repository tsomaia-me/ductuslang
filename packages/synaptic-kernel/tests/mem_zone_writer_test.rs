use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::mem_zone_writer::MemZoneWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

#[test]
fn new_creates_view_at_start_index() {
    let mem = create_mem(128);
    let view = MemZoneWriter::new(&mem, 16, 0);

    for i in 0..16 {
        assert_eq!(view.read(i), 0);
    }
}

#[test]
fn raw_read_write_round_trip() {
    let mem = create_mem(128);
    let view = MemZoneWriter::new(&mem, 16, 10);

    view.write(0, 500);
    view.write(15, -42);

    assert_eq!(view.read(0), 500);
    assert_eq!(view.read(15), -42);
}

#[test]
fn fields_do_not_bleed() {
    let mem = create_mem(128);
    let view = MemZoneWriter::new(&mem, 16, 0);

    view.write(0, i32::MAX);
    assert_eq!(view.read(1), 0);

    view.write(1, i32::MIN);
    assert_eq!(view.read(0), i32::MAX);
}

#[test]
fn two_views_different_offsets_are_independent() {
    let mem = create_mem(128);
    let view_a = MemZoneWriter::new(&mem, 16, 0);
    let view_b = MemZoneWriter::new(&mem, 16, 16);

    view_a.write(0, 100);
    view_b.write(0, 200);

    assert_eq!(view_a.read(0), 100);
    assert_eq!(view_b.read(0), 200);
}

#[test]
fn two_views_share_mem_see_writes() {
    let mem = create_mem(128);
    let view_a = MemZoneWriter::new(&mem, 16, 10);
    let view_b = MemZoneWriter::new(&mem, 16, 10);

    view_a.write(5, 999);
    assert_eq!(view_b.read(5), 999);
}

#[test]
fn works_with_different_slot_sizes() {
    let mem = create_mem(128);
    let view_8 = MemZoneWriter::new(&mem, 8, 0);
    let view_16 = MemZoneWriter::new(&mem, 16, 64);

    view_8.write(0, 111);
    view_16.write(0, 222);

    assert_eq!(view_8.read(0), 111);
    assert_eq!(view_16.read(0), 222);
}

#[test]
#[should_panic(expected = "MemZoneWriter::new | range")]
fn new_panics_if_out_of_bounds() {
    let mem = create_mem(10);
    let _view = MemZoneWriter::new(&mem, 16, 0);
}

#[test]
#[should_panic(expected = "MemZoneWriter::new | range")]
fn new_panics_if_start_index_crosses_bounds() {
    let mem = create_mem(32);
    let _view = MemZoneWriter::new(&mem, 16, 20);
}
