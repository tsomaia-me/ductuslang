use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::tb_zone_reader::TbZoneReader;
use synaptic_kernel::primitives::tb_zone_writer::TbZoneWriter;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

// ============ Construction: STRIDE coverage ============

#[test]
fn construct_stride_1_at_offset_0() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let zone = TbZoneReader::new(&reader, 1, 0);
    assert_eq!(zone.tb_start_offset(), 0);
    assert_eq!(zone.tb_end_offset(), 1);
}

#[test]
fn construct_stride_4_at_offset_0() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let zone = TbZoneReader::new(&reader, 4, 0);
    assert_eq!(zone.tb_start_offset(), 0);
    assert_eq!(zone.tb_end_offset(), 4);
}

#[test]
fn construct_stride_64_at_offset_0() {
    let mem = create_mem(2048);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let reader = writer.to_reader();
    let zone = TbZoneReader::new(&reader, 64, 0);
    assert_eq!(zone.tb_start_offset(), 0);
    assert_eq!(zone.tb_end_offset(), 64);
}

// ============ Construction: non-zero offsets ============

#[test]
fn construct_at_small_nonzero_offset() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let reader = writer.to_reader();
    let zone = TbZoneReader::new(&reader, 16, 1);
    assert_eq!(zone.tb_start_offset(), 1);
    assert_eq!(zone.tb_end_offset(), 17);
}

#[test]
fn construct_at_large_nonzero_offset() {
    let mem = create_mem(8192);
    let writer = TripleBufferWriter::new(mem, 0, 1024);
    let reader = writer.to_reader();
    let zone = TbZoneReader::new(&reader, 64, 500);
    assert_eq!(zone.tb_start_offset(), 500);
    assert_eq!(zone.tb_end_offset(), 564);
}

// ============ Construction: exact fit ============

#[test]
fn construct_exact_fit_stride_1_at_tail() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let zone = TbZoneReader::new(&reader, 1, 255);
    assert_eq!(zone.tb_start_offset(), 255);
    assert_eq!(zone.tb_end_offset(), 256);
}

#[test]
fn construct_exact_fit_stride_16_at_tail() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let zone = TbZoneReader::new(&reader, 16, 240);
    assert_eq!(zone.tb_start_offset(), 240);
    assert_eq!(zone.tb_end_offset(), 256);
}

#[test]
fn construct_exact_fit_full_capacity() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 64);
    let reader = writer.to_reader();
    let zone = TbZoneReader::new(&reader, 64, 0);
    assert_eq!(zone.tb_start_offset(), 0);
    assert_eq!(zone.tb_end_offset(), 64);
}

// ============ Construction: out-of-bounds panic ============

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneReader::new | range")]
fn panic_end_exceeds_capacity_by_one() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 32);
    let reader = writer.to_reader();
    let _zone = TbZoneReader::new(&reader, 16, 17);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneReader::new | range")]
fn panic_start_equals_capacity_with_stride_1() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 16);
    let reader = writer.to_reader();
    let _zone = TbZoneReader::new(&reader, 1, 16);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneReader::new | range")]
fn panic_far_out_of_bounds() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 16);
    let reader = writer.to_reader();
    let _zone = TbZoneReader::new(&reader, 16, 8);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneReader::new | range")]
fn panic_stride_exceeds_capacity() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 32);
    let reader = writer.to_reader();
    let _zone = TbZoneReader::new(&reader, 64, 0);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneReader.read | offset")]
fn panic_on_read_offset_equal_to_stride() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let zone = TbZoneReader::new(&reader, 8, 0);
    let _ = zone.read(8);
}

// ============ Reader read single: after publish/swap ============

#[test]
fn read_returns_published_value_at_offset_0() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    writer.write(0, 42);
    writer.publish();
    assert!(reader.swap());

    let zone = TbZoneReader::new(&reader, 4, 0);
    assert_eq!(zone.read(0), 42);
}

#[test]
fn read_returns_published_value_at_nonzero_offset() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    for i in 0..8 {
        writer.write(100 + i, (i as i32) + 1);
    }
    writer.publish();
    assert!(reader.swap());

    let zone = TbZoneReader::new(&reader, 8, 100);
    for i in 0..8 {
        assert_eq!(zone.read(i), (i as i32) + 1);
    }
}

#[test]
fn read_on_fresh_reader_returns_zero() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();
    let zone = TbZoneReader::new(&reader, 8, 32);

    for i in 0..8 {
        assert_eq!(zone.read(i), 0);
    }
}

// ============ Reader read_all ============

#[test]
fn read_all_returns_full_array_at_offset_0() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    for i in 0..16 {
        writer.write(i, (i as i32) * 10);
    }
    writer.publish();
    assert!(reader.swap());

    let zone = TbZoneReader::new(&reader, 16, 0);
    let expected: [i32; 16] = [
        0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150,
    ];
    let mut out = [0i32; 16];
    zone.read_all(&mut out);
    assert_eq!(out, expected);
}

#[test]
fn read_all_at_nonzero_offset_returns_zone_data_not_buffer_start() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    // Preload buffer start with distinct values so a "from 0" bug is detectable.
    for i in 0..8 {
        writer.write(i, -100 - i as i32);
    }
    // Publish zone data at offset 64.
    for i in 0..8 {
        writer.write(64 + i, 500 + i as i32);
    }
    writer.publish();
    assert!(reader.swap());

    let zone = TbZoneReader::new(&reader, 8, 64);
    let mut out = [0i32; 8];
    zone.read_all(&mut out);
    assert_eq!(out, [500, 501, 502, 503, 504, 505, 506, 507]);

    // Reader zone at offset 0 must see the buffer-start values, not the offset-64 values.
    let zone_head = TbZoneReader::new(&reader, 8, 0);
    let mut head = [0i32; 8];
    zone_head.read_all(&mut head);
    assert_eq!(head, [-100, -101, -102, -103, -104, -105, -106, -107]);
}

#[test]
fn read_all_at_tail_offset_capacity_minus_stride() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    for i in 0..16 {
        writer.write(240 + i, (i as i32) + 1);
    }
    writer.publish();
    assert!(reader.swap());

    let zone = TbZoneReader::new(&reader, 16, 240);
    let expected: [i32; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut out = [0i32; 16];
    zone.read_all(&mut out);
    assert_eq!(out, expected);
}

#[test]
fn read_all_stride_1() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    writer.write(200, 999);
    writer.publish();
    assert!(reader.swap());

    let zone = TbZoneReader::new(&reader, 1, 200);
    let mut buf = [0i32; 1];
    zone.read_all(&mut buf);
    assert_eq!(buf, [999]);
}

#[test]
fn read_all_full_capacity_zone() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 8);
    let reader = writer.to_reader();

    for i in 0..8 {
        writer.write(i, (i as i32) + 100);
    }
    writer.publish();
    assert!(reader.swap());

    let zone = TbZoneReader::new(&reader, 8, 0);
    let mut out = [0i32; 8];
    zone.read_all(&mut out);
    assert_eq!(out, [100, 101, 102, 103, 104, 105, 106, 107]);
}

#[test]
fn multiple_zones_at_distinct_offsets_each_return_own_region() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let reader = writer.to_reader();

    for i in 0..8 {
        writer.write(0 + i, 10 + i as i32);
        writer.write(64 + i, 100 + i as i32);
        writer.write(200 + i, 1000 + i as i32);
    }
    writer.publish();
    assert!(reader.swap());

    let a = TbZoneReader::new(&reader, 8, 0);
    let b = TbZoneReader::new(&reader, 8, 64);
    let c = TbZoneReader::new(&reader, 8, 200);

    let mut ra = [0i32; 8];
    let mut rb = [0i32; 8];
    let mut rc = [0i32; 8];
    a.read_all(&mut ra);
    b.read_all(&mut rb);
    c.read_all(&mut rc);
    assert_eq!(ra, [10, 11, 12, 13, 14, 15, 16, 17]);
    assert_eq!(rb, [100, 101, 102, 103, 104, 105, 106, 107]);
    assert_eq!(rc, [1000, 1001, 1002, 1003, 1004, 1005, 1006, 1007]);
}

// ============ Writer-Reader interaction via zones ============

#[test]
fn zone_writer_to_zone_reader_roundtrip_at_offset_0() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    let zw = TbZoneWriter::new(&writer, 8, 0);
    let zr = TbZoneReader::new(&reader, 8, 0);

    let data: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    zw.write_all(&data);
    writer.publish();
    assert!(reader.swap());

    let mut out = [0i32; 8];
    zr.read_all(&mut out);
    assert_eq!(out, data);
}

#[test]
fn zone_writer_to_zone_reader_roundtrip_at_nonzero_offset() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    let zw = TbZoneWriter::new(&writer, 8, 64);
    let zr = TbZoneReader::new(&reader, 8, 64);

    let data: [i32; 8] = [100, 200, 300, 400, 500, 600, 700, 800];
    zw.write_all(&data);
    writer.publish();
    assert!(reader.swap());

    let mut out = [0i32; 8];
    zr.read_all(&mut out);
    assert_eq!(out, data);

    // Reader zone at offset 0 must NOT see this data.
    let zr_head = TbZoneReader::new(&reader, 8, 0);
    let mut head = [0i32; 8];
    zr_head.read_all(&mut head);
    assert_eq!(head, [0i32; 8]);
}

#[test]
fn zone_writer_to_zone_reader_roundtrip_at_tail_offset() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    let zw = TbZoneWriter::new(&writer, 16, 240);
    let zr = TbZoneReader::new(&reader, 16, 240);

    let data: [i32; 16] = [
        -1, -2, -3, -4, -5, -6, -7, -8, -9, -10, -11, -12, -13, -14, -15, -16,
    ];
    zw.write_all(&data);
    writer.publish();
    assert!(reader.swap());

    let mut out = [0i32; 16];
    zr.read_all(&mut out);
    assert_eq!(out, data);
}

#[test]
fn roundtrip_stride_1() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    let zw = TbZoneWriter::new(&writer, 1, 128);
    let zr = TbZoneReader::new(&reader, 1, 128);

    zw.write_all(&[31337]);
    writer.publish();
    assert!(reader.swap());

    let mut buf = [0i32; 1];
    zr.read_all(&mut buf);
    assert_eq!(buf, [31337]);
    assert_eq!(zr.read(0), 31337);
}

#[test]
fn multiple_publish_swap_cycles_preserve_data_at_nonzero_offset() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    let zw = TbZoneWriter::new(&writer, 4, 32);
    let zr = TbZoneReader::new(&reader, 4, 32);

    zw.write_all(&[1, 2, 3, 4]);
    writer.publish();
    assert!(reader.swap());
    let mut out = [0i32; 4];
    zr.read_all(&mut out);
    assert_eq!(out, [1, 2, 3, 4]);

    zw.write_all(&[10, 20, 30, 40]);
    writer.publish();
    assert!(reader.swap());
    zr.read_all(&mut out);
    assert_eq!(out, [10, 20, 30, 40]);

    zw.write_all(&[100, 200, 300, 400]);
    writer.publish();
    assert!(reader.swap());
    zr.read_all(&mut out);
    assert_eq!(out, [100, 200, 300, 400]);
}

#[test]
fn reader_sees_prior_value_when_writer_has_not_published() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let reader = writer.to_reader();

    let zw = TbZoneWriter::new(&writer, 4, 0);
    let zr = TbZoneReader::new(&reader, 4, 0);

    // Seed a first published value.
    zw.write_all(&[1, 2, 3, 4]);
    writer.publish();
    assert!(reader.swap());
    let mut out = [0i32; 4];
    zr.read_all(&mut out);
    assert_eq!(out, [1, 2, 3, 4]);

    // Write new data but do NOT publish.
    zw.write_all(&[999, 999, 999, 999]);

    // Reader still sees the prior published value.
    zr.read_all(&mut out);
    assert_eq!(out, [1, 2, 3, 4]);
    // And swap() returns false since NEW_DATA is not set.
    assert!(!reader.swap());
    zr.read_all(&mut out);
    assert_eq!(out, [1, 2, 3, 4]);
}

#[test]
fn multiple_zones_writer_and_reader_independent_channels() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let reader = writer.to_reader();

    let zw_a = TbZoneWriter::new(&writer, 4, 0);
    let zw_b = TbZoneWriter::new(&writer, 4, 64);
    let zw_c = TbZoneWriter::new(&writer, 4, 200);

    zw_a.write_all(&[1, 1, 1, 1]);
    zw_b.write_all(&[2, 2, 2, 2]);
    zw_c.write_all(&[3, 3, 3, 3]);
    writer.publish();
    assert!(reader.swap());

    let zr_a = TbZoneReader::new(&reader, 4, 0);
    let zr_b = TbZoneReader::new(&reader, 4, 64);
    let zr_c = TbZoneReader::new(&reader, 4, 200);

    let mut ra = [0i32; 4];
    let mut rb = [0i32; 4];
    let mut rc = [0i32; 4];
    zr_a.read_all(&mut ra);
    zr_b.read_all(&mut rb);
    zr_c.read_all(&mut rc);
    assert_eq!(ra, [1, 1, 1, 1]);
    assert_eq!(rb, [2, 2, 2, 2]);
    assert_eq!(rc, [3, 3, 3, 3]);
}
