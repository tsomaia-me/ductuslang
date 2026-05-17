use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use synaptic_kernel::primitives::triple_buffer_reader::TripleBufferReader;
use synaptic_kernel::primitives::triple_buffer_reader_registry::TripleBufferReaderRegistry;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::triple_buffer_writer_registry::TripleBufferWriterRegistry;
use synaptic_kernel::primitives::types::AtomicBuffer;

/// Default TB in the registry must have strictly positive `buffer_capacity` (`TripleBufferWriter` invariant).
const DEFAULT_TB_CAP: u32 = 1;

/// MEM footprint of the default triple buffer in `TripleBufferWriterRegistry` (before user TBs).
fn default_tb_mem() -> usize {
    TripleBufferWriter::calculate_size_on_mem(DEFAULT_TB_CAP as usize)
}

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

fn def(id: u16, cap: usize) -> TripleBufferDef {
    TripleBufferDef {
        id: TripleBufferId(id),
        buffer_capacity: cap,
    }
}

// ============ Happy Path — Construction ============

#[test]
fn construct_n_eq_one() {
    let mem = create_mem(1024);
    let defs = [def(0, 4)];
    let reg = TripleBufferWriterRegistry::<1>::new(mem, defs, 0, DEFAULT_TB_CAP);
    assert_eq!(reg.mem_start_offset(), 0);
    assert_eq!(reg.mem_end_offset(), default_tb_mem() + (4 + 4 * 3));
}

#[test]
fn construct_n_eq_two_identity_permutation() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];
    let reg = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);

    // ID 0 and 1 must both be accessible and point to distinct writers.
    let w0_base_initial = reg.get(TripleBufferId(0)).mem_start_offset();
    let w1_base_initial = reg.get(TripleBufferId(1)).mem_start_offset();
    let stride = 4 + 4 * 3;
    assert_eq!(w0_base_initial, default_tb_mem());
    assert_eq!(w1_base_initial, default_tb_mem() + stride);
}

#[test]
fn construct_n_eq_two_reversed_permutation() {
    let mem = create_mem(1024);
    // Position 0 in defs has user_id=1, position 1 has user_id=0.
    let defs = [def(1, 4), def(0, 4)];
    let reg = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);

    let stride = 4 + 4 * 3;
    let d = default_tb_mem();
    // ID=1 was placed at position 0 → first user TB after the default TB.
    // ID=0 was placed at position 1 → second user TB.
    assert_eq!(reg.get(TripleBufferId(1)).mem_start_offset(), d);
    assert_eq!(reg.get(TripleBufferId(0)).mem_start_offset(), d + stride);
}

#[test]
fn construct_n_eq_four_arbitrary_permutation() {
    let mem = create_mem(4096);
    // defs order: [id=2, id=0, id=3, id=1] with cap=4 each.
    let defs = [def(2, 4), def(0, 4), def(3, 4), def(1, 4)];
    let reg = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);

    let stride = 4 + 4 * 3;
    let d = default_tb_mem();
    // id_index[user_id] = position; get(user_id) returns tbs[id_index[user_id]].
    // Positions: id=2→0, id=0→1, id=3→2, id=1→3.
    assert_eq!(
        reg.get(TripleBufferId(2)).mem_start_offset(),
        d + 0 * stride
    );
    assert_eq!(
        reg.get(TripleBufferId(0)).mem_start_offset(),
        d + 1 * stride
    );
    assert_eq!(
        reg.get(TripleBufferId(3)).mem_start_offset(),
        d + 2 * stride
    );
    assert_eq!(
        reg.get(TripleBufferId(1)).mem_start_offset(),
        d + 3 * stride
    );
}

#[test]
fn construct_n_eq_eight_shuffled_permutation() {
    let mem = create_mem(4096);
    // A shuffled permutation of 0..8.
    let order = [5u16, 2, 7, 0, 6, 3, 1, 4];
    let defs: [TripleBufferDef; 8] = std::array::from_fn(|i| def(order[i], 4));
    let reg = TripleBufferWriterRegistry::<8>::new(mem, defs, 0, DEFAULT_TB_CAP);

    let stride = 4 + 4 * 3;
    let d = default_tb_mem();
    // Each user_id must map to the correct position in memory.
    for (pos, &user_id) in order.iter().enumerate() {
        assert_eq!(
            reg.get(TripleBufferId(user_id)).mem_start_offset(),
            d + pos * stride,
            "user_id {} should be at position {}",
            user_id,
            pos
        );
    }
}

#[test]
fn construct_with_nonzero_mem_start_offset() {
    let mem = create_mem(4096);
    let defs = [def(0, 4), def(1, 4)];
    let reg = TripleBufferWriterRegistry::<2>::new(mem, defs, 100, DEFAULT_TB_CAP);

    let stride = 4 + 4 * 3;
    let d = default_tb_mem();
    assert_eq!(reg.mem_start_offset(), 100);
    assert_eq!(reg.mem_end_offset(), 100 + d + 2 * stride);
    assert_eq!(reg.get(TripleBufferId(0)).mem_start_offset(), 100 + d);
    assert_eq!(
        reg.get(TripleBufferId(1)).mem_start_offset(),
        100 + d + stride
    );
}

#[test]
fn construct_with_varying_capacity_per_def() {
    let mem = create_mem(4096);
    let defs = [def(0, 10), def(1, 50), def(2, 8)];
    let reg = TripleBufferWriterRegistry::<3>::new(mem, defs, 0, DEFAULT_TB_CAP);

    let d = default_tb_mem();
    // User TBs pack after the default TB on MEM.
    assert_eq!(reg.get(TripleBufferId(0)).mem_start_offset(), d);
    assert_eq!(
        reg.get(TripleBufferId(0)).mem_end_offset(),
        d + (4 + 10 * 3)
    );

    assert_eq!(
        reg.get(TripleBufferId(1)).mem_start_offset(),
        d + (4 + 10 * 3)
    );
    assert_eq!(
        reg.get(TripleBufferId(1)).mem_end_offset(),
        d + (4 + 10 * 3) + (4 + 50 * 3)
    );

    assert_eq!(
        reg.get(TripleBufferId(2)).mem_start_offset(),
        d + (4 + 10 * 3) + (4 + 50 * 3)
    );
    assert_eq!(
        reg.get(TripleBufferId(2)).mem_end_offset(),
        d + (4 + 10 * 3) + (4 + 50 * 3) + (4 + 8 * 3)
    );

    assert_eq!(
        reg.mem_end_offset(),
        d + (4 + 10 * 3) + (4 + 50 * 3) + (4 + 8 * 3)
    );
}

#[test]
fn new_initializes_fresh_memory_and_bind_reattaches() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];

    // new() on fresh zeroed memory.
    let w1 = TripleBufferWriterRegistry::<2>::new(mem.clone(), defs, 0, DEFAULT_TB_CAP);
    w1.get(TripleBufferId(0)).write(0, 77);
    w1.get(TripleBufferId(0)).publish();

    w1.get(TripleBufferId(1)).write(0, 88);
    w1.get(TripleBufferId(1)).publish();

    // bind() on memory previously used — should NOT re-initialize state.
    let w2 = TripleBufferWriterRegistry::<2>::bind(mem.clone(), defs, 0, DEFAULT_TB_CAP);
    let r2 = w2.to_reader();

    // r2 should see the previously published values through bind recovery.
    assert!(r2.get(TripleBufferId(0)).swap());
    assert_eq!(r2.get(TripleBufferId(0)).read(0), 77);

    assert!(r2.get(TripleBufferId(1)).swap());
    assert_eq!(r2.get(TripleBufferId(1)).read(0), 88);
}

// ============ get — Correctness Under Permutations ============

#[test]
fn get_identity_permutation_writes_do_not_cross_contaminate() {
    let mem = create_mem(4096);
    let defs = [def(0, 4), def(1, 4), def(2, 4), def(3, 4)];
    let reg = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);

    // Write a distinctive value through each ID's writer.
    for i in 0..4u16 {
        reg.get(TripleBufferId(i)).write(0, ((i + 1) * 1000) as i32);
    }

    // Read back — each writer should see its own value.
    for i in 0..4u16 {
        assert_eq!(
            reg.get(TripleBufferId(i)).read(0),
            ((i + 1) * 1000) as i32,
            "ID {} cross-contaminated",
            i
        );
    }
}

#[test]
fn get_reversed_permutation_maps_ids_correctly() {
    let mem = create_mem(4096);
    // Positions: id=3→0, id=2→1, id=1→2, id=0→3.
    let defs = [def(3, 4), def(2, 4), def(1, 4), def(0, 4)];
    let reg = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);

    // Write through each ID.
    reg.get(TripleBufferId(0)).write(0, 100);
    reg.get(TripleBufferId(1)).write(0, 200);
    reg.get(TripleBufferId(2)).write(0, 300);
    reg.get(TripleBufferId(3)).write(0, 400);

    // Each ID reads its own value — no cross-contamination.
    assert_eq!(reg.get(TripleBufferId(0)).read(0), 100);
    assert_eq!(reg.get(TripleBufferId(1)).read(0), 200);
    assert_eq!(reg.get(TripleBufferId(2)).read(0), 300);
    assert_eq!(reg.get(TripleBufferId(3)).read(0), 400);

    // Verify physical placement: id=3 is at position 0 in the defs array.
    let stride = 4 + 4 * 3;
    let d = default_tb_mem();
    assert_eq!(
        reg.get(TripleBufferId(3)).mem_start_offset(),
        d + 0 * stride
    );
    assert_eq!(
        reg.get(TripleBufferId(0)).mem_start_offset(),
        d + 3 * stride
    );
}

#[test]
fn get_arbitrary_permutation_no_leakage() {
    let mem = create_mem(4096);
    let order = [5u16, 2, 7, 0, 6, 3, 1, 4];
    let defs: [TripleBufferDef; 8] = std::array::from_fn(|i| def(order[i], 4));
    let reg = TripleBufferWriterRegistry::<8>::new(mem, defs, 0, DEFAULT_TB_CAP);

    // Write a unique value through each ID.
    for id in 0..8u16 {
        reg.get(TripleBufferId(id)).write(0, (id as i32 + 1) * 11);
    }

    // Verify each ID reads back exactly what it wrote.
    for id in 0..8u16 {
        assert_eq!(
            reg.get(TripleBufferId(id)).read(0),
            (id as i32 + 1) * 11,
            "ID {} leakage detected",
            id
        );
    }
}

// ============ calculate_size_on_mem ============

#[test]
fn calculate_size_on_mem_single_def() {
    let defs = [def(0, 7)];
    let size = TripleBufferWriterRegistry::<1>::calculate_size_on_mem(DEFAULT_TB_CAP as usize, &defs);
    assert_eq!(
        size,
        TripleBufferWriter::calculate_size_on_mem(DEFAULT_TB_CAP as usize)
            + TripleBufferWriter::calculate_size_on_mem(7)
    );
}

#[test]
fn calculate_size_on_mem_n_defs_uniform_capacity() {
    let defs = [def(0, 4), def(1, 4), def(2, 4)];
    let size = TripleBufferWriterRegistry::<3>::calculate_size_on_mem(DEFAULT_TB_CAP as usize, &defs);
    assert_eq!(
        size,
        TripleBufferWriter::calculate_size_on_mem(DEFAULT_TB_CAP as usize) + 3 * (4 + 4 * 3)
    );
}

#[test]
fn calculate_size_on_mem_n_defs_varying_capacity() {
    let defs = [def(0, 10), def(1, 50), def(2, 8)];
    let size = TripleBufferWriterRegistry::<3>::calculate_size_on_mem(DEFAULT_TB_CAP as usize, &defs);
    assert_eq!(
        size,
        TripleBufferWriter::calculate_size_on_mem(DEFAULT_TB_CAP as usize) + (4 + 30) + (4 + 150) + (4 + 24)
    );
}

#[test]
fn calculate_size_on_mem_takes_reference() {
    // Verify we can call it with a borrow and defs stays usable afterwards.
    let defs = [def(0, 4), def(1, 8)];
    let size1 = TripleBufferWriterRegistry::<2>::calculate_size_on_mem(DEFAULT_TB_CAP as usize, &defs);
    let size2 = TripleBufferWriterRegistry::<2>::calculate_size_on_mem(DEFAULT_TB_CAP as usize, &defs);
    assert_eq!(size1, size2);
    // defs is still usable:
    assert_eq!(defs[0].buffer_capacity, 4);
}

// ============ mem_start_offset / mem_end_offset ============

#[test]
fn mem_offsets_at_zero_start() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 8)];
    let reg = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
    assert_eq!(reg.mem_start_offset(), 0);
    assert_eq!(
        reg.mem_end_offset(),
        TripleBufferWriterRegistry::<2>::calculate_size_on_mem(DEFAULT_TB_CAP as usize, &defs)
    );
}

#[test]
fn mem_offsets_at_nonzero_start() {
    let mem = create_mem(2048);
    let defs = [def(0, 4), def(1, 8), def(2, 12)];
    let start = 257;
    let reg = TripleBufferWriterRegistry::<3>::new(mem, defs, start, DEFAULT_TB_CAP);
    assert_eq!(reg.mem_start_offset(), start);
    assert_eq!(
        reg.mem_end_offset(),
        start + TripleBufferWriterRegistry::<3>::calculate_size_on_mem(DEFAULT_TB_CAP as usize, &defs)
    );
}

// ============ to_reader ============

#[test]
fn to_reader_produces_registry_with_matching_offsets() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<2>::new(mem, defs, 50, DEFAULT_TB_CAP);
    let r: TripleBufferReaderRegistry<2> = w.to_reader();

    assert_eq!(r.mem_start_offset(), w.mem_start_offset());
    assert_eq!(r.mem_end_offset(), w.mem_end_offset());
}

#[test]
fn to_reader_preserves_id_mapping_roundtrip() {
    let mem = create_mem(4096);
    let defs = [def(2, 4), def(0, 4), def(3, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Round-trip value through each ID.
    for id in 0..4u16 {
        w.get(TripleBufferId(id)).write(0, (id as i32 + 1) * 9);
        w.get(TripleBufferId(id)).publish();
    }

    for id in 0..4u16 {
        assert!(
            r.get(TripleBufferId(id)).swap(),
            "ID {} swap returned false",
            id
        );
        assert_eq!(
            r.get(TripleBufferId(id)).read(0),
            (id as i32 + 1) * 9,
            "ID {} value mismatch",
            id
        );
    }
}

#[test]
fn to_reader_pairs_are_independent_across_ids() {
    let mem = create_mem(4096);
    let defs = [def(0, 4), def(1, 4), def(2, 4)];
    let w = TripleBufferWriterRegistry::<3>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Publish ONLY through ID=2.
    w.get(TripleBufferId(2)).write(0, 999);
    w.get(TripleBufferId(2)).publish();

    // Only ID=2's reader should report new data.
    assert!(!r.get(TripleBufferId(0)).swap());
    assert!(!r.get(TripleBufferId(1)).swap());
    assert!(r.get(TripleBufferId(2)).swap());
    assert_eq!(r.get(TripleBufferId(2)).read(0), 999);
}

// ============ Independence of TBs in Registry ============

#[test]
fn distinct_patterns_through_different_ids_no_cross_contamination() {
    let mem = create_mem(4096);
    let defs = [def(0, 4), def(1, 4), def(2, 4), def(3, 4)];
    let w = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Each ID writes a unique pattern on all four offsets, then publishes.
    for id in 0..4u16 {
        for off in 0..4 {
            w.get(TripleBufferId(id))
                .write(off, (id as i32 * 100) + off as i32);
        }
        w.get(TripleBufferId(id)).publish();
    }

    for id in 0..4u16 {
        assert!(r.get(TripleBufferId(id)).swap());
        for off in 0..4 {
            assert_eq!(
                r.get(TripleBufferId(id)).read(off),
                (id as i32 * 100) + off as i32,
                "cross-contamination: id {} off {}",
                id,
                off
            );
        }
    }
}

#[test]
fn sequential_publishes_on_all_tbs_no_state_bleed() {
    let mem = create_mem(4096);
    let defs: [TripleBufferDef; 5] = std::array::from_fn(|i| def(i as u16, 4));
    let w = TripleBufferWriterRegistry::<5>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Run N concurrent publishes on N different TBs, single-threaded.
    for id in 0..5u16 {
        w.get(TripleBufferId(id)).write(0, id as i32 * 777);
        w.get(TripleBufferId(id)).publish();
    }

    for id in 0..5u16 {
        assert!(r.get(TripleBufferId(id)).swap());
        assert_eq!(r.get(TripleBufferId(id)).read(0), id as i32 * 777);
    }
}

// ============ Clone ============

#[test]
fn clone_shares_underlying_atomic_buffer() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let w_clone = w.clone();

    // Write through original, publish.
    w.get(TripleBufferId(0)).write(0, 42);
    w.get(TripleBufferId(0)).publish();

    // Clone's reader should see the data (shared Arc<[AtomicI32]>).
    let r_clone = w_clone.to_reader();
    assert!(r_clone.get(TripleBufferId(0)).swap());
    assert_eq!(r_clone.get(TripleBufferId(0)).read(0), 42);
}

#[test]
fn clone_modifications_visible_through_original() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let w_clone = w.clone();

    // Write through the clone, publish.
    w_clone.get(TripleBufferId(1)).write(0, 314);
    w_clone.get(TripleBufferId(1)).publish();

    // Original's reader sees it.
    let r = w.to_reader();
    assert!(r.get(TripleBufferId(1)).swap());
    assert_eq!(r.get(TripleBufferId(1)).read(0), 314);
}

// ============ Cross-Registry Integration ============

#[test]
fn spsc_roundtrip_per_tb_independent() {
    let mem = create_mem(4096);
    let defs = [def(0, 8), def(1, 8), def(2, 8), def(3, 8)];
    let w = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Producer writes a distinct vector to each TB.
    for id in 0..4u16 {
        for off in 0..8 {
            w.get(TripleBufferId(id))
                .write(off, (id as i32 + 1) * 10 + off as i32);
        }
        w.get(TripleBufferId(id)).publish();
    }

    // Consumer reads each TB independently.
    for id in 0..4u16 {
        assert!(r.get(TripleBufferId(id)).swap());
        for off in 0..8 {
            assert_eq!(
                r.get(TripleBufferId(id)).read(off),
                (id as i32 + 1) * 10 + off as i32
            );
        }
    }
}

#[test]
fn multiple_publish_cycles_one_tb_others_quiescent() {
    let mem = create_mem(4096);
    let defs = [def(0, 4), def(1, 4), def(2, 4)];
    let w = TripleBufferWriterRegistry::<3>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Run 10 publish/swap cycles on ID=1 only; ID=0 and ID=2 stay quiescent.
    for round in 0..10i32 {
        w.get(TripleBufferId(1)).write(0, round * 100);
        w.get(TripleBufferId(1)).publish();
        assert!(r.get(TripleBufferId(1)).swap());
        assert_eq!(r.get(TripleBufferId(1)).read(0), round * 100);
    }

    // ID=0 and ID=2 never saw a publish — swap returns false, read returns 0.
    assert!(!r.get(TripleBufferId(0)).swap());
    assert_eq!(r.get(TripleBufferId(0)).read(0), 0);
    assert!(!r.get(TripleBufferId(2)).swap());
    assert_eq!(r.get(TripleBufferId(2)).read(0), 0);
}

#[test]
fn drop_frame_semantics_per_tb() {
    let mem = create_mem(4096);
    let defs = [def(0, 4), def(1, 4), def(2, 4), def(3, 4)];
    let w = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Writer publishes twice on ID=3 without reader swapping.
    w.get(TripleBufferId(3)).write(0, 100);
    w.get(TripleBufferId(3)).publish();
    w.get(TripleBufferId(3)).write(0, 200);
    w.get(TripleBufferId(3)).publish();

    // Reader swaps once — must see the LATEST value (200), dropping frame 100.
    assert!(r.get(TripleBufferId(3)).swap());
    assert_eq!(r.get(TripleBufferId(3)).read(0), 200);

    // Other TBs report no new data.
    assert!(!r.get(TripleBufferId(0)).swap());
    assert!(!r.get(TripleBufferId(1)).swap());
    assert!(!r.get(TripleBufferId(2)).swap());
}

#[test]
fn get_returns_reference_can_be_reborrowed() {
    let mem = create_mem(1024);
    let defs = [def(0, 4)];
    let reg = TripleBufferWriterRegistry::<1>::new(mem, defs, 0, DEFAULT_TB_CAP);

    // Exercise that get returns an immutable reference usable across statements.
    let w_ref: &TripleBufferWriter = reg.get(TripleBufferId(0));
    w_ref.write(0, 111);
    w_ref.publish();
    // After publish the new writer buffer is synced from the published one.
    assert_eq!(w_ref.read(0), 111);
}

#[test]
fn reader_obtained_via_to_reader_is_correctly_typed() {
    let mem = create_mem(1024);
    let defs = [def(0, 4)];
    let w = TripleBufferWriterRegistry::<1>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Exercise the trait-free reference to TripleBufferReader.
    let r_ref: &TripleBufferReader = r.get(TripleBufferId(0));
    w.get(TripleBufferId(0)).write(0, 55);
    w.get(TripleBufferId(0)).publish();
    assert!(r_ref.swap());
    assert_eq!(r_ref.read(0), 55);
}

// ============ Should-Panic (debug-only) ============

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "duplicate id")]
fn duplicate_ids_panic_at_construction() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(0, 4)];
    let _ = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TripleBufferWriterRegistry::create | id 5 out of bounds")]
fn out_of_range_id_panics_at_construction() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(5, 4)];
    let _ = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TripleBufferWriterRegistry::get | id 2 out of bounds")]
fn get_out_of_range_id_panics() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];
    let reg = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let _ = reg.get(TripleBufferId(2));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "out of AtomicBuffer bounds")]
fn insufficient_mem_panics_at_construction() {
    // defs need default_tb_mem() + 2 * (4 + 4*3) slots. mem has only 10 → must panic.
    let mem = create_mem(10);
    let defs = [def(0, 4), def(1, 4)];
    let _ = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "out of AtomicBuffer bounds")]
fn mem_start_offset_pushes_past_mem_end_panics() {
    // Layout needs 16 slots; with start=20 → cursor reaches 36 > 32.
    let mem = create_mem(32);
    let defs = [def(0, 4)];
    let _ = TripleBufferWriterRegistry::<1>::new(mem, defs, 20, DEFAULT_TB_CAP);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "duplicate id")]
fn bind_with_duplicate_ids_also_panics() {
    let mem = create_mem(1024);
    let defs = [def(1, 4), def(1, 4)];
    let _ = TripleBufferWriterRegistry::<2>::bind(mem, defs, 0, DEFAULT_TB_CAP);
}
