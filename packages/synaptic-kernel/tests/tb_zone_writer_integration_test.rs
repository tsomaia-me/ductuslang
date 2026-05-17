use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::tb_zone_writer::TbZoneWriter;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

// ============ Construction ============

#[test]
fn new_creates_view() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 0);
    assert_eq!(view.read(0), 0);
}

#[test]
fn new_stride_1() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 1, 0);
    assert_eq!(view.tb_start_offset(), 0);
    assert_eq!(view.tb_end_offset(), 1);
}

#[test]
fn new_stride_8() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 8, 0);
    assert_eq!(view.tb_start_offset(), 0);
    assert_eq!(view.tb_end_offset(), 8);
}

#[test]
fn new_stride_16() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 0);
    assert_eq!(view.tb_start_offset(), 0);
    assert_eq!(view.tb_end_offset(), 16);
}

#[test]
fn new_stride_256_fits_capacity_exactly() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 256, 0);
    assert_eq!(view.tb_start_offset(), 0);
    assert_eq!(view.tb_end_offset(), 256);
}

#[test]
fn new_at_nonzero_start_offset() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let view = TbZoneWriter::new(&writer, 16, 100);
    assert_eq!(view.tb_start_offset(), 100);
    assert_eq!(view.tb_end_offset(), 116);
}

#[test]
fn tb_start_offset_accessor_various_offsets() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);

    let a = TbZoneWriter::new(&writer, 8, 0);
    let b = TbZoneWriter::new(&writer, 8, 7);
    let c = TbZoneWriter::new(&writer, 8, 128);

    assert_eq!(a.tb_start_offset(), 0);
    assert_eq!(b.tb_start_offset(), 7);
    assert_eq!(c.tb_start_offset(), 128);
}

#[test]
fn tb_end_equals_start_plus_stride() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);

    let v1 = TbZoneWriter::new(&writer, 1, 64);
    assert_eq!(v1.tb_end_offset(), v1.tb_start_offset() + 1);

    let v8 = TbZoneWriter::new(&writer, 8, 64);
    assert_eq!(v8.tb_end_offset(), v8.tb_start_offset() + 8);

    let v16 = TbZoneWriter::new(&writer, 16, 64);
    assert_eq!(v16.tb_end_offset(), v16.tb_start_offset() + 16);

    let v256 = TbZoneWriter::new(&writer, 256, 0);
    assert_eq!(v256.tb_end_offset(), v256.tb_start_offset() + 256);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter::new | range")]
fn panics_if_out_of_bounds() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 16);
    let _view = TbZoneWriter::new(&writer, 16, 8);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter::new | range")]
fn panics_if_start_offset_equals_capacity() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 16);
    let _view = TbZoneWriter::new(&writer, 1, 16);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter::new | range")]
fn panics_if_end_offset_exceeds_capacity_by_one() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 32);
    let _view = TbZoneWriter::new(&writer, 16, 17);
}

// ============ Read / Write ============

#[test]
fn write_then_read_same_offset() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 0);

    view.write(0, 42);
    assert_eq!(view.read(0), 42);
}

#[test]
fn write_all_offsets_then_read_back() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 0);

    for i in 0..16 {
        view.write(i, (i as i32) * 10);
    }

    for i in 0..16 {
        assert_eq!(view.read(i), (i as i32) * 10);
    }
}

#[test]
fn multiple_writes_to_same_offset_last_wins() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 0);

    view.write(5, 1);
    view.write(5, 2);
    view.write(5, 3);
    view.write(5, 999);
    assert_eq!(view.read(5), 999);
}

#[test]
fn writes_dont_interfere_across_disjoint_struct_writers() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);

    let view_a = TbZoneWriter::new(&writer, 16, 0);
    let view_b = TbZoneWriter::new(&writer, 16, 64);

    for i in 0..16 {
        view_a.write(i, 100 + i as i32);
        view_b.write(i, 200 + i as i32);
    }

    for i in 0..16 {
        assert_eq!(view_a.read(i), 100 + i as i32);
        assert_eq!(view_b.read(i), 200 + i as i32);
    }
}

#[test]
fn reads_on_fresh_zeroed_slot_return_zero() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 0);

    for i in 0..16 {
        assert_eq!(view.read(i), 0);
    }
}

#[test]
fn write_supports_negative_values() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 0);

    view.write(0, i32::MIN);
    view.write(1, -1);
    view.write(2, i32::MAX);

    assert_eq!(view.read(0), i32::MIN);
    assert_eq!(view.read(1), -1);
    assert_eq!(view.read(2), i32::MAX);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter.read | offset")]
fn panics_on_read_offset_equal_to_stride() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 0);
    let _ = view.read(16);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter.write | offset")]
fn panics_on_write_offset_equal_to_stride() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 0);
    view.write(16, 0);
}

// ============ read_all / write_all ============

#[test]
fn write_all_then_read_all_roundtrip() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 8, 0);

    let data: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    view.write_all(&data);
    let mut out = [0i32; 8];
    view.read_all(&mut out);
    assert_eq!(out, data);
}

#[test]
fn write_all_then_individual_reads() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 8, 0);

    let data: [i32; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
    view.write_all(&data);

    for i in 0..8 {
        assert_eq!(view.read(i), data[i]);
    }
}

#[test]
fn individual_writes_then_read_all() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 8, 0);

    let expected: [i32; 8] = [11, 22, 33, 44, 55, 66, 77, 88];
    for i in 0..8 {
        view.write(i, expected[i]);
    }

    let mut out = [0i32; 8];
    view.read_all(&mut out);
    assert_eq!(out, expected);
}

#[test]
fn read_all_on_fresh_slot_returns_zeros() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 0);

    let mut out = [0i32; 16];
    view.read_all(&mut out);
    assert_eq!(out, [0i32; 16]);
}

// ============ Interaction with TripleBufferWriter ============

#[test]
fn write_via_struct_visible_to_underlying_tb_writer() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 32);

    view.write(0, 123);
    view.write(5, 456);

    assert_eq!(writer.read(32), 123);
    assert_eq!(writer.read(37), 456);
}

#[test]
fn write_via_tb_writer_visible_to_struct_writer() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let view = TbZoneWriter::new(&writer, 16, 32);

    writer.write(32, 789);
    writer.write(47, 111);

    assert_eq!(view.read(0), 789);
    assert_eq!(view.read(15), 111);
}

#[test]
fn struct_writer_changes_observable_by_reader_after_publish_and_swap() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let view = TbZoneWriter::new(&writer, 16, 32);

    view.write_all(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    writer.publish();
    assert!(reader.swap());

    for i in 0..16 {
        assert_eq!(reader.read(32 + i), (i as i32) + 1);
    }
}

#[test]
fn struct_writer_without_publish_invisible_to_reader() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let view = TbZoneWriter::new(&writer, 16, 0);

    view.write(0, 42);

    assert!(!reader.swap());
    assert_eq!(reader.read(0), 0);
}

// ============ Integration ============

#[test]
fn multiple_struct_writers_independent_regions() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);

    let a = TbZoneWriter::new(&writer, 16, 0);
    let b = TbZoneWriter::new(&writer, 16, 16);
    let c = TbZoneWriter::new(&writer, 16, 32);

    a.write_all(&[1i32; 16]);
    b.write_all(&[2i32; 16]);
    c.write_all(&[3i32; 16]);

    let mut ra = [0i32; 16];
    let mut rb = [0i32; 16];
    let mut rc = [0i32; 16];
    a.read_all(&mut ra);
    b.read_all(&mut rb);
    c.read_all(&mut rc);
    assert_eq!(ra, [1i32; 16]);
    assert_eq!(rb, [2i32; 16]);
    assert_eq!(rc, [3i32; 16]);
}

#[test]
fn multiple_struct_writers_different_strides_independent() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);

    let small = TbZoneWriter::new(&writer, 4, 0);
    let medium = TbZoneWriter::new(&writer, 16, 4);
    let large = TbZoneWriter::new(&writer, 64, 20);

    small.write_all(&[7, 8, 9, 10]);
    medium.write_all(&[100i32; 16]);
    large.write_all(&[-1i32; 64]);

    let mut sa = [0i32; 4];
    let mut me = [0i32; 16];
    let mut la = [0i32; 64];
    small.read_all(&mut sa);
    medium.read_all(&mut me);
    large.read_all(&mut la);
    assert_eq!(sa, [7, 8, 9, 10]);
    assert_eq!(me, [100i32; 16]);
    assert_eq!(la, [-1i32; 64]);
}
