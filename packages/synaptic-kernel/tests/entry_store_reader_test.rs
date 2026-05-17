use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use synaptic_kernel::primitives::entry_store_config::EntryStoreConfig;
use synaptic_kernel::primitives::entry_store_reader::EntryStoreReader;
use synaptic_kernel::primitives::entry_store_writer::EntryStoreWriter;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::slot_allocator::SlotAllocator;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

const TB_BUFFER_CAPACITY: u32 = 1024;
const TB_MEM_RESERVED: usize = 4 + TB_BUFFER_CAPACITY as usize * 3;
const DEFAULT_MEM_START_OFFSET: usize = TB_MEM_RESERVED + 8;
const MEM_SIZE: usize = 16384;

fn make_tb(mem: &AtomicBuffer) -> TripleBufferWriter {
    TripleBufferWriter::new(Arc::clone(mem), 0, TB_BUFFER_CAPACITY)
}

fn slot(value: u32) -> SlotId {
    SlotId::new(value).unwrap()
}

// ============ Construction via writer.to_reader() ============

#[test]
fn to_reader_produces_matching_reader() {
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let reader = store.to_reader();
    assert_eq!(reader.capacity(), store.capacity());
    assert_eq!(reader.mem_start_offset(), store.mem_start_offset());
    assert_eq!(reader.mem_end_offset(), store.mem_end_offset());
    assert_eq!(reader.tb_start_offset(), store.tb_start_offset());
    assert_eq!(reader.tb_end_offset(), store.tb_end_offset());
}

#[test]
fn calculate_size_matches_writer() {
    assert_eq!(
        EntryStoreReader::calculate_size_on_mem(&EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 16
        }),
        EntryStoreWriter::calculate_size_on_mem(&EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 16
        }),
    );
    assert_eq!(
        EntryStoreReader::calculate_size_on_tb(&EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 16
        }),
        EntryStoreWriter::calculate_size_on_tb(&EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 16
        }),
    );
    assert_eq!(
        EntryStoreReader::calculate_size_on_mem(&EntryStoreConfig {
            core_stride: 4,
            meta_stride: 0,
            attr_stride: 32,
            capacity: 64
        }),
        EntryStoreWriter::calculate_size_on_mem(&EntryStoreConfig {
            core_stride: 4,
            meta_stride: 0,
            attr_stride: 32,
            capacity: 64
        }),
    );
    assert_eq!(
        EntryStoreReader::calculate_size_on_tb(&EntryStoreConfig {
            core_stride: 4,
            meta_stride: 0,
            attr_stride: 32,
            capacity: 64
        }),
        EntryStoreWriter::calculate_size_on_tb(&EntryStoreConfig {
            core_stride: 4,
            meta_stride: 0,
            attr_stride: 32,
            capacity: 64
        }),
    );
}

// ============ Struct plane read (requires TB publish + swap) ============

#[test]
fn struct_read_after_publish_and_swap() {
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    // External TripleBufferReader: EntryStoreReader has no swap() method,
    // but the reader buffer id lives in shared mem, so an external swap()
    // advances the shared reader buffer that EntryStoreReader reads from.
    let tb_reader = tb.to_reader();
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let s = store.insert().unwrap();
    let data = [11, 22, 33, 44, 55, 66, 77, 88];
    store.get(s).core_write_all(&data);

    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();
    let mut cr = [0i32; 8];
    reader.get(s).core_read_all(&mut cr);
    assert_eq!(cr, data);
    for (i, expected) in data.iter().enumerate() {
        assert_eq!(reader.get(s).core_read(i), *expected);
    }
}

#[test]
fn struct_reads_isolated_per_slot_on_reader_side() {
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let tb_reader = tb.to_reader();
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let s1 = store.insert().unwrap();
    let s2 = store.insert().unwrap();
    store.get(s1).core_write_all(&[1, 1, 1, 1, 1, 1, 1, 1]);
    store.get(s2).core_write_all(&[2, 2, 2, 2, 2, 2, 2, 2]);

    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();
    let mut b1 = [0i32; 8];
    let mut b2 = [0i32; 8];
    reader.get(s1).core_read_all(&mut b1);
    reader.get(s2).core_read_all(&mut b2);
    assert_eq!(b1, [1; 8]);
    assert_eq!(b2, [2; 8]);
}

#[test]
fn struct_reader_handle_from_get_struct() {
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let tb_reader = tb.to_reader();
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let s = store.insert().unwrap();
    store
        .get(s)
        .core_write_all(&[100, 200, 300, 400, 500, 600, 700, 800]);
    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();
    let handle = reader.get(s);
    assert_eq!(handle.core_read(0), 100);
    assert_eq!(handle.core_read(3), 400);
    assert_eq!(handle.core_read(7), 800);
    let mut h = [0i32; 8];
    handle.core_read_all(&mut h);
    assert_eq!(h, [100, 200, 300, 400, 500, 600, 700, 800]);
}

// ============ Attribute plane read (instantly visible) ============

#[test]
fn attr_read_visible_without_publish() {
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let s = store.insert().unwrap();
    store.get(s).attr_write(0, 1234);
    store.get(s).attr_write(15, -42);

    let reader = store.to_reader();
    // No publish, no swap — mem plane writes are immediately visible.
    assert_eq!(reader.get(s).attr_read(0), 1234);
    assert_eq!(reader.get(s).attr_read(15), -42);
}

#[test]
fn attr_read_all_visible_without_publish() {
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let s = store.insert().unwrap();
    let mut data: [i32; 16] = [0; 16];
    for i in 0..16 {
        data[i] = (i as i32) * 13 - 7;
    }
    store.get(s).attr_write_all(&data);

    let reader = store.to_reader();
    let mut ar = [0i32; 16];
    reader.get(s).attr_read_all(&mut ar);
    assert_eq!(ar, data);
}

// ============ Multiple readers share state ============

#[test]
fn multiple_readers_share_underlying_state() {
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let tb_reader = tb.to_reader();
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let s = store.insert().unwrap();
    store.get(s).attr_write(0, 999);
    store.get(s).core_write(0, 7);
    tb.publish();
    assert!(tb_reader.swap());

    let reader_a = store.to_reader();
    let reader_b = store.to_reader();

    // Both readers observe the same published struct and attr data.
    assert_eq!(reader_a.get(s).attr_read(0), 999);
    assert_eq!(reader_b.get(s).attr_read(0), 999);
    assert_eq!(reader_a.get(s).core_read(0), 7);
    assert_eq!(reader_b.get(s).core_read(0), 7);

    // A subsequent writer-side attr update (mem plane) is visible to both.
    store.get(s).attr_write(0, -1);
    assert_eq!(reader_a.get(s).attr_read(0), -1);
    assert_eq!(reader_b.get(s).attr_read(0), -1);
}

// ============ Cross-configuration combinations ============

#[test]
fn reader_roundtrip_with_1_1_1_config() {
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let tb_reader = tb.to_reader();
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 1,
            meta_stride: 0,
            attr_stride: 1,
            capacity: 1,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let s = store.insert().unwrap();
    store.get(s).core_write(0, 5);
    store.get(s).attr_write(0, 6);
    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();
    assert_eq!(reader.get(s).core_read(0), 5);
    assert_eq!(reader.get(s).attr_read(0), 6);
}

#[test]
fn reader_offsets_for_nonzero_start_offsets() {
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let mem_start = DEFAULT_MEM_START_OFFSET + 128;
    let tb_start = 64;
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
        mem_start,
        tb_start,
    );
    let reader = store.to_reader();

    assert_eq!(reader.mem_start_offset(), mem_start);
    assert_eq!(reader.tb_start_offset(), tb_start);
    assert_eq!(
        reader.tb_end_offset() - reader.tb_start_offset(),
        EntryStoreReader::calculate_size_on_tb(&EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4
        })
    );
    assert_eq!(
        reader.mem_end_offset() - reader.mem_start_offset(),
        EntryStoreReader::calculate_size_on_mem(&EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4
        })
    );
}

// ============ Cross-layer layout verification ============
//
// These tests verify that the reader resolves struct-plane and attribute-plane
// offsets to the same absolute memory locations predicted by the documented
// layout formulas. One side of each assertion uses the EntryStoreReader API;
// the other side reads from the raw TripleBufferReader / raw AtomicBuffer at
// externally-computed absolute offsets. A symmetric offset bug in the
// resolution logic would fail these cross-checks even if the writer/reader
// round-trip via the abstraction keeps passing.

#[test]
fn reader_struct_read_sees_value_written_via_tb_at_expected_offset() {
    const S: usize = 8;
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let tb_reader = tb.to_reader();
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: S,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let _s1 = store.insert().unwrap();
    let slot_id = store.insert().unwrap();
    assert_eq!(slot_id, slot(2));

    let expected_abs = (slot_id.to_usize() - 1) * S + 3; // tb_start_offset=0
    store.get(slot_id).core_write(3, 4242);

    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();
    // EntryStoreReader API resolves the offset...
    assert_eq!(reader.get(slot_id).core_read(3), 4242);
    // ...and the raw TripleBufferReader at the externally-computed absolute
    // offset sees the same value. If either side miscomputes the offset, these
    // two reads disagree.
    assert_eq!(tb_reader.read(expected_abs), 4242);
}

#[test]
fn reader_struct_reads_distinct_slots_at_distinct_tb_offsets() {
    const S: usize = 8;
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let tb_reader = tb.to_reader();
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: S,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 4,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let s1 = store.insert().unwrap();
    let s2 = store.insert().unwrap();
    let s3 = store.insert().unwrap();
    assert_eq!((s1, s2, s3), (slot(1), slot(2), slot(3)));

    store.get(s1).core_write(0, 91);
    store.get(s2).core_write(0, 92);
    store.get(s3).core_write(0, 93);

    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();

    // Reader API resolves slot -> field-0 as (slot - 1) * S + 0.
    assert_eq!(reader.get(s1).core_read(0), 91);
    assert_eq!(reader.get(s2).core_read(0), 92);
    assert_eq!(reader.get(s3).core_read(0), 93);

    // Raw TripleBufferReader reads at the externally-computed absolute offsets.
    assert_eq!(tb_reader.read(0 * S), 91);
    assert_eq!(tb_reader.read(1 * S), 92);
    assert_eq!(tb_reader.read(2 * S), 93);
}

#[test]
fn reader_attr_read_sees_value_written_at_expected_mem_offset() {
    const A: usize = 16;
    const CAP: u32 = 4;
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: A,
            capacity: CAP,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let _s1 = store.insert().unwrap();
    let slot_id = store.insert().unwrap();
    assert_eq!(slot_id, slot(2));

    store.get(slot_id).attr_write(5, 7777);

    let attr_base = DEFAULT_MEM_START_OFFSET + SlotAllocator::calculate_size_on_mem(CAP as usize);
    let expected_abs = attr_base + (slot_id.to_usize() - 1) * A + 5;

    let reader = store.to_reader();
    // Reader API resolves slot -> field-5.
    assert_eq!(reader.get(slot_id).attr_read(5), 7777);
    // Raw mem at the externally-computed absolute offset must agree.
    assert_eq!(mem[expected_abs].load(Ordering::Relaxed), 7777);
}

#[test]
fn reader_attr_reads_distinct_slots_at_distinct_mem_offsets() {
    const A: usize = 16;
    const CAP: u32 = 4;
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: A,
            capacity: CAP,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );

    let s1 = store.insert().unwrap();
    let s2 = store.insert().unwrap();
    let s3 = store.insert().unwrap();
    assert_eq!((s1, s2, s3), (slot(1), slot(2), slot(3)));

    store.get(s1).attr_write(0, 501);
    store.get(s2).attr_write(0, 502);
    store.get(s3).attr_write(0, 503);

    let attr_base = DEFAULT_MEM_START_OFFSET + SlotAllocator::calculate_size_on_mem(CAP as usize);
    let reader = store.to_reader();

    assert_eq!(reader.get(s1).attr_read(0), 501);
    assert_eq!(reader.get(s2).attr_read(0), 502);
    assert_eq!(reader.get(s3).attr_read(0), 503);

    // Raw mem at externally-computed absolute offsets must agree slot-for-slot.
    assert_eq!(mem[attr_base + 0 * A].load(Ordering::Relaxed), 501);
    assert_eq!(mem[attr_base + 1 * A].load(Ordering::Relaxed), 502);
    assert_eq!(mem[attr_base + 2 * A].load(Ordering::Relaxed), 503);
}

#[test]
fn reader_layout_sizes_match_writer_layout() {
    const S: usize = 8;
    const A: usize = 16;
    const CAP: u32 = 4;
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride: S,
            meta_stride: 0,
            attr_stride: A,
            capacity: CAP,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );
    let reader = store.to_reader();

    // Mem plane span as predicted by the layout formula: allocator + attr plane.
    assert_eq!(
        reader.mem_end_offset() - reader.mem_start_offset(),
        SlotAllocator::calculate_size_on_mem(CAP as usize) + CAP as usize * A,
    );

    // TB plane span as predicted by the layout formula: capacity * STRUCT_STRIDE.
    assert_eq!(
        reader.tb_end_offset() - reader.tb_start_offset(),
        CAP as usize * S,
    );
}

// ============ META_STRIDE > 0 ============
//
// Symmetric section to the writer's META_STRIDE > 0 block. Same layout
// invariant under test: per-slot TB layout is `[core | meta]` interleaved.
// For slot k (1-based):
//   struct_start = tb_start_offset + (k - 1) * (CORE_STRIDE + META_STRIDE)
//   core zone   = [struct_start, struct_start + CORE_STRIDE)
//   meta zone   = [struct_start + CORE_STRIDE, struct_start + CORE_STRIDE + META_STRIDE)

/// Local helper for the META_STRIDE > 0 section. The existing reader tests
/// construct stores inline; this helper keeps the new tests compact.
fn make_store_cma(
    core_stride: usize,
    meta_stride: usize,
    attr_stride: usize,
    capacity: u32,
) -> (AtomicBuffer, TripleBufferWriter, EntryStoreWriter) {
    let mem = create_mem(MEM_SIZE);
    let tb = make_tb(&mem);
    let store = EntryStoreWriter::new(
        Arc::clone(&mem),
        tb.clone(),
        EntryStoreConfig {
            core_stride,
            meta_stride,
            attr_stride,
            capacity,
        },
        DEFAULT_MEM_START_OFFSET,
        0,
    );
    (mem, tb, store)
}

// ---- Construction + size ----

#[test]
fn meta_reader_calculate_size_on_tb_is_capacity_times_core_plus_meta() {
    assert_eq!(
        EntryStoreReader::calculate_size_on_tb(&EntryStoreConfig {
            core_stride: 4,
            meta_stride: 4,
            attr_stride: 16,
            capacity: 4
        }),
        4 * (4 + 4)
    );
    assert_eq!(
        EntryStoreReader::calculate_size_on_tb(&EntryStoreConfig {
            core_stride: 8,
            meta_stride: 16,
            attr_stride: 16,
            capacity: 4
        }),
        4 * (8 + 16)
    );
    assert_eq!(
        EntryStoreReader::calculate_size_on_tb(&EntryStoreConfig {
            core_stride: 1,
            meta_stride: 1,
            attr_stride: 1,
            capacity: 1
        }),
        1 * (1 + 1)
    );
    assert_eq!(
        EntryStoreReader::calculate_size_on_tb(&EntryStoreConfig {
            core_stride: 16,
            meta_stride: 0,
            attr_stride: 8,
            capacity: 256
        }),
        256 * (16 + 0)
    );
    assert_eq!(
        EntryStoreReader::calculate_size_on_tb(&EntryStoreConfig {
            core_stride: 64,
            meta_stride: 64,
            attr_stride: 16,
            capacity: 32
        }),
        32 * (64 + 64)
    );

    // Reader and writer formulas must agree across several combinations.
    for cap in [1u32, 4, 16, 32] {
        assert_eq!(
            EntryStoreReader::calculate_size_on_tb(&EntryStoreConfig {
                core_stride: 4,
                meta_stride: 4,
                attr_stride: 16,
                capacity: cap
            }),
            EntryStoreWriter::calculate_size_on_tb(&EntryStoreConfig {
                core_stride: 4,
                meta_stride: 4,
                attr_stride: 16,
                capacity: cap
            }),
        );
        assert_eq!(
            EntryStoreReader::calculate_size_on_tb(&EntryStoreConfig {
                core_stride: 8,
                meta_stride: 16,
                attr_stride: 16,
                capacity: cap
            }),
            EntryStoreWriter::calculate_size_on_tb(&EntryStoreConfig {
                core_stride: 8,
                meta_stride: 16,
                attr_stride: 16,
                capacity: cap
            }),
        );
    }
}

#[test]
fn meta_reader_calculate_size_on_mem_is_independent_of_core_and_meta() {
    let base = EntryStoreReader::calculate_size_on_mem(&EntryStoreConfig {
        core_stride: 0,
        meta_stride: 0,
        attr_stride: 16,
        capacity: 32,
    });
    assert_eq!(
        EntryStoreReader::calculate_size_on_mem(&EntryStoreConfig {
            core_stride: 8,
            meta_stride: 0,
            attr_stride: 16,
            capacity: 32
        }),
        base
    );
    assert_eq!(
        EntryStoreReader::calculate_size_on_mem(&EntryStoreConfig {
            core_stride: 0,
            meta_stride: 8,
            attr_stride: 16,
            capacity: 32
        }),
        base
    );
    assert_eq!(
        EntryStoreReader::calculate_size_on_mem(&EntryStoreConfig {
            core_stride: 8,
            meta_stride: 16,
            attr_stride: 16,
            capacity: 32
        }),
        base
    );
    assert_eq!(
        EntryStoreReader::calculate_size_on_mem(&EntryStoreConfig {
            core_stride: 64,
            meta_stride: 64,
            attr_stride: 16,
            capacity: 32
        }),
        base
    );
}

// ---- Writer -> Reader roundtrip with META ----

#[test]
fn core_meta_writer_reader_roundtrip_after_publish_swap() {
    const C: usize = 4;
    const M: usize = 4;
    let (_mem, tb, store) = make_store_cma(C, M, 16, 4);
    let tb_reader = tb.to_reader();

    let s = store.insert().unwrap();
    let core: [i32; C] = [11, 22, 33, 44];
    let meta: [i32; M] = [-11, -22, -33, -44];
    store.get(s).core_write_all(&core);
    store.get(s).meta_write_all(&meta);

    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();
    let mut cr = [0i32; C];
    let mut mr = [0i32; M];
    reader.get(s).core_read_all(&mut cr);
    reader.get(s).meta_read_all(&mut mr);
    assert_eq!(cr, core);
    assert_eq!(mr, meta);
    for i in 0..C {
        assert_eq!(reader.get(s).core_read(i), core[i]);
    }
    for j in 0..M {
        assert_eq!(reader.get(s).meta_read(j), meta[j]);
    }
}

#[test]
fn core_meta_roundtrip_with_1_1_edge_case() {
    let (_mem, tb, store) = make_store_cma(1, 1, 1, 1);
    let tb_reader = tb.to_reader();

    let s = store.insert().unwrap();
    store.get(s).core_write(0, 7);
    store.get(s).meta_write(0, -9);

    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();
    assert_eq!(reader.get(s).core_read(0), 7);
    assert_eq!(reader.get(s).meta_read(0), -9);
    let mut cbuf = [0i32; 1];
    let mut mbuf = [0i32; 1];
    reader.get(s).core_read_all(&mut cbuf);
    reader.get(s).meta_read_all(&mut mbuf);
    assert_eq!(cbuf, [7]);
    assert_eq!(mbuf, [-9]);
}

#[test]
fn core_meta_roundtrip_with_large_strides() {
    // CORE=64, META=64, capacity=4 => 4 * 128 = 512 <= TB_BUFFER_CAPACITY (1024).
    const C: usize = 64;
    const M: usize = 64;
    const CAP: u32 = 4;
    let (_mem, tb, store) = make_store_cma(C, M, 16, CAP);
    let tb_reader = tb.to_reader();

    let s1 = store.insert().unwrap();
    let s2 = store.insert().unwrap();
    assert_eq!((s1, s2), (slot(1), slot(2)));

    let mut c1 = [0i32; C];
    let mut m1 = [0i32; M];
    let mut c2 = [0i32; C];
    let mut m2 = [0i32; M];
    for i in 0..C {
        c1[i] = i as i32;
        c2[i] = -(i as i32) - 1;
    }
    for j in 0..M {
        m1[j] = (j as i32) + 1000;
        m2[j] = -(j as i32) - 2000;
    }
    store.get(s1).core_write_all(&c1);
    store.get(s1).meta_write_all(&m1);
    store.get(s2).core_write_all(&c2);
    store.get(s2).meta_write_all(&m2);

    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();
    let mut r1c = [0i32; C];
    let mut r1m = [0i32; M];
    let mut r2c = [0i32; C];
    let mut r2m = [0i32; M];
    reader.get(s1).core_read_all(&mut r1c);
    reader.get(s1).meta_read_all(&mut r1m);
    reader.get(s2).core_read_all(&mut r2c);
    reader.get(s2).meta_read_all(&mut r2m);
    assert_eq!(r1c, c1);
    assert_eq!(r1m, m1);
    assert_eq!(r2c, c2);
    assert_eq!(r2m, m2);
}

// ---- Layout verification via raw TripleBufferReader ----

#[test]
fn reader_core_meta_sees_tb_at_expected_interleaved_offsets() {
    const C: usize = 4;
    const M: usize = 4;
    let (_mem, tb, store) = make_store_cma(C, M, 16, 4);
    let tb_reader = tb.to_reader();

    let s1 = store.insert().unwrap();
    let s2 = store.insert().unwrap();
    let s3 = store.insert().unwrap();
    assert_eq!((s1, s2, s3), (slot(1), slot(2), slot(3)));

    store.get(s1).core_write_all(&[1, 2, 3, 4]);
    store.get(s1).meta_write_all(&[5, 6, 7, 8]);
    store.get(s2).core_write_all(&[9, 10, 11, 12]);
    store.get(s2).meta_write_all(&[13, 14, 15, 16]);
    store.get(s3).core_write_all(&[17, 18, 19, 20]);
    store.get(s3).meta_write_all(&[21, 22, 23, 24]);

    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();

    for (k, (core_exp, meta_exp)) in [
        ([1, 2, 3, 4], [5, 6, 7, 8]),
        ([9, 10, 11, 12], [13, 14, 15, 16]),
        ([17, 18, 19, 20], [21, 22, 23, 24]),
    ]
    .iter()
    .enumerate()
    {
        let slot_id = slot((k + 1) as u32);
        let mut cr = [0i32; C];
        let mut mr = [0i32; M];
        reader.get(slot_id).core_read_all(&mut cr);
        reader.get(slot_id).meta_read_all(&mut mr);
        assert_eq!(cr, *core_exp, "reader core slot {}", slot_id);
        assert_eq!(mr, *meta_exp, "reader meta slot {}", slot_id);

        // Raw TripleBufferReader at externally-computed absolute offsets.
        let start = k * (C + M); // tb_start_offset = 0
        for i in 0..C {
            assert_eq!(
                tb_reader.read(start + i),
                core_exp[i],
                "tb core slot {} [{}]",
                slot_id,
                i
            );
        }
        for j in 0..M {
            assert_eq!(
                tb_reader.read(start + C + j),
                meta_exp[j],
                "tb meta slot {} [{}]",
                slot_id,
                j
            );
        }
    }
}

#[test]
fn reader_core_meta_distinct_slots_do_not_overlap() {
    const C: usize = 4;
    const M: usize = 4;
    let (_mem, tb, store) = make_store_cma(C, M, 16, 4);
    let tb_reader = tb.to_reader();

    let s1 = store.insert().unwrap();
    let s2 = store.insert().unwrap();
    assert_eq!((s1, s2), (slot(1), slot(2)));

    // Write only slot 1. Slot 2's core+meta zone must remain zero.
    store.get(s1).core_write_all(&[0xAA_AA_AA_AAu32 as i32; C]);
    store.get(s1).meta_write_all(&[0xBB_BB_BB_BBu32 as i32; M]);

    tb.publish();
    assert!(tb_reader.swap());

    let reader = store.to_reader();
    let mut aac = [0i32; C];
    let mut aam = [0i32; M];
    let mut z2c = [0i32; C];
    let mut z2m = [0i32; M];
    reader.get(s1).core_read_all(&mut aac);
    reader.get(s1).meta_read_all(&mut aam);
    reader.get(s2).core_read_all(&mut z2c);
    reader.get(s2).meta_read_all(&mut z2m);
    assert_eq!(aac, [0xAA_AA_AA_AAu32 as i32; C]);
    assert_eq!(aam, [0xBB_BB_BB_BBu32 as i32; M]);
    assert_eq!(z2c, [0; C]);
    assert_eq!(z2m, [0; M]);
}

// ---- Bounds panics for META on reader side ----

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TbZoneReader.read | offset")]
fn reader_meta_read_at_stride_panics() {
    const M: usize = 4;
    let (_mem, tb, store) = make_store_cma(4, M, 16, 4);
    let tb_reader = tb.to_reader();

    // Publish an active slot so the reader has something valid behind slot 1,
    // but the bounds check in TbZoneReader.read fires regardless.
    let _s = store.insert().unwrap();
    tb.publish();
    let _ = tb_reader.swap();

    let reader = store.to_reader();
    // One past the last valid meta offset.
    let _ = reader.get(slot(1)).meta_read(M);
}
