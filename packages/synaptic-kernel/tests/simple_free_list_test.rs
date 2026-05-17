use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::simple_free_list::SimpleFreeList;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

// ============ Happy Paths ============

#[test]
fn alloc_returns_logical_slot_index() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 4);

    let slot = fl.alloc();
    assert!(slot.is_some());
    let idx = slot.unwrap();
    assert!(idx.get() >= 1 && idx.get() <= 4); // 1-based slot index within capacity
}

#[test]
fn free_count_starts_at_capacity() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 8);

    assert_eq!(fl.free_count(), 8);
}

#[test]
fn alloc_decrements_free_count() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 8);

    fl.alloc().unwrap();
    assert_eq!(fl.free_count(), 7);

    fl.alloc().unwrap();
    assert_eq!(fl.free_count(), 6);
}

#[test]
fn free_increments_free_count() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 8);

    let slot = fl.alloc().unwrap();
    assert_eq!(fl.free_count(), 7);

    fl.free(slot).unwrap();
    assert_eq!(fl.free_count(), 8);
}

#[test]
fn alloc_returns_sequential_indices() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 4);

    // Free chain is initialized as 0 → 1 → 2 → 3, alloc returns slot_index + 1
    assert_eq!(fl.alloc().unwrap(), SlotId::new(1).unwrap());
    assert_eq!(fl.alloc().unwrap(), SlotId::new(2).unwrap());
    assert_eq!(fl.alloc().unwrap(), SlotId::new(3).unwrap());
    assert_eq!(fl.alloc().unwrap(), SlotId::new(4).unwrap());
}

#[test]
fn alloc_all_then_returns_none() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 4);

    for _ in 0..4 {
        fl.alloc().unwrap();
    }
    assert_eq!(fl.free_count(), 0);
    assert!(fl.alloc().is_none());
}

#[test]
fn free_and_realloc_reuses_slot() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 4);

    let slot0 = fl.alloc().unwrap();
    let _slot1 = fl.alloc().unwrap();

    fl.free(slot0).unwrap();

    // Freed slot goes to head of free chain, so next alloc returns it
    let reused = fl.alloc().unwrap();
    assert_eq!(reused, slot0);
}

#[test]
fn mem_end_offset_is_correct() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 8);

    // Layout: head(1) + free_count(1) + bitmap(ceil(8/32)=1) + slots(8) = 11
    // mem_end_offset = start of slots + capacity = 3 + 8 = 11
    assert_eq!(fl.mem_end_offset(), 3 + 8);
}

#[test]
fn mem_end_offset_with_larger_capacity() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 64);

    // Layout: head(1) + free_count(1) + bitmap(ceil(64/32)=2) + slots(64)
    // mem_end_offset = 0 + 2 + 2 + 64 = 68
    assert_eq!(fl.mem_end_offset(), 4 + 64);
}

#[test]
fn nonzero_start_index() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 100, 8);

    assert_eq!(fl.free_count(), 8);

    let slot = fl.alloc().unwrap();
    assert!(slot.get() >= 1 && slot.get() <= 8); // 1-based slot index
    assert_eq!(fl.free_count(), 7);

    fl.free(slot).unwrap();
    assert_eq!(fl.free_count(), 8);
}

#[test]
fn nonzero_start_index_end_index() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 100, 8);

    // Layout: 100 + head(1) + free_count(1) + bitmap(1) + slots(8) = 111
    assert_eq!(fl.mem_end_offset(), 100 + 2 + 1 + 8);
}

// ============ Edge Cases ============

#[test]
fn capacity_of_one() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 1);

    assert_eq!(fl.free_count(), 1);

    let slot = fl.alloc().unwrap();
    assert_eq!(slot, SlotId::new(1).unwrap());
    assert!(fl.alloc().is_none());
    assert_eq!(fl.free_count(), 0);

    fl.free(slot).unwrap();
    assert_eq!(fl.free_count(), 1);

    let slot2 = fl.alloc().unwrap();
    assert_eq!(slot2, SlotId::new(1).unwrap());
}

#[test]
fn capacity_of_two() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 2);

    let a = fl.alloc().unwrap();
    let b = fl.alloc().unwrap();
    assert!(fl.alloc().is_none());

    fl.free(a).unwrap();
    fl.free(b).unwrap();
    assert_eq!(fl.free_count(), 2);
}

#[test]
fn alloc_free_alloc_cycle() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 4);

    for _ in 0..10 {
        let mut slots = Vec::new();
        for _ in 0..4 {
            slots.push(fl.alloc().unwrap());
        }
        assert!(fl.alloc().is_none());

        for s in slots {
            fl.free(s).unwrap();
        }
        assert_eq!(fl.free_count(), 4);
    }
}

// ============ Double-Free Detection ============

#[test]
fn double_free_returns_error() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 4);

    let slot = fl.alloc().unwrap();
    fl.free(slot).unwrap();

    let result = fl.free(slot);
    assert!(result.is_err());
}

#[test]
fn double_free_does_not_corrupt_free_count() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 4);

    let slot = fl.alloc().unwrap();
    fl.free(slot).unwrap();
    assert_eq!(fl.free_count(), 4);

    let _ = fl.free(slot); // should fail
    assert_eq!(fl.free_count(), 4); // unchanged
}

#[test]
fn free_realloc_free_is_not_double_free() {
    let mem = create_mem(4096);
    let fl = SimpleFreeList::new(mem, 0, 4);

    let slot = fl.alloc().unwrap();
    fl.free(slot).unwrap();

    // Realloc same slot
    let slot_again = fl.alloc().unwrap();
    assert_eq!(slot_again, slot);

    // Free again — should succeed (it's occupied now)
    fl.free(slot_again).unwrap();
    assert_eq!(fl.free_count(), 4);
}

// ============ Bitmap Correctness ============

#[test]
fn bitmap_spans_multiple_words() {
    let mem = create_mem(8192);
    // 64 slots = 2 bitmap words (64 / 32 = 2)
    let fl = SimpleFreeList::new(mem, 0, 64);

    // Alloc slots from both bitmap words
    let mut slots = Vec::new();
    for _ in 0..64 {
        slots.push(fl.alloc().unwrap());
    }
    assert!(fl.alloc().is_none());

    // Free every slot — exercises both bitmap words
    for s in &slots {
        fl.free(*s).unwrap();
    }
    assert_eq!(fl.free_count(), 64);

    // Double-free check across bitmap word boundary
    let _ = fl.alloc().unwrap(); // alloc one
    let first_free = slots[1]; // should still be free
    let result = fl.free(first_free);
    assert!(result.is_err()); // it's free, so double-free
}

#[test]
fn slot_31_and_32_bitmap_boundary() {
    let mem = create_mem(8192);
    let fl = SimpleFreeList::new(mem, 0, 64);

    // Alloc all
    let mut slots = Vec::new();
    for _ in 0..64 {
        slots.push(fl.alloc().unwrap());
    }

    // Free slot 32 (1-based: internal index 31, last bit of word 0)
    // and slot 33 (1-based: internal index 32, first bit of word 1)
    fl.free(SlotId::new(32).unwrap()).unwrap();
    fl.free(SlotId::new(33).unwrap()).unwrap();
    assert_eq!(fl.free_count(), 2);

    // Double-free on boundary slots
    assert!(fl.free(SlotId::new(32).unwrap()).is_err());
    assert!(fl.free(SlotId::new(33).unwrap()).is_err());
}

// ============ Bind (Attach to Existing AtomicBuffer) ============

#[test]
fn bind_reads_existing_state() {
    let mem = create_mem(4096);

    // Initialize with new
    let fl = SimpleFreeList::new(mem.clone(), 0, 4);
    let _s0 = fl.alloc().unwrap();
    let _s1 = fl.alloc().unwrap();
    assert_eq!(fl.free_count(), 2);

    // Bind to same region — should see same state
    let fl2 = SimpleFreeList::bind(mem, 0, 4);
    assert_eq!(fl2.free_count(), 2);
}

#[test]
fn bind_can_alloc_remaining() {
    let mem = create_mem(4096);

    let fl = SimpleFreeList::new(mem.clone(), 0, 4);
    fl.alloc().unwrap();
    fl.alloc().unwrap();

    let fl2 = SimpleFreeList::bind(mem, 0, 4);
    let s = fl2.alloc().unwrap();
    assert_eq!(fl2.free_count(), 1);
    fl2.free(s).unwrap();
    assert_eq!(fl2.free_count(), 2);
}

// ============ Stress Tests ============

#[test]
fn stress_alloc_free_all_256() {
    let mem = create_mem(65536);
    let fl = SimpleFreeList::new(mem, 0, 256);

    // Alloc all
    let mut slots: Vec<SlotId> = (0..256).map(|_| fl.alloc().unwrap()).collect();
    assert!(fl.alloc().is_none());
    assert_eq!(fl.free_count(), 0);

    // Free in reverse order
    while let Some(s) = slots.pop() {
        fl.free(s).unwrap();
    }
    assert_eq!(fl.free_count(), 256);

    // Alloc all again
    let slots: Vec<SlotId> = (0..256).map(|_| fl.alloc().unwrap()).collect();
    assert!(fl.alloc().is_none());

    // Free every other one
    for (i, s) in slots.iter().enumerate() {
        if i % 2 == 0 {
            fl.free(*s).unwrap();
        }
    }
    assert_eq!(fl.free_count(), 128);

    // Alloc the freed ones back
    for _ in 0..128 {
        fl.alloc().unwrap();
    }
    assert!(fl.alloc().is_none());
    assert_eq!(fl.free_count(), 0);
}

#[test]
fn stress_interleaved_alloc_free() {
    let mem = create_mem(65536);
    let fl = SimpleFreeList::new(mem, 0, 128);

    let mut active = Vec::new();

    for i in 0..10_000 {
        if active.len() < 128 && i % 3 != 0 {
            if let Some(slot) = fl.alloc() {
                active.push(slot);
            }
        } else if !active.is_empty() {
            let slot = active.remove(0);
            fl.free(slot).unwrap();
        }
    }

    // Invariant: free_count + active = capacity
    assert_eq!(fl.free_count() + active.len(), 128);
}

#[test]
fn stress_unique_indices() {
    let mem = create_mem(65536);
    let fl = SimpleFreeList::new(mem, 0, 256);

    let slots: Vec<SlotId> = (0..256).map(|_| fl.alloc().unwrap()).collect();

    // All indices should be unique
    let mut sorted = slots.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 256);

    // All should be in [1, 256] (1-based)
    for s in &slots {
        assert!(s.get() >= 1 && s.get() <= 256);
    }
}

#[test]
fn stress_free_chain_integrity_after_mixed_ops() {
    let mem = create_mem(65536);
    let fl = SimpleFreeList::new(mem, 0, 64);

    // Alloc 64, free odd indices, alloc 32, free all
    let slots: Vec<SlotId> = (0..64).map(|_| fl.alloc().unwrap()).collect();

    for s in &slots {
        if s.get() % 2 == 0 {
            fl.free(*s).unwrap();
        }
    }
    assert_eq!(fl.free_count(), 32);

    // Alloc the 32 freed slots
    let mut reallocated = Vec::new();
    for _ in 0..32 {
        reallocated.push(fl.alloc().unwrap());
    }
    assert!(fl.alloc().is_none());
    assert_eq!(fl.free_count(), 0);

    // Free everything
    for s in &slots {
        if s.get() % 2 == 1 {
            fl.free(*s).unwrap();
        }
    }
    for s in reallocated {
        fl.free(s).unwrap();
    }
    assert_eq!(fl.free_count(), 64);
}

// ============ Copy From / Resize ============

#[test]
fn copy_from_preserves_state_and_adds_capacity() {
    let mem_small = create_mem(4096);
    let small_fl = SimpleFreeList::new(mem_small, 0, 16);

    let _a = small_fl.alloc().unwrap();
    let b = small_fl.alloc().unwrap();
    let _c = small_fl.alloc().unwrap();

    small_fl.free(b).unwrap(); // b is freed, free_count is 14

    assert_eq!(small_fl.free_count(), 14);

    let mem_large = create_mem(4096);
    let large_fl = SimpleFreeList::new(mem_large, 0, 32);

    large_fl.copy_from(&small_fl);

    // Initial 16 had 14 free. 16 new slots added. Total free: 30.
    assert_eq!(large_fl.free_count(), 30);

    // With the splice fix, the old chain naturally flows into the expanded region.
    // The old free list head was slot b (freed), so that pops first.
    let d = large_fl.alloc().unwrap();
    assert_eq!(d, b);

    // Then the remaining old chain drains: slots 4..16 (13 slots)
    for expected in 4u32..=16 {
        let got = large_fl.alloc().unwrap();
        assert_eq!(got, SlotId::new(expected).unwrap());
    }

    // Then the expanded region follows sequentially: slots 17..32
    for expected in 17u32..=32 {
        let got = large_fl.alloc().unwrap();
        assert_eq!(got, SlotId::new(expected).unwrap());
    }

    // All 30 free slots consumed
    assert_eq!(large_fl.free_count(), 0);
    assert!(large_fl.alloc().is_none());
}

#[test]
#[should_panic]
fn copy_from_panics_if_source_larger() {
    let mem_small = create_mem(4096);
    let small_fl = SimpleFreeList::new(mem_small, 0, 16);

    let mem_large = create_mem(4096);
    let large_fl = SimpleFreeList::new(mem_large, 0, 32);

    small_fl.copy_from(&large_fl);
}
