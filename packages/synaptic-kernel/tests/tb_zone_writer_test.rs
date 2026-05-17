use std::sync::atomic::AtomicI32;
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
    let zone = TbZoneWriter::new(&writer, 1, 0);
    assert_eq!(zone.tb_start_offset(), 0);
    assert_eq!(zone.tb_end_offset(), 1);
}

#[test]
fn construct_stride_4_at_offset_0() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 4, 0);
    assert_eq!(zone.tb_start_offset(), 0);
    assert_eq!(zone.tb_end_offset(), 4);
}

#[test]
fn construct_stride_64_at_offset_0() {
    let mem = create_mem(2048);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let zone = TbZoneWriter::new(&writer, 64, 0);
    assert_eq!(zone.tb_start_offset(), 0);
    assert_eq!(zone.tb_end_offset(), 64);
}

// ============ Construction: non-zero offsets ============

#[test]
fn construct_at_small_nonzero_offset() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);
    let zone = TbZoneWriter::new(&writer, 16, 1);
    assert_eq!(zone.tb_start_offset(), 1);
    assert_eq!(zone.tb_end_offset(), 17);
}

#[test]
fn construct_at_large_nonzero_offset() {
    let mem = create_mem(8192);
    let writer = TripleBufferWriter::new(mem, 0, 1024);
    let zone = TbZoneWriter::new(&writer, 64, 500);
    assert_eq!(zone.tb_start_offset(), 500);
    assert_eq!(zone.tb_end_offset(), 564);
}

// ============ Construction: exact fit (tb_end_offset == buffer_capacity) ============

#[test]
fn construct_exact_fit_stride_1() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 1, 255);
    assert_eq!(zone.tb_start_offset(), 255);
    assert_eq!(zone.tb_end_offset(), 256);
}

#[test]
fn construct_exact_fit_stride_16() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 16, 240);
    assert_eq!(zone.tb_start_offset(), 240);
    assert_eq!(zone.tb_end_offset(), 256);
}

#[test]
fn construct_exact_fit_full_capacity() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 64);
    let zone = TbZoneWriter::new(&writer, 64, 0);
    assert_eq!(zone.tb_start_offset(), 0);
    assert_eq!(zone.tb_end_offset(), 64);
}

// ============ Construction: out-of-bounds panic ============

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter::new | range")]
fn panic_end_exceeds_capacity_by_one() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 32);
    let _zone = TbZoneWriter::new(&writer, 16, 17);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter::new | range")]
fn panic_start_equals_capacity_with_stride_1() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 16);
    let _zone = TbZoneWriter::new(&writer, 1, 16);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter::new | range")]
fn panic_far_out_of_bounds() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 16);
    let _zone = TbZoneWriter::new(&writer, 16, 8);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter::new | range")]
fn panic_stride_exceeds_capacity() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 32);
    let _zone = TbZoneWriter::new(&writer, 64, 0);
}

// ============ Read / write single: non-zero offset regression ============

#[test]
fn write_read_single_roundtrip_at_nonzero_offset() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 8, 64);

    for i in 0..8 {
        zone.write(i, 1000 + i as i32);
    }
    for i in 0..8 {
        assert_eq!(zone.read(i), 1000 + i as i32);
    }
}

#[test]
fn zone_write_writes_at_start_offset_plus_index_in_underlying_tb() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 8, 100);

    zone.write(0, 777);
    zone.write(7, 888);

    assert_eq!(writer.read(100), 777);
    assert_eq!(writer.read(107), 888);
}

#[test]
fn zone_read_reads_at_start_offset_plus_index_in_underlying_tb() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 8, 100);

    writer.write(100, 555);
    writer.write(107, 444);

    assert_eq!(zone.read(0), 555);
    assert_eq!(zone.read(7), 444);
}

#[test]
fn disjoint_zones_at_nonzero_offsets_are_isolated() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);

    let zone_a = TbZoneWriter::new(&writer, 8, 32);
    let zone_b = TbZoneWriter::new(&writer, 8, 40);

    for i in 0..8 {
        zone_a.write(i, 100 + i as i32);
        zone_b.write(i, 200 + i as i32);
    }

    for i in 0..8 {
        assert_eq!(zone_a.read(i), 100 + i as i32);
        assert_eq!(zone_b.read(i), 200 + i as i32);
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter.read | offset")]
fn panic_on_read_offset_equal_to_stride() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 8, 0);
    let _ = zone.read(8);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneWriter.write | offset")]
fn panic_on_write_offset_equal_to_stride() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 8, 0);
    zone.write(8, 0);
}

// ============ read_all / write_all at NON-ZERO offset (regression guard) ============

#[test]
fn write_all_then_read_all_at_offset_1() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 8, 1);

    let data: [i32; 8] = [11, 22, 33, 44, 55, 66, 77, 88];
    zone.write_all(&data);
    let mut read_back = [0i32; 8];
    zone.read_all(&mut read_back);
    assert_eq!(read_back, data);
}

#[test]
fn write_all_then_read_all_at_offset_64() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 16, 64);

    let data: [i32; 16] = [
        100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115,
    ];
    zone.write_all(&data);
    let mut read_back = [0i32; 16];
    zone.read_all(&mut read_back);
    assert_eq!(read_back, data);
}

#[test]
fn read_all_at_nonzero_offset_does_not_return_buffer_start_data() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);

    // Preload distinct values at buffer start so we can detect a "from 0" bug.
    for i in 0..8 {
        writer.write(i, -1 - i as i32);
    }

    let zone = TbZoneWriter::new(&writer, 8, 64);
    let data: [i32; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
    zone.write_all(&data);

    let mut read_back = [0i32; 8];
    zone.read_all(&mut read_back);
    assert_eq!(read_back, data);

    // Start-of-buffer is untouched by the zone at offset 64.
    for i in 0..8 {
        assert_eq!(writer.read(i), -1 - i as i32);
    }
}

#[test]
fn write_all_at_tail_offset_capacity_minus_stride() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    // tail zone: tb_start_offset + STRIDE == capacity
    let zone = TbZoneWriter::new(&writer, 16, 240);

    let data: [i32; 16] = [
        -1, -2, -3, -4, -5, -6, -7, -8, -9, -10, -11, -12, -13, -14, -15, -16,
    ];
    zone.write_all(&data);
    let mut read_back = [0i32; 16];
    zone.read_all(&mut read_back);
    assert_eq!(read_back, data);

    // Verify via underlying TripleBufferWriter on exact slots.
    for i in 0..16 {
        assert_eq!(writer.read(240 + i), data[i]);
    }
}

#[test]
fn write_all_at_nonzero_offset_matches_individual_reads() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 8, 32);

    let data: [i32; 8] = [9, 8, 7, 6, 5, 4, 3, 2];
    zone.write_all(&data);

    for i in 0..8 {
        assert_eq!(zone.read(i), data[i]);
        // And cross-check via underlying TB at absolute slot.
        assert_eq!(writer.read(32 + i), data[i]);
    }
}

#[test]
fn individual_writes_at_nonzero_offset_match_read_all() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 8, 32);

    let expected: [i32; 8] = [21, 22, 23, 24, 25, 26, 27, 28];
    for i in 0..8 {
        zone.write(i, expected[i]);
    }

    let mut read_back = [0i32; 8];
    zone.read_all(&mut read_back);
    assert_eq!(read_back, expected);
}

// ============ STRIDE edge cases ============

#[test]
fn stride_1_write_all_read_all_at_offset_0() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 1, 0);

    zone.write_all(&[42]);
    let mut buf = [0i32; 1];
    zone.read_all(&mut buf);
    assert_eq!(buf, [42]);
}

#[test]
fn stride_1_write_all_read_all_at_nonzero_offset() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);
    let zone = TbZoneWriter::new(&writer, 1, 199);

    zone.write_all(&[1234]);
    let mut buf = [0i32; 1];
    zone.read_all(&mut buf);
    assert_eq!(buf, [1234]);
    // Cross-check with underlying TB to ensure the write landed at slot 199.
    assert_eq!(writer.read(199), 1234);
}

#[test]
fn stride_1_write_all_read_all_at_tail() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 64);
    let zone = TbZoneWriter::new(&writer, 1, 63);

    zone.write_all(&[777]);
    let mut buf = [0i32; 1];
    zone.read_all(&mut buf);
    assert_eq!(buf, [777]);
}

#[test]
fn full_capacity_zone_write_all_read_all() {
    let mem = create_mem(1024);
    // buffer_capacity == STRIDE: the zone spans the entire TB buffer.
    let writer = TripleBufferWriter::new(mem, 0, 16);
    let zone = TbZoneWriter::new(&writer, 16, 0);

    let data: [i32; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    zone.write_all(&data);
    let mut read_back = [0i32; 16];
    zone.read_all(&mut read_back);
    assert_eq!(read_back, data);
}

// ============ Multi-zone isolation via read_all / write_all ============

#[test]
fn write_all_in_one_zone_does_not_affect_other_zone_read_all() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);

    let zone_a = TbZoneWriter::new(&writer, 8, 32);
    let zone_b = TbZoneWriter::new(&writer, 8, 64);

    let a_data: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let b_data: [i32; 8] = [10, 20, 30, 40, 50, 60, 70, 80];

    zone_a.write_all(&a_data);
    zone_b.write_all(&b_data);

    let mut ra = [0i32; 8];
    let mut rb = [0i32; 8];
    zone_a.read_all(&mut ra);
    zone_b.read_all(&mut rb);
    assert_eq!(ra, a_data);
    assert_eq!(rb, b_data);
}

#[test]
fn read_all_returns_only_own_region_on_multi_zone_layout() {
    let mem = create_mem(4096);
    let writer = TripleBufferWriter::new(mem, 0, 512);

    let zone_a = TbZoneWriter::new(&writer, 4, 0);
    let zone_b = TbZoneWriter::new(&writer, 4, 4);
    let zone_c = TbZoneWriter::new(&writer, 4, 8);

    zone_a.write_all(&[1, 1, 1, 1]);
    zone_b.write_all(&[2, 2, 2, 2]);
    zone_c.write_all(&[3, 3, 3, 3]);

    let mut ra = [0i32; 4];
    let mut rb = [0i32; 4];
    let mut rc = [0i32; 4];
    zone_a.read_all(&mut ra);
    zone_b.read_all(&mut rb);
    zone_c.read_all(&mut rc);
    assert_eq!(ra, [1, 1, 1, 1]);
    assert_eq!(rb, [2, 2, 2, 2]);
    assert_eq!(rc, [3, 3, 3, 3]);
}

#[test]
fn adjacent_zones_do_not_bleed_at_boundary() {
    let mem = create_mem(1024);
    let writer = TripleBufferWriter::new(mem, 0, 256);

    // Place two zones immediately adjacent: [16..24) and [24..32).
    let left = TbZoneWriter::new(&writer, 8, 16);
    let right = TbZoneWriter::new(&writer, 8, 24);

    left.write_all(&[1, 2, 3, 4, 5, 6, 7, 8]);
    right.write_all(&[100, 200, 300, 400, 500, 600, 700, 800]);

    let mut la = [0i32; 8];
    let mut ra = [0i32; 8];
    left.read_all(&mut la);
    right.read_all(&mut ra);
    assert_eq!(la, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(ra, [100, 200, 300, 400, 500, 600, 700, 800]);

    // Writing the right zone must not reach back into the left zone's last slot.
    assert_eq!(left.read(7), 8);
    // And writing the left zone must not spill into the right zone's first slot.
    assert_eq!(right.read(0), 100);
}
