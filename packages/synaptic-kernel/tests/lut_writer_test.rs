//! Unit-style tests for [`LutWriter`] against a raw [`TripleBufferWriter`].

use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::lut_writer::LutWriter;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

const TB_BUFFER_CAPACITY: u32 = 4096;

fn make_tb(mem: &AtomicBuffer) -> TripleBufferWriter {
    TripleBufferWriter::new(Arc::clone(mem), 0, TB_BUFFER_CAPACITY)
}

#[test]
fn write_read_roundtrip() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let lut = LutWriter::new(tb.clone(), 16, 0);
    for i in 0..16 {
        lut.write(i, (i as i32) * 7 + 3);
    }
    for i in 0..16 {
        assert_eq!(lut.read(i), (i as i32) * 7 + 3);
    }
}

#[test]
fn write_all_read_all_roundtrip() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let lut = LutWriter::new(tb.clone(), 8, 0);
    let data: Vec<i32> = (0..8).map(|i| i * 11 - 100).collect();
    lut.write_all(&data);
    let mut out = [0i32; 8];
    lut.read_all(&mut out);
    assert_eq!(out, data.as_slice());
}

#[test]
fn write_all_partial_only_writes_prefix() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let lut = LutWriter::new(tb.clone(), 10, 0);
    lut.write_all(&[1, 2, 3]);
    assert_eq!(lut.read(0), 1);
    assert_eq!(lut.read(1), 2);
    assert_eq!(lut.read(2), 3);
    for i in 3..10 {
        assert_eq!(lut.read(i), 0, "index {} should remain zero-filled", i);
    }
}

#[test]
fn read_respects_tb_start_offset() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let start = 100usize;
    let lut = LutWriter::new(tb.clone(), 4, start);
    lut.write(0, 111);
    lut.write(3, 222);
    assert_eq!(lut.read(0), 111);
    assert_eq!(lut.read(3), 222);
    assert_eq!(lut.tb_start_offset(), start);
}

#[test]
fn write_does_not_corrupt_neighbors() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let a = LutWriter::new(tb.clone(), 3, 10);
    let b = LutWriter::new(tb.clone(), 3, 20);
    a.write(0, 1);
    a.write(1, 2);
    a.write(2, 3);
    b.write(0, 9);
    b.write(1, 8);
    b.write(2, 7);
    assert_eq!(a.read(0), 1);
    assert_eq!(a.read(1), 2);
    assert_eq!(a.read(2), 3);
    assert_eq!(b.read(0), 9);
    assert_eq!(b.read(1), 8);
    assert_eq!(b.read(2), 7);
}

#[test]
fn len_returns_size() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let lut = LutWriter::new(tb.clone(), 42, 0);
    assert_eq!(lut.len(), 42);
}

#[test]
fn tb_start_offset_getter() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let lut = LutWriter::new(tb.clone(), 5, 77);
    assert_eq!(lut.tb_start_offset(), 77);
}

#[test]
fn tb_end_offset_getter() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let lut = LutWriter::new(tb.clone(), 5, 77);
    assert_eq!(lut.tb_end_offset(), 77 + 5);
}

#[test]
fn calculate_size_on_tb_is_identity() {
    assert_eq!(LutWriter::calculate_size_on_tb(0), 0);
    assert_eq!(LutWriter::calculate_size_on_tb(99), 99);
}

#[test]
fn zero_filled_on_new() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let lut = LutWriter::new(tb.clone(), 32, 5);
    for i in 0..32 {
        assert_eq!(lut.read(i), 0);
    }
}

#[test]
fn bind_does_not_zero_fill() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    {
        let lut = LutWriter::new(tb.clone(), 5, 100);
        lut.write(0, 123);
        lut.write(4, 456);
    }
    let lut2 = LutWriter::bind(tb, 5, 100);
    assert_eq!(lut2.read(0), 123);
    assert_eq!(lut2.read(4), 456);
}

#[test]
fn copy_from_preserves_data() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let src = LutWriter::new(tb.clone(), 6, 0);
    let dst = LutWriter::new(tb.clone(), 6, 200);
    for i in 0..6 {
        src.write(i, 1000 + i as i32);
    }
    dst.copy_from(&src);
    for i in 0..6 {
        assert_eq!(dst.read(i), 1000 + i as i32);
    }
}

#[test]
fn copy_from_source_smaller_tail_unchanged() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let src = LutWriter::new(tb.clone(), 4, 0);
    let dst = LutWriter::new(tb.clone(), 8, 50);
    for i in 0..4 {
        src.write(i, i as i32 + 1);
    }
    for i in 0..8 {
        dst.write(i, -999);
    }
    dst.copy_from(&src);
    for i in 0..4 {
        assert_eq!(dst.read(i), i as i32 + 1);
    }
    for i in 4..8 {
        assert_eq!(dst.read(i), -999);
    }
}

#[test]
fn to_reader_after_publish_swap_sees_written_data() {
    let mem = create_mem(65536);
    let tb = make_tb(&mem);
    let lut = LutWriter::new(tb.clone(), 4, 10);
    lut.write_all(&[77, 88, 99, 100]);
    tb.publish();
    let tb_reader = tb.to_reader();
    assert!(tb_reader.swap());
    let lr = lut.to_reader();
    assert_eq!(lr.read(0), 77);
    assert_eq!(lr.read(1), 88);
    assert_eq!(lr.read(2), 99);
    assert_eq!(lr.read(3), 100);
    assert_eq!(lr.len(), 4);
    assert_eq!(lr.tb_start_offset(), 10);
    assert_eq!(lr.tb_end_offset(), 14);
}

#[cfg(debug_assertions)]
mod debug_checks {
    use super::*;

    #[test]
    #[should_panic]
    fn write_out_of_bounds_panics() {
        let mem = create_mem(65536);
        let tb = make_tb(&mem);
        let lut = LutWriter::new(tb.clone(), 3, 0);
        lut.write(3, 0);
    }

    #[test]
    #[should_panic]
    fn read_out_of_bounds_panics() {
        let mem = create_mem(65536);
        let tb = make_tb(&mem);
        let lut = LutWriter::new(tb.clone(), 3, 0);
        let _ = lut.read(3);
    }
}
