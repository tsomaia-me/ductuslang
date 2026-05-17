use crate::primitives::types::AtomicBuffer;
use std::sync::atomic::Ordering;

/// Bit-packed boolean array backed by a shared `AtomicBuffer`.
///
/// Packs 32 flags per `i32` word. Uses bitwise shifts and masks for (1) access
/// to individual bits.
///
/// # Threading
/// Single-threaded only. All atomic operations use `Relaxed` ordering.
///
/// # Memory Layout
/// ```text
/// Offset      Size            Field
/// ---------------------------------
/// 0           ceil(N/32)      words
///
/// N = capacity (power of 2)
/// ```
///
/// # Constraints
/// - `capacity` must be a power of 2.
/// - 0-based bit indexing.
#[derive(Clone)]
pub struct Bitmap {
    mem: AtomicBuffer,
    mem_start_offset: usize,
    mem_end_offset: usize,
    capacity: usize,
    word_count: usize,
}

impl Bitmap {
    pub fn new(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        Self::create(mem, mem_start_offset, capacity, false)
    }

    pub fn bind(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        Self::create(mem, mem_start_offset, capacity, true)
    }

    pub fn create(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32, bind: bool) -> Self {
        assert!(
            capacity > 0,
            "Bitmap::create | capacity {} must be positive",
            capacity
        );
        assert_eq!(
            capacity & (capacity - 1),
            0,
            "Bitmap::create | capacity {} must be power of 2",
            capacity
        );
        let word_count = Self::calculate_size_on_mem(capacity as usize);
        let mem_end_offset = mem_start_offset + word_count;

        if !bind {
            for i in mem_start_offset..mem_end_offset {
                mem[i].store(0, Ordering::Relaxed);
            }
        }

        Bitmap {
            mem,
            mem_start_offset,
            mem_end_offset,
            capacity: capacity as usize,
            word_count,
        }
    }

    pub fn calculate_size_on_mem(capacity: usize) -> usize {
        (capacity + 31) / 32
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

    pub fn word_count(&self) -> usize {
        self.word_count
    }

    pub fn is_off(&self, bit_offset: usize) -> bool {
        debug_assert!(
            bit_offset < self.capacity,
            "Bitmap.is_off | bit_offset {} out of bounds",
            bit_offset
        );
        let bitmask = self.mem[self.mem_start_offset + (bit_offset >> 5)].load(Ordering::Relaxed);
        bitmask & (1 << (bit_offset & 31)) == 0
    }

    pub fn is_on(&self, bit_offset: usize) -> bool {
        debug_assert!(
            bit_offset < self.capacity,
            "Bitmap.is_on | bit_offset {} out of bounds",
            bit_offset
        );
        !self.is_off(bit_offset)
    }

    pub fn on(&self, bit_offset: usize) {
        debug_assert!(
            bit_offset < self.capacity,
            "Bitmap.on | bit_offset {} out of bounds",
            bit_offset
        );
        self.mem[self.mem_start_offset + (bit_offset >> 5)]
            .fetch_or(1 << (bit_offset & 31), Ordering::Relaxed);
    }

    pub fn off(&self, bit_offset: usize) {
        debug_assert!(
            bit_offset < self.capacity,
            "Bitmap.off | bit_offset {} out of bounds",
            bit_offset
        );
        self.mem[self.mem_start_offset + (bit_offset >> 5)]
            .fetch_and(!(1 << (bit_offset & 31)), Ordering::Relaxed);
    }

    pub fn clear(&self) {
        for i in self.mem_start_offset..self.mem_end_offset {
            self.mem[i].store(0, Ordering::Relaxed);
        }
    }

    pub fn copy_from(&self, source: &Bitmap) {
        debug_assert!(
            source.capacity <= self.capacity,
            "Bitmap.copy_from | source.capacity {} cannot be greater than destination.capacity {}",
            source.capacity,
            self.capacity,
        );

        for i in 0..source.word_count {
            self.mem[self.mem_start_offset + i].store(
                source.mem[source.mem_start_offset + i].load(Ordering::Relaxed),
                Ordering::Relaxed,
            )
        }
    }
}
