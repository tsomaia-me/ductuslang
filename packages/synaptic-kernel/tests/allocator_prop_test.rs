use proptest::prelude::*;
use std::sync::atomic::AtomicI32;
use synaptic_kernel::primitives::simple_free_list::SimpleFreeList;
use synaptic_kernel::primitives::slot::SlotId;
use synaptic_kernel::primitives::slot_allocator::SlotAllocator;
use synaptic_kernel::primitives::types::AtomicBuffer;

fn create_mem(size: usize) -> AtomicBuffer {
    (0..size).map(|_| AtomicI32::new(0)).collect()
}

// ============ SimpleFreeList Property Tests ============

/// Operation enum for property-based testing of free list
#[derive(Debug, Clone)]
enum FreeListOp {
    Alloc,
    Free(usize), // index into active slots
}

fn free_list_op_strategy() -> impl Strategy<Value = FreeListOp> {
    prop_oneof![
        2 => Just(FreeListOp::Alloc),
        1 => (0..64usize).prop_map(FreeListOp::Free),
    ]
}

proptest! {
    #[test]
    fn free_list_invariant_free_plus_alloc_equals_capacity(
        ops in proptest::collection::vec(free_list_op_strategy(), 1..200)
    ) {
        let capacity: u32 = 32;
        let mem = create_mem(65536);
        let fl = SimpleFreeList::new(mem, 0, capacity);
        let mut active: Vec<SlotId> = Vec::new();

        for op in ops {
            match op {
                FreeListOp::Alloc => {
                    if let Some(slot) = fl.alloc() {
                        active.push(slot);
                    }
                }
                FreeListOp::Free(idx) => {
                    if !active.is_empty() {
                        let actual_idx = idx % active.len();
                        let slot = active.remove(actual_idx);
                        fl.free(slot).unwrap();
                    }
                }
            }

            // INVARIANT: free_count + active.len() == capacity
            prop_assert_eq!(
                fl.free_count() + active.len(),
                capacity as usize,
                "invariant violated: free={} active={} capacity={}",
                fl.free_count(), active.len(), capacity
            );
        }
    }

    #[test]
    fn free_list_all_allocated_slots_are_unique(
        alloc_count in 1..64usize
    ) {
        let capacity: u32 = 64;
        let mem = create_mem(65536);
        let fl = SimpleFreeList::new(mem, 0, capacity);

        let mut slots = Vec::new();
        for _ in 0..alloc_count {
            if let Some(s) = fl.alloc() {
                slots.push(s);
            }
        }

        // All slots should be unique
        let mut sorted = slots.clone();
        sorted.sort();
        sorted.dedup();
        prop_assert_eq!(sorted.len(), slots.len(), "duplicate slots allocated");

        // All should be in valid range [1, capacity]
        for s in &slots {
            prop_assert!(s.get() >= 1 && s.get() <= capacity, "slot {} out of range", s);
        }
    }

    #[test]
    fn free_list_no_double_alloc_without_free(
        ops in proptest::collection::vec(free_list_op_strategy(), 1..300)
    ) {
        let capacity: u32 = 16;
        let mem = create_mem(65536);
        let fl = SimpleFreeList::new(mem, 0, capacity);
        let mut active: std::collections::HashSet<SlotId> = std::collections::HashSet::new();

        for op in ops {
            match op {
                FreeListOp::Alloc => {
                    if let Some(slot) = fl.alloc() {
                        // Slot must NOT already be in active set
                        prop_assert!(
                            active.insert(slot),
                            "slot {} was allocated twice without being freed", slot
                        );
                    }
                }
                FreeListOp::Free(idx) => {
                    if !active.is_empty() {
                        let slots_vec: Vec<SlotId> = active.iter().cloned().collect();
                        let actual_idx = idx % slots_vec.len();
                        let slot = slots_vec[actual_idx];
                        active.remove(&slot);
                        fl.free(slot).unwrap();
                    }
                }
            }
        }
    }
}

// ============ SlotAllocator Property Tests ============

#[derive(Debug, Clone)]
enum AllocatorOp {
    Alloc,
    DeferFree(usize),
    Flush,
}

fn allocator_op_strategy() -> impl Strategy<Value = AllocatorOp> {
    prop_oneof![
        3 => Just(AllocatorOp::Alloc),
        2 => (0..64usize).prop_map(AllocatorOp::DeferFree),
        1 => Just(AllocatorOp::Flush),
    ]
}

proptest! {
    #[test]
    fn allocator_invariant_counts_are_consistent(
        ops in proptest::collection::vec(allocator_op_strategy(), 1..200)
    ) {
        let capacity: u32 = 32;
        let size = SlotAllocator::calculate_size_on_mem(capacity as usize);
        let mem = create_mem(size);
        let alloc = SlotAllocator::new(mem, 0, capacity);
        let mut active: Vec<SlotId> = Vec::new();

        for op in ops {
            match op {
                AllocatorOp::Alloc => {
                    if let Some(slot) = alloc.alloc() {
                        active.push(slot);
                    }
                }
                AllocatorOp::DeferFree(idx) => {
                    if !active.is_empty() {
                        let actual_idx = idx % active.len();
                        let slot = active.remove(actual_idx);
                        let _ = alloc.defer_free(slot);
                    }
                }
                AllocatorOp::Flush => {
                    alloc.publish();
                }
            }

            // INVARIANT: free_count + alloc_count == capacity
            prop_assert_eq!(
                alloc.free_count() + alloc.alloc_count(),
                capacity as usize,
                "invariant violated: free={} alloc={} capacity={}",
                alloc.free_count(), alloc.alloc_count(), capacity
            );
        }
    }

    #[test]
    fn allocator_active_slots_are_unique(
        ops in proptest::collection::vec(allocator_op_strategy(), 1..200)
    ) {
        let capacity: u32 = 32;
        let size = SlotAllocator::calculate_size_on_mem(capacity as usize);
        let mem = create_mem(size);
        let alloc = SlotAllocator::new(mem, 0, capacity);
        let mut active: std::collections::HashSet<SlotId> = std::collections::HashSet::new();

        for op in ops {
            match op {
                AllocatorOp::Alloc => {
                    if let Some(slot) = alloc.alloc() {
                        prop_assert!(
                            active.insert(slot),
                            "slot {} allocated while still active", slot
                        );
                    }
                }
                AllocatorOp::DeferFree(idx) => {
                    if !active.is_empty() {
                        let slots_vec: Vec<SlotId> = active.iter().cloned().collect();
                        let actual_idx = idx % slots_vec.len();
                        let slot = slots_vec[actual_idx];
                        active.remove(&slot);
                        let _ = alloc.defer_free(slot);
                    }
                }
                AllocatorOp::Flush => {
                    alloc.publish();
                }
            }
        }
    }
}
