use std::sync::atomic::AtomicI32;
use std::sync::Arc;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::slot_allocator::SlotAllocator;
use synaptic_kernel::primitives::staging_buffer_reader::StagingBufferReader;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_allocator(capacity: u32) -> (SlotAllocator, StagingBufferReader, AtomicBuffer) {
    let size = SlotAllocator::calculate_size_on_mem(capacity as usize);
    let mem: AtomicBuffer = (0..size).map(|_| AtomicI32::new(0)).collect();
    let alloc = SlotAllocator::new(Arc::clone(&mem), 0, capacity);

    // We need a StagingBufferReader to simulate the reader acking generations
    let reader = alloc.to_staging_buffer_reader();
    (alloc, reader, mem)
}

// ============ State transitions: is_active / is_deferred / is_free ============

#[test]
fn fresh_allocator_all_slots_free() {
    let (alloc, _, _) = create_allocator(4);
    // Slots are 1-based
    for i in 1u32..=4 {
        let slot = SlotId::new(i).unwrap();
        assert!(alloc.is_free(slot), "slot {} should be free", i);
        assert!(!alloc.is_allocated(slot), "slot {} should not be allocated", i);
    }
}

#[test]
fn after_alloc_slot_is_active() {
    let (alloc, _, _) = create_allocator(4);
    let s = alloc.alloc().unwrap();

    assert!(alloc.is_allocated(s));
    assert!(alloc.is_active(s));
    assert!(!alloc.is_deferred(s));
    assert!(!alloc.is_free(s));
}

#[test]
fn after_defer_slot_is_deferred_not_active() {
    let (alloc, _, _) = create_allocator(4);
    let s = alloc.alloc().unwrap();

    alloc.defer_free(s).unwrap();

    assert!(
        alloc.is_allocated(s),
        "still allocated (hasn't been freed yet)"
    );
    assert!(alloc.is_deferred(s), "marked as deferred");
    assert!(
        !alloc.is_active(s),
        "not active (deferred takes precedence)"
    );
    assert!(!alloc.is_free(s), "not free (still in alloc bitmap)");
}

#[test]
fn after_publish_without_ack_slot_still_deferred() {
    let (alloc, _, _) = create_allocator(4);
    let s = alloc.alloc().unwrap();
    alloc.defer_free(s).unwrap();

    // Publish (increments generation), but no reader ack yet.
    alloc.publish();

    // Slot is still allocated (in use) and still deferred
    assert!(alloc.is_allocated(s));
    assert!(alloc.is_deferred(s));
}

#[test]
fn after_publish_and_ack_next_publish_frees_slot() {
    let (alloc, reader, _) = create_allocator(4);
    let s = alloc.alloc().unwrap();
    alloc.defer_free(s).unwrap();

    // Cycle 1: push and publish
    alloc.publish();

    // Reader acks the published generation
    reader.ack();

    // Cycle 2: next publish drains the acked items and returns them to free list
    alloc.publish();

    assert!(alloc.is_free(s), "slot should be fully free");
    assert!(!alloc.is_allocated(s));
    assert!(!alloc.is_deferred(s));
}

// ============ copy_from (growth scenario) ============

#[test]
fn copy_from_preserves_state_and_adds_capacity() {
    let (small, _, _) = create_allocator(4);

    let s1 = small.alloc().unwrap();
    let s2 = small.alloc().unwrap();
    let _s3 = small.alloc().unwrap();

    small.defer_free(s2).unwrap();

    assert_eq!(small.alloc_count(), 3);
    assert_eq!(small.deferred_count(), 1);

    // Create larger allocator and copy
    let large_size = SlotAllocator::calculate_size_on_mem(8);
    let large_mem: AtomicBuffer = (0..large_size).map(|_| AtomicI32::new(0)).collect();
    let large = SlotAllocator::new(Arc::clone(&large_mem), 0, 8);
    large.copy_from(&small);

    // State should be exactly reproduced
    assert_eq!(large.alloc_count(), 3, "allocations preserved");
    assert_eq!(large.deferred_count(), 1, "deferred count preserved");
    assert_eq!(large.capacity(), 8, "capacity is new");

    assert!(large.is_active(s1), "s1 is active");
    assert!(large.is_deferred(s2), "s2 is deferred");
    assert!(!large.is_free(s1));
    assert!(!large.is_free(s2));

    // Can allocate from remainder up to 8
    for _ in 0..5 {
        assert!(large.alloc().is_some());
    }
    // Now full
    assert!(large.alloc().is_none());
}

#[test]
fn copy_from_deferred_items_flush_correctly_on_destination() {
    let (small, _, _) = create_allocator(4);

    let s1 = small.alloc().unwrap();
    let s2 = small.alloc().unwrap();
    small.defer_free(s1).unwrap();
    small.defer_free(s2).unwrap();

    let large_size = SlotAllocator::calculate_size_on_mem(8);
    let large_mem: AtomicBuffer = (0..large_size).map(|_| AtomicI32::new(0)).collect();
    let large = SlotAllocator::new(Arc::clone(&large_mem), 0, 8);

    // Also bind a reader to the large
    let large_reader = large.to_staging_buffer_reader();

    large.copy_from(&small);

    // Publish to advance generation on destination
    large.publish();

    // Reader acks
    large_reader.ack();

    // Next publish drains and frees
    large.publish();

    assert_eq!(large.deferred_count(), 0);
    assert!(large.is_free(s1));
    assert!(large.is_free(s2));
}

#[test]
#[should_panic]
fn copy_from_panics_if_source_larger() {
    let (large, _, _) = create_allocator(8);
    let (small, _, _) = create_allocator(4);
    small.copy_from(&large);
}

// ============ Stress: rapid alloc/defer/flush cycles ============

#[test]
fn stress_alloc_defer_flush_cycles() {
    let (alloc, reader, _) = create_allocator(64);

    for _cycle in 0..50 {
        // Alloc some slots
        let mut slots = Vec::new();
        for _ in 0..16 {
            if let Some(s) = alloc.alloc() {
                slots.push(s);
            }
        }

        // Defer half
        for s in slots.iter().take(slots.len() / 2) {
            alloc.defer_free(*s).unwrap();
        }

        // Cycle 1: publish and ack
        alloc.publish();
        reader.ack();

        // Cycle 2: next publish un-defers and reclaims
        alloc.publish();

        // Invariant: free_count + alloc_count == capacity
        // Note: alloc_count inside the allocator includes deferred items.
        // Because we deferred and then fully reclaimed, the slots returned
        // to the free list are no longer counted as allocated.
        assert_eq!(
            alloc.free_count() + alloc.alloc_count(),
            64,
            "invariant violated in cycle {}",
            _cycle
        );

        // Free the remaining non-deferred slots
        for s in slots.iter().skip(slots.len() / 2) {
            alloc.defer_free(*s).unwrap();
        }

        alloc.publish();
        reader.ack();
        alloc.publish(); // Reclaims the remaining
    }

    // At the end, everything should be free
    assert_eq!(alloc.free_count(), 64);
    assert_eq!(alloc.alloc_count(), 0);
    assert_eq!(alloc.deferred_count(), 0);
}

#[test]
fn stress_interleaved_alloc_defer_with_partial_flush() {
    let (alloc, reader, _) = create_allocator(32);

    let mut active: Vec<SlotId> = Vec::new();

    for i in 0..200 {
        if active.len() < 32 && i % 3 != 0 {
            if let Some(s) = alloc.alloc() {
                active.push(s);
            }
        } else if !active.is_empty() {
            let s = active.remove(0);
            alloc.defer_free(s).unwrap();
        }

        // Periodically sequence a complete cycle
        if i % 3 == 0 {
            alloc.publish();
        }
        if i % 4 == 0 {
            reader.ack();
        }
    }

    // Final cleanup
    for s in active {
        alloc.defer_free(s).unwrap();
    }

    // We need up to 2 publish cycles with an ack to guarantee everything drains
    alloc.publish();
    reader.ack();
    alloc.publish();

    assert_eq!(alloc.free_count(), 32);
    assert_eq!(alloc.deferred_count(), 0);
}

// ============ Utilization ============

#[test]
fn utilization_tracks_allocation_ratio() {
    let (alloc, _, _) = create_allocator(4);
    assert_eq!(alloc.utilization(), 0.0);

    alloc.alloc().unwrap();
    assert_eq!(alloc.utilization(), 0.25);

    alloc.alloc().unwrap();
    assert_eq!(alloc.utilization(), 0.5);

    alloc.alloc().unwrap();
    alloc.alloc().unwrap();
    assert_eq!(alloc.utilization(), 1.0);
}

// ============ Bind ============

#[test]
fn bind_reads_existing_allocator_state() {
    let size = SlotAllocator::calculate_size_on_mem(4);
    let mem: AtomicBuffer = (0..size).map(|_| AtomicI32::new(0)).collect();

    let alloc1 = SlotAllocator::new(Arc::clone(&mem), 0, 4);
    let _s1 = alloc1.alloc().unwrap();
    let _s2 = alloc1.alloc().unwrap();
    assert_eq!(alloc1.alloc_count(), 2);

    let alloc2 = SlotAllocator::bind(Arc::clone(&mem), 0, 4);
    assert_eq!(alloc2.alloc_count(), 2);
    assert_eq!(alloc2.free_count(), 2);
}
