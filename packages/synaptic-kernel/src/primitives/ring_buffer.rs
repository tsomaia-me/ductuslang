use crate::errors::ring_buffer_error::RingBufferError;
use crate::primitives::types::AtomicBuffer;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Fixed-capacity SPSC ring buffer with const-generic slot width.
///
/// Each entry is a `[i32; STRIDE]` array stored inline in the backing `AtomicBuffer`.
/// Uses bitmask (& (capacity-1)) for index wrapping instead of modulo.
///
/// # Threading
/// SPSC. One producer, one consumer. `pending_count` uses `Acquire`/`Release` to synchronize
/// visibility between `write()` and `read()`/`peek()`.
/// All other atomics use `Relaxed`.
///
/// # Memory Layout
/// ```text
/// Offset          Size            Field
/// ---------------------------------------------
/// 0               1               read_index
/// 1               1               write_index
/// 2               1               pending_count
/// 3               N * S           slots
///
/// N = capacity (power of 2)
/// S = STRIDE (const generic)
/// ```
///
/// # Constraints
/// - `capacity` must be a power of 2.
/// - `peek()` reads without advancing the read cursor.
/// - `read()` reads and advances.
#[derive(Clone)]
pub struct RingBuffer<const STRIDE: usize> {
    mem: AtomicBuffer,
    mod_mask: i32,
    capacity: usize,
    mem_start_offset: usize,
    mem_read_offset: usize,
    mem_write_offset: usize,
    mem_pending_offset: usize,
    mem_end_offset: usize,
}

impl<const STRIDE: usize> RingBuffer<STRIDE> {
    pub fn new(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        Self::create(mem, mem_start_offset, capacity, false)
    }

    pub fn bind(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        Self::create(mem, mem_start_offset, capacity, true)
    }

    pub fn create(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32, bind: bool) -> Self {
        let len = 3 + capacity as usize * STRIDE;
        let mem_end_offset = mem_start_offset + len;

        assert!(
            mem_end_offset <= mem.len(),
            "RingBuffer::create | range [{}..{}] exceeds AtomicBuffer boundaries",
            mem_start_offset,
            len
        );
        assert!(
            capacity > 0,
            "RingBuffer::create | capacity {} must be positive",
            capacity
        );
        assert_eq!(
            capacity & (capacity - 1),
            0,
            "RingBuffer::create | capacity {} must be power of 2",
            capacity
        );

        let mem_read_offset = mem_start_offset;
        let mem_write_offset = mem_start_offset + 1;
        let mem_pending_offset = mem_start_offset + 2;

        if !bind {
            for i in mem_start_offset..mem_end_offset {
                mem[i].store(0, Ordering::Relaxed);
            }
        }

        RingBuffer {
            mem: Arc::clone(&mem),
            capacity: capacity as usize,
            mod_mask: (capacity as i32) - 1,
            mem_read_offset,
            mem_write_offset,
            mem_pending_offset,
            mem_start_offset,
            mem_end_offset,
        }
    }

    pub fn calculate_size_on_mem(capacity: usize) -> usize {
        3 + capacity * STRIDE
    }

    pub fn pending_count(&self) -> usize {
        self.mem[self.mem_pending_offset].load(Ordering::Acquire) as usize
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

    pub fn peek(&self) -> Option<[i32; STRIDE]> {
        match self.retrieve() {
            Some((data, _)) => Some(data),
            None => None,
        }
    }

    pub fn read(&self) -> Option<[i32; STRIDE]> {
        match self.retrieve() {
            Some((data, read_offset)) => {
                self.mem[self.mem_read_offset]
                    .store((read_offset + 1) & self.mod_mask, Ordering::Relaxed);
                self.mem[self.mem_pending_offset].fetch_sub(1, Ordering::Release);
                Some(data)
            }
            None => None,
        }
    }

    pub fn write(&self, data: [i32; STRIDE]) -> Result<(), RingBufferError> {
        let pending_count = self.mem[self.mem_pending_offset].load(Ordering::Acquire) as usize;

        if pending_count >= self.capacity {
            return Err(RingBufferError::Full);
        }

        let write_index = self.mem[self.mem_write_offset].load(Ordering::Relaxed) as usize;
        let mem_slot_base = self.mem_start_offset + 3 + write_index * STRIDE;

        for i in 0..STRIDE {
            self.mem[mem_slot_base + i].store(data[i], Ordering::Relaxed)
        }

        self.mem[self.mem_write_offset]
            .store((write_index as i32 + 1) & self.mod_mask, Ordering::Relaxed);
        self.mem[self.mem_pending_offset].fetch_add(1, Ordering::Release);

        Ok(())
    }

    pub fn copy_from(&self, source: &Self) {
        debug_assert!(
            source.capacity <= self.capacity,
            "RingBuffer.copy_from | source.capacity {} cannot be greater than destination.capacity {}",
            source.capacity,
            self.capacity,
        );

        for i in 0..Self::calculate_size_on_mem(source.capacity) {
            self.mem[self.mem_start_offset + i].store(
                source.mem[source.mem_start_offset + i].load(Ordering::Relaxed),
                Ordering::Relaxed,
            );
        }
    }

    fn retrieve(&self) -> Option<([i32; STRIDE], i32)> {
        let pending_count = self.mem[self.mem_pending_offset].load(Ordering::Acquire);

        if pending_count == 0 {
            return None;
        }

        let read_index = self.mem[self.mem_read_offset].load(Ordering::Relaxed) as usize;

        let mut entry: [i32; STRIDE] = [0; STRIDE];
        let mem_slot_base = self.mem_start_offset + 3 + read_index * STRIDE;

        for i in 0..STRIDE {
            entry[i] = self.mem[mem_slot_base + i].load(Ordering::Relaxed)
        }

        Some((entry, read_index as i32))
    }
}
