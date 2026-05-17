use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use synaptic_kernel::primitives::triple_buffer_reader::TripleBufferReader;
use synaptic_kernel::primitives::triple_buffer_reader_registry::TripleBufferReaderRegistry;
use synaptic_kernel::primitives::triple_buffer_writer::TripleBufferWriter;
use synaptic_kernel::primitives::triple_buffer_writer_registry::TripleBufferWriterRegistry;
use synaptic_kernel::primitives::types::AtomicBuffer;

/// Default TB in the registry must have strictly positive `buffer_capacity` (`TripleBufferWriter` invariant).
const DEFAULT_TB_CAP: u32 = 1;

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

// Helper to silence unused-import warnings for the typed imports; also acts
// as a sanity check that the imported types line up with what `get` returns.
fn _type_assertions(
    r: &TripleBufferReaderRegistry<1>,
    w: &TripleBufferWriterRegistry<1>,
) -> (usize, usize) {
    let _reader_ref: &TripleBufferReader = r.get(TripleBufferId(0));
    let _writer_ref: &TripleBufferWriter = w.get(TripleBufferId(0));
    (default_tb_mem(), r.mem_start_offset())
}

// ============ Happy Path — Construction via to_reader ============

#[test]
fn construct_via_to_reader_matches_writer_offsets() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<2>::new(mem, defs, 100, DEFAULT_TB_CAP);
    let r = w.to_reader();

    assert_eq!(r.mem_start_offset(), w.mem_start_offset());
    assert_eq!(r.mem_end_offset(), w.mem_end_offset());
    assert_eq!(r.mem_start_offset(), 100);
    assert_eq!(
        r.mem_end_offset(),
        100 + TripleBufferWriter::calculate_size_on_mem(DEFAULT_TB_CAP as usize) + 2 * (4 + 4 * 3)
    );
}

#[test]
fn construct_via_to_reader_n_eq_one() {
    let mem = create_mem(1024);
    let defs = [def(0, 8)];
    let w = TripleBufferWriterRegistry::<1>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();
    assert_eq!(r.mem_start_offset(), 0);
    assert_eq!(
        r.mem_end_offset(),
        TripleBufferWriter::calculate_size_on_mem(DEFAULT_TB_CAP as usize) + (4 + 8 * 3)
    );
}

#[test]
fn construct_via_to_reader_varying_capacities() {
    let mem = create_mem(4096);
    let defs = [def(0, 10), def(1, 50), def(2, 8)];
    let w = TripleBufferWriterRegistry::<3>::new(mem, defs, 17, DEFAULT_TB_CAP);
    let r = w.to_reader();
    assert_eq!(r.mem_start_offset(), 17);
    assert_eq!(
        r.mem_end_offset(),
        17 + TripleBufferWriter::calculate_size_on_mem(DEFAULT_TB_CAP as usize)
            + (4 + 30)
            + (4 + 150)
            + (4 + 24)
    );
}

// ============ get — Correctness Under Permutations ============

#[test]
fn reader_get_identity_permutation_matches_writer_memory() {
    let mem = create_mem(4096);
    let defs = [def(0, 4), def(1, 4), def(2, 4), def(3, 4)];
    let w = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Writer's mem_start_offset per ID must equal reader's mem_start_offset per ID.
    for id in 0..4u16 {
        assert_eq!(
            r.get(TripleBufferId(id)).mem_start_offset(),
            w.get(TripleBufferId(id)).mem_start_offset(),
            "id {}",
            id
        );
        assert_eq!(
            r.get(TripleBufferId(id)).mem_end_offset(),
            w.get(TripleBufferId(id)).mem_end_offset(),
            "id {}",
            id
        );
    }
}

#[test]
fn reader_get_reversed_permutation_maps_to_same_memory_as_writer() {
    let mem = create_mem(4096);
    let defs = [def(3, 4), def(2, 4), def(1, 4), def(0, 4)];
    let w = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    for id in 0..4u16 {
        assert_eq!(
            r.get(TripleBufferId(id)).mem_start_offset(),
            w.get(TripleBufferId(id)).mem_start_offset(),
            "reversed mapping mismatch for id {}",
            id
        );
    }
}

#[test]
fn reader_get_arbitrary_permutation_shared_memory_roundtrip() {
    let mem = create_mem(4096);
    // Arbitrary permutation.
    let defs = [def(2, 4), def(0, 4), def(3, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Write through writer's TB with each ID, then verify reader with same ID reads the value.
    for id in 0..4u16 {
        w.get(TripleBufferId(id)).write(0, (id as i32 + 1) * 111);
        w.get(TripleBufferId(id)).publish();
    }

    for id in 0..4u16 {
        assert!(
            r.get(TripleBufferId(id)).swap(),
            "id {} swap returned false",
            id
        );
        assert_eq!(
            r.get(TripleBufferId(id)).read(0),
            (id as i32 + 1) * 111,
            "id {} mismatch",
            id
        );
    }
}

#[test]
fn cross_component_writer_and_reader_share_memory_per_id() {
    let mem = create_mem(4096);
    let defs = [def(2, 4), def(0, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<3>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // For each ID, write through writer, swap reader, read back.
    for id in 0..3u16 {
        w.get(TripleBufferId(id)).write(0, id as i32 + 42);
        w.get(TripleBufferId(id)).publish();
        assert!(r.get(TripleBufferId(id)).swap());
        assert_eq!(r.get(TripleBufferId(id)).read(0), id as i32 + 42);
    }
}

// ============ Independent Per-ID Swap ============

#[test]
fn publish_on_one_id_only_that_readers_swap_true() {
    let mem = create_mem(4096);
    let defs = [def(0, 4), def(1, 4), def(2, 4)];
    let w = TripleBufferWriterRegistry::<3>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Publish only through ID=0.
    w.get(TripleBufferId(0)).publish();

    assert!(r.get(TripleBufferId(0)).swap());
    assert!(!r.get(TripleBufferId(1)).swap());
    assert!(!r.get(TripleBufferId(2)).swap());
}

#[test]
fn swap_per_id_isolated_across_many_cycles() {
    let mem = create_mem(4096);
    let defs = [def(0, 4), def(1, 4), def(2, 4), def(3, 4)];
    let w = TripleBufferWriterRegistry::<4>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();

    // Cycle: each round publish on a different ID and verify no others advance.
    for round in 0..12usize {
        let target = (round % 4) as u16;
        w.get(TripleBufferId(target))
            .write(0, (round as i32 + 1) * 17);
        w.get(TripleBufferId(target)).publish();

        for id in 0..4u16 {
            if id == target {
                assert!(
                    r.get(TripleBufferId(id)).swap(),
                    "round {} target {} swap false",
                    round,
                    id
                );
                assert_eq!(
                    r.get(TripleBufferId(id)).read(0),
                    (round as i32 + 1) * 17,
                    "round {} target {} value mismatch",
                    round,
                    id
                );
            } else {
                assert!(
                    !r.get(TripleBufferId(id)).swap(),
                    "round {} non-target {} swap true",
                    round,
                    id
                );
            }
        }
    }
}

// ============ Clone ============

#[test]
fn clone_shares_underlying_atomic_buffer() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r1 = w.to_reader();
    let r2 = r1.clone();

    // Writer publishes.
    w.get(TripleBufferId(0)).write(0, 1234);
    w.get(TripleBufferId(0)).publish();

    // r2 swaps first — should see data.
    assert!(r2.get(TripleBufferId(0)).swap());
    assert_eq!(r2.get(TripleBufferId(0)).read(0), 1234);

    // r1 observes that shared state has advanced: swap returns false since the
    // NEW_DATA flag was cleared by r2 (shared AtomicBuffer).
    assert!(!r1.get(TripleBufferId(0)).swap());
}

#[test]
fn clone_observes_writer_publishes_on_independent_clone() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();
    let r_clone = r.clone();

    // Publish twice on ID=1.
    w.get(TripleBufferId(1)).write(0, 555);
    w.get(TripleBufferId(1)).publish();

    // r swaps first.
    assert!(r.get(TripleBufferId(1)).swap());
    assert_eq!(r.get(TripleBufferId(1)).read(0), 555);

    // Second publish on same ID.
    w.get(TripleBufferId(1)).write(0, 666);
    w.get(TripleBufferId(1)).publish();

    // r_clone can consume the new publish (shared state via Arc).
    assert!(r_clone.get(TripleBufferId(1)).swap());
    assert_eq!(r_clone.get(TripleBufferId(1)).read(0), 666);
}

// ============ Construction Pairing With Writer Registry ============

#[test]
fn to_reader_called_twice_yields_compatible_registries() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r_a = w.to_reader();
    let r_b = w.to_reader();

    // Both readers share underlying memory. A publish must be consumable by only
    // one of them (whichever swaps first).
    w.get(TripleBufferId(0)).write(0, 321);
    w.get(TripleBufferId(0)).publish();

    assert!(r_a.get(TripleBufferId(0)).swap());
    assert_eq!(r_a.get(TripleBufferId(0)).read(0), 321);
    assert!(!r_b.get(TripleBufferId(0)).swap());
}

// ============ Should-Panic (debug-only) ============

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TripleBufferReaderRegistry::get | id 2 out of bounds")]
fn reader_get_out_of_range_id_panics() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4)];
    let w = TripleBufferWriterRegistry::<2>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();
    let _ = r.get(TripleBufferId(2));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "TripleBufferReaderRegistry::get | id 5 out of bounds")]
fn reader_get_far_out_of_range_id_panics() {
    let mem = create_mem(1024);
    let defs = [def(0, 4), def(1, 4), def(2, 4)];
    let w = TripleBufferWriterRegistry::<3>::new(mem, defs, 0, DEFAULT_TB_CAP);
    let r = w.to_reader();
    let _ = r.get(TripleBufferId(5));
}
