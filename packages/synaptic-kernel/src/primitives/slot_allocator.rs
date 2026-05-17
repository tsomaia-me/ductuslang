use crate::errors::slot_allocator_error::SlotAllocatorError;
use crate::primitives::bitmap::Bitmap;
use crate::primitives::simple_free_list::SimpleFreeList;
use crate::primitives::slot::SlotId;
use crate::primitives::staging_buffer_reader::StagingBufferReader;
use crate::primitives::staging_buffer_writer::StagingBufferWriter;
use crate::primitives::types::AtomicBuffer;
use std::sync::Arc;

/// 1-based slot allocator with generation-gated deferred freeing.
///
/// Combines a `SimpleFreeList` for immediate allocation, a `StagingBuffer` for
/// deferred free() requests, and a `Bitmap` to track which slots are pending reclamation (retired).
/// Slots are not returned to the free list until `publish()` drains acknowledged entries from the
/// staging buffer.
///
/// # Threading
/// Producer-side only (Producer Thread).
/// Cross-thread coordination is handled by the underlying `StagingBuffer`'s generation protocol.
///
/// # Memory Layout
/// ```text
/// Offset              Size        Field
/// ------------------------------------
/// 0                   free(N)     staging_bitmap
/// bitmask(N)          free(N)     free_list
/// bitmask(N)+free(N)  staging(N)  staging_buffer
///
/// N           = capacity (power of 2)
/// bitmask(N)  = ceil(N/32)
/// free(N)     = 2 + ceil(N/32) + N
/// staging(N)  = 2 + 3 + 2N
/// ```
///
/// # Slot Lifecycle
/// `alloc()` -> active -> `defer_free()` -> deferred -> `publish()` -> free -> `alloc()`
///
/// # Constraints
/// - `capacity` must be a power of 2.
/// - 1-based slot API (same as `SimpleFreeList`).
/// - `defer_free()` on an unallocated or already-deferred slot returns an error.
#[derive(Clone)]
pub struct SlotAllocator {
    mem: AtomicBuffer,
    mem_start_offset: usize,
    mem_end_offset: usize,
    capacity: usize,
    staging_bitmap: Bitmap,
    free_list: SimpleFreeList,
    staging_buffer: StagingBufferWriter,
}

/**
 * SPSC Slot Allocator
 */
impl SlotAllocator {
    pub fn new(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        Self::create(mem, mem_start_offset, capacity, false)
    }

    pub fn bind(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        Self::create(mem, mem_start_offset, capacity, true)
    }

    pub fn create(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32, bind: bool) -> Self {
        let bitmap = Bitmap::create(Arc::clone(&mem), mem_start_offset, capacity, bind);
        let free_list =
            SimpleFreeList::create(Arc::clone(&mem), bitmap.mem_end_offset(), capacity, bind);
        let deferred_frees_list = StagingBufferWriter::create(
            Arc::clone(&mem),
            free_list.mem_end_offset(),
            capacity,
            bind,
        );
        let mem_end_offset = deferred_frees_list.mem_end_offset();

        assert!(
            mem_end_offset <= mem.len(),
            "SlotAllocator::create | range [{}..{}] exceeds AtomicBuffer boundaries",
            mem_start_offset,
            capacity,
        );

        SlotAllocator {
            mem,
            mem_start_offset,
            mem_end_offset,
            capacity: capacity as usize,
            staging_bitmap: bitmap,
            free_list,
            staging_buffer: deferred_frees_list,
        }
    }

    pub fn calculate_size_on_mem(capacity: usize) -> usize {
        Bitmap::calculate_size_on_mem(capacity)
            + SimpleFreeList::calculate_size_on_mem(capacity)
            + StagingBufferWriter::calculate_size_on_mem(capacity)
    }

    pub fn to_staging_buffer_reader(&self) -> StagingBufferReader {
        StagingBufferReader::bind(
            Arc::clone(&self.mem),
            self.staging_buffer.mem_start_offset(),
            self.capacity as u32,
        )
    }

    pub fn free_count(&self) -> usize {
        self.free_list.free_count()
    }

    pub fn deferred_count(&self) -> usize {
        self.staging_buffer.len()
    }

    pub fn alloc_count(&self) -> usize {
        self.free_list.alloc_count()
    }

    pub fn mem_start_offset(&self) -> usize {
        self.mem_start_offset
    }

    pub fn mem_end_offset(&self) -> usize {
        self.mem_end_offset
    }

    pub fn mem_staging_buffer_start_offset(&self) -> usize {
        self.staging_buffer.mem_start_offset()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn utilization(&self) -> f32 {
        self.free_list.utilization()
    }

    pub fn is_allocated(&self, slot: SlotId) -> bool {
        self.free_list.is_allocated(slot)
    }

    pub fn is_active(&self, slot: SlotId) -> bool {
        self.is_allocated(slot) && !self.is_deferred(slot)
    }

    pub fn is_deferred(&self, slot: SlotId) -> bool {
        self.staging_bitmap.is_on(slot.to_usize() - 1)
    }

    pub fn is_free(&self, slot: SlotId) -> bool {
        self.free_list.is_free(slot)
    }

    pub fn alloc(&self) -> Option<SlotId> {
        self.free_list.alloc()
    }

    pub fn defer_free(&self, slot: SlotId) -> Result<(), SlotAllocatorError> {
        if !self.is_allocated(slot) {
            return Err(SlotAllocatorError::InvalidSlot);
        }

        let slot_index = slot.to_usize() - 1;

        if self.staging_bitmap.is_on(slot_index) {
            return Err(SlotAllocatorError::DoubleFree);
        }

        self.staging_buffer.push(slot)?;
        self.staging_bitmap.on(slot_index);

        Ok(())
    }

    pub fn publish(&self) {
        for slot in self.staging_buffer.drain() {
            self.staging_bitmap.off(slot.to_usize() - 1);
            let result = self.free_list.free(slot);
            debug_assert!(
                result.is_ok(),
                "SlotAllocator.flush_deferred | internal invariant violated: double free during flush"
            )
        }

        self.staging_buffer.publish()
    }

    pub fn copy_from(&self, source: &SlotAllocator) {
        debug_assert!(
            source.capacity <= self.capacity,
            "SlotAllocator.copy_from | source.capacity {} cannot be greater than destination.capacity {}",
            source.capacity,
            self.capacity,
        );
        self.staging_bitmap.copy_from(&source.staging_bitmap);
        self.free_list.copy_from(&source.free_list);
        self.staging_buffer.copy_from(&source.staging_buffer);
    }
}
