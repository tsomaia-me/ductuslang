use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::tb_zone_reader::TbZoneReader;
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
    let reader = writer.to_reader();
    let view = TbZoneReader::new(&reader, 16, 0);
    assert_eq!(view.read(0), 0);
}

#[test]
fn read_returns_zero_on_fresh_mem() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let view = TbZoneReader::new(&reader, 16, 0);

    for i in 0..16 {
        assert_eq!(view.read(i), 0);
    }
}

#[test]
fn read_at_nonzero_offset() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    writer.write(32, 777);
    writer.publish();
    reader.swap();

    let view = TbZoneReader::new(&reader, 16, 32);
    assert_eq!(view.read(0), 777);
}

#[test]
fn reads_are_isolated_between_slots() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    writer.write(0, 100);
    writer.write(16, 200);
    writer.publish();
    reader.swap();

    let view_a = TbZoneReader::new(&reader, 16, 0);
    let view_b = TbZoneReader::new(&reader, 16, 16);

    assert_eq!(view_a.read(0), 100);
    assert_eq!(view_b.read(0), 200);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneReader::new | range")]
fn panics_if_out_of_bounds() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 16);
    let reader = writer.to_reader();
    let _view = TbZoneReader::new(&reader, 16, 8);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneReader::new | range")]
fn panics_if_start_offset_equals_capacity() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 16);
    let reader = writer.to_reader();
    let _view = TbZoneReader::new(&reader, 1, 16);
}

// ============ Accessors / strides ============

#[test]
fn tb_start_and_end_offsets_stride_1() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let view = TbZoneReader::new(&reader, 1, 50);
    assert_eq!(view.tb_start_offset(), 50);
    assert_eq!(view.tb_end_offset(), 51);
}

#[test]
fn tb_start_and_end_offsets_stride_16() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let view = TbZoneReader::new(&reader, 16, 64);
    assert_eq!(view.tb_start_offset(), 64);
    assert_eq!(view.tb_end_offset(), 80);
}

#[test]
fn tb_start_and_end_offsets_stride_256() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let reader = writer.to_reader();
    let view = TbZoneReader::new(&reader, 256, 0);
    assert_eq!(view.tb_start_offset(), 0);
    assert_eq!(view.tb_end_offset(), 256);
}

#[test]
fn tb_end_equals_start_plus_stride_varied() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let reader = writer.to_reader();

    let v1 = TbZoneReader::new(&reader, 1, 3);
    let v8 = TbZoneReader::new(&reader, 8, 16);
    let v256 = TbZoneReader::new(&reader, 256, 100);

    assert_eq!(v1.tb_end_offset(), v1.tb_start_offset() + 1);
    assert_eq!(v8.tb_end_offset(), v8.tb_start_offset() + 8);
    assert_eq!(v256.tb_end_offset(), v256.tb_start_offset() + 256);
}

// ============ read_all ============

#[test]
fn read_all_on_fresh_reader_returns_zeros() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let view = TbZoneReader::new(&reader, 16, 0);

    let mut out = [0i32; 16];
    view.read_all(&mut out);
    assert_eq!(out, [0i32; 16]);
}

#[test]
fn read_all_returns_full_array_after_publish_and_swap() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    for i in 0..16 {
        writer.write(i, (i as i32) + 1);
    }
    writer.publish();
    assert!(reader.swap());

    let view = TbZoneReader::new(&reader, 16, 0);
    let expected: [i32; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut out = [0i32; 16];
    view.read_all(&mut out);
    assert_eq!(out, expected);
}

#[test]
fn read_all_at_nonzero_offset() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    for i in 0..8 {
        writer.write(64 + i, 1000 + i as i32);
    }
    writer.publish();
    reader.swap();

    let view = TbZoneReader::new(&reader, 8, 64);
    let mut out = [0i32; 8];
    view.read_all(&mut out);
    assert_eq!(out, [1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007]);
}

// ============ Consumer-side semantics ============

#[test]
fn read_reflects_latest_published_value() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    writer.write(0, 10);
    writer.publish();
    assert!(reader.swap());

    let view = TbZoneReader::new(&reader, 4, 0);
    assert_eq!(view.read(0), 10);
}

#[test]
fn read_stays_on_previous_buffer_when_writer_does_not_publish() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    writer.write(0, 111);
    writer.publish();
    assert!(reader.swap());

    let view = TbZoneReader::new(&reader, 4, 0);
    assert_eq!(view.read(0), 111);

    writer.write(0, 222);

    assert_eq!(view.read(0), 111);
}

#[test]
fn read_picks_up_new_value_after_second_publish_and_swap() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let view = TbZoneReader::new(&reader, 4, 0);

    writer.write(0, 1);
    writer.publish();
    assert!(reader.swap());
    assert_eq!(view.read(0), 1);

    writer.write(0, 2);
    writer.publish();
    assert!(reader.swap());
    assert_eq!(view.read(0), 2);
}

// ============ Isolation ============

#[test]
fn multiple_struct_readers_at_disjoint_offsets_no_cross_contamination() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let reader = writer.to_reader();

    for i in 0..16 {
        writer.write(i, 100 + i as i32);
        writer.write(64 + i, 200 + i as i32);
        writer.write(128 + i, 300 + i as i32);
    }
    writer.publish();
    assert!(reader.swap());

    let a = TbZoneReader::new(&reader, 16, 0);
    let b = TbZoneReader::new(&reader, 16, 64);
    let c = TbZoneReader::new(&reader, 16, 128);

    for i in 0..16 {
        assert_eq!(a.read(i), 100 + i as i32);
        assert_eq!(b.read(i), 200 + i as i32);
        assert_eq!(c.read(i), 300 + i as i32);
    }
}

#[test]
fn multiple_struct_readers_different_strides() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let reader = writer.to_reader();

    writer.write(0, 7);
    writer.write(1, 8);
    writer.write(2, 9);
    writer.write(3, 10);
    for i in 0..16 {
        writer.write(4 + i, 100);
    }
    for i in 0..64 {
        writer.write(20 + i, -1);
    }
    writer.publish();
    assert!(reader.swap());

    let small = TbZoneReader::new(&reader, 4, 0);
    let medium = TbZoneReader::new(&reader, 16, 4);
    let large = TbZoneReader::new(&reader, 64, 20);

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

// ============ Out-of-bounds read ============

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneReader.read | offset")]
fn panics_on_read_offset_equal_to_stride() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let view = TbZoneReader::new(&reader, 16, 0);
    let _ = view.read(16);
}
