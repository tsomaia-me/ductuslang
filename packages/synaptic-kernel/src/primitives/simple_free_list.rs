use crate::errors::free_list_error::FreeListError;
use crate::primitives::bitmap::Bitmap;
use crate::primitives::slot::SlotId;
use crate::primitives::types::AtomicBuffer;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// 1-based free list backed by a shared `AtomicBuffer`.
///
/// Manages a fixed pool of reusable slot numbers using a singly-linked list embedded in the buffer.
/// Uses Bitmap to guard against double-frees.
///
/// # Threading
/// Single-threaded only. All atomic operations use `Relaxed` ordering
///
/// # Memory Layout
/// ```text
/// Offset          Size            Field
/// ------------------------------------------
/// 0               1               head
/// 1               1               free_count
/// 2               ceil(N/32)      bitmap
/// 2+ceil(N/32)    N               slots
///
/// N = capacity (power of 2)
/// ```
///
/// # Constraints
/// - `capacity` must be a power of 2.
/// - The API is 1-based: `alloc()` returns slots starting at 1,
///   `free()` accepts the same. Internally 0-based.
#[derive(Clone)]
pub struct SimpleFreeList {
    mem: AtomicBuffer,
    mem_start_offset: usize,
    alloc_bitmap: Bitmap,
    slots_start_index: usize,
    mem_head_offset: usize,
    mem_free_count_offset: usize,
    capacity: usize,
    mem_end_offset: usize,
}

impl SimpleFreeList {
    pub fn new(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        Self::create(mem, mem_start_offset, capacity, false)
    }

    pub fn bind(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        Self::create(mem, mem_start_offset, capacity, true)
    }

    pub fn create(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32, bind: bool) -> Self {
        assert!(
            capacity > 0,
            "SimpleFreeList::create | capacity {} must be positive",
            capacity
        );
        assert_eq!(
            capacity & (capacity - 1),
            0,
            "SimpleFreeList::create | capacity {} must be power of 2",
            capacity
        );

        let free_count_slot_index = mem_start_offset + 1;
        let alloc_bitmap = Bitmap::create(Arc::clone(&mem), mem_start_offset + 2, capacity, bind);
        let slots_start_index = alloc_bitmap.mem_end_offset();
        let slots_end_index = slots_start_index + capacity as usize;

        assert!(slots_end_index <= mem.len(), "SimpleFreeList out of bounds");

        if !bind {
            for i in 0..capacity as usize {
                mem[slots_start_index + i].store((i as i32) + 1, Ordering::Relaxed);
            }

            mem[mem_start_offset].store(0, Ordering::Relaxed);
            mem[free_count_slot_index].store(capacity as i32, Ordering::Relaxed);
        }

        SimpleFreeList {
            mem: Arc::clone(&mem),
            mem_start_offset,
            alloc_bitmap,
            mem_head_offset: mem_start_offset,
            mem_free_count_offset: free_count_slot_index,
            slots_start_index,
            mem_end_offset: slots_end_index,
            capacity: capacity as usize,
        }
    }

    pub fn calculate_size_on_mem(capacity: usize) -> usize {
        2 + Bitmap::calculate_size_on_mem(capacity) + capacity
    }

    pub fn free_count(&self) -> usize {
        self.mem[self.mem_free_count_offset].load(Ordering::Relaxed) as usize
    }

    pub fn alloc_count(&self) -> usize {
        self.capacity - self.free_count()
    }

    pub fn mem_start_offset(&self) -> usize {
        self.mem_start_offset
    }

    pub fn mem_end_offset(&self) -> usize {
        self.mem_end_offset
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn utilization(&self) -> f32 {
        self.alloc_count() as f32 / self.capacity as f32
    }

    pub fn is_allocated(&self, slot: SlotId) -> bool {
        self.alloc_bitmap.is_on(slot.to_usize() - 1)
    }

    pub fn is_free(&self, slot: SlotId) -> bool {
        self.alloc_bitmap.is_off(slot.to_usize() - 1)
    }

    pub fn alloc(&self) -> Option<SlotId> {
        let head_index = self.mem[self.mem_head_offset].load(Ordering::Relaxed);

        if head_index >= self.capacity as i32 {
            return None;
        }

        let slot_index = head_index as u32;
        let next_index =
            self.mem[self.slots_start_index + head_index as usize].load(Ordering::Relaxed);
        self.mem[self.mem_head_offset].store(next_index, Ordering::Relaxed);
        self.mem[self.mem_free_count_offset].fetch_sub(1, Ordering::Relaxed);

        self.alloc_bitmap.on(slot_index as usize);

        SlotId::new(slot_index + 1)
    }

    pub fn free(&self, slot: SlotId) -> Result<(), FreeListError> {
        let slot_index = slot.to_usize() - 1;
        debug_assert!(
            slot_index < self.capacity as usize,
            "SimpleFreeList.free | slot_number {} out of bounds",
            slot
        );

        if self.alloc_bitmap.is_off(slot_index) {
            return Err(FreeListError::DoubleFree);
        }

        let head_index = self.mem[self.mem_head_offset].load(Ordering::Relaxed);
        self.mem[self.slots_start_index + slot_index].store(head_index, Ordering::Relaxed);
        self.mem[self.mem_head_offset].store(slot_index as i32, Ordering::Relaxed);
        self.mem[self.mem_free_count_offset].fetch_add(1, Ordering::Relaxed);
        self.alloc_bitmap.off(slot_index);

        Ok(())
    }

    pub fn copy_from(&self, source: &SimpleFreeList) {
        debug_assert!(
            source.capacity <= self.capacity,
            "SimpleFreeList.copy_from | source.capacity {} cannot be greater than destination.capacity {}",
            source.capacity,
            self.capacity,
        );

        let additional_capacity = (self.capacity() - source.capacity) as i32;
        let old_free_count = source.mem[source.mem_free_count_offset].load(Ordering::Relaxed);

        self.mem[self.mem_head_offset].store(
            source.mem[source.mem_head_offset].load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        self.mem[self.mem_free_count_offset]
            .store(old_free_count + additional_capacity, Ordering::Relaxed);

        self.alloc_bitmap.copy_from(&source.alloc_bitmap);

        for i in 0..source.capacity as usize {
            self.mem[self.slots_start_index + i].store(
                source.mem[source.slots_start_index + i].load(Ordering::Relaxed),
                Ordering::Relaxed,
            )
        }
    }
}
