use crate::primitives::types::AtomicBuffer;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Consumer-side of an SPSC triple buffer backed by shared `AtomicBuffer`.
///
/// Calls `swap()` to atomically acquire the latest published buffer.
/// Returns `false` if no new data is available (NEW_DATA flag is 0).
///
/// # Threading
/// Consumer thread only. `swap()` loads state with `Acquire`, then `AcqRel` on the state swap:
/// - `Acquire`: ensures the writer's writes to the buffer are visible before the reader accesses it.
/// - `Release`: finishes all reads before handing the buffer back to the writer.
///
/// All other atomics use `Relaxed`.
///
/// # Memory Layout
/// Shares backing region with `TripleBufferWriter`. See its layout.
///
/// # Constraints
/// - Created exclusively via `TripleBufferWriter::to_reader()`.
#[derive(Clone)]
pub struct TripleBufferReader {
    mem: AtomicBuffer,
    mem_start_offset: usize,
    mem_state_offset: usize,
    mem_reader_offset: usize,
    buffer_bases: [usize; 3],
    buffer_capacity: usize,
    mem_end_offset: usize,
}

impl TripleBufferReader {
    pub(crate) fn bind(mem: AtomicBuffer, mem_start_offset: usize, buffer_capacity: usize) -> Self {
        assert!(
            buffer_capacity > 0,
            "TripleBufferReader::bind | buffer_capacity {} must be positive",
            buffer_capacity
        );

        let mem_state_offset = mem_start_offset;
        let mem_reader_offset = mem_start_offset + 3;
        let mem_buffers_base = mem_start_offset + 4;
        let buffer_bases: [usize; 3] = [
            mem_buffers_base,
            mem_buffers_base + buffer_capacity,
            mem_buffers_base + buffer_capacity * 2,
        ];
        let mem_end_offset = mem_buffers_base + buffer_capacity * 3;

        TripleBufferReader {
            mem: Arc::clone(&mem),
            mem_start_offset,
            mem_state_offset,
            mem_reader_offset,
            buffer_bases,
            buffer_capacity,
            mem_end_offset,
        }
    }

    #[inline]
    pub fn buffer_capacity(&self) -> usize {
        self.buffer_capacity
    }

    #[inline]
    pub fn mem_start_offset(&self) -> usize {
        self.mem_start_offset
    }

    #[inline]
    pub fn mem_end_offset(&self) -> usize {
        self.mem_end_offset
    }

    #[inline]
    pub fn mem_reader_base(&self) -> usize {
        let buffer_id = self.mem[self.mem_reader_offset].load(Ordering::Relaxed) as usize;
        self.buffer_bases[buffer_id]
    }

    pub fn swap(&self) -> bool {
        let state = self.mem[self.mem_state_offset].load(Ordering::Acquire);

        if state & 0b100 == 0 {
            return false;
        }

        let current_id = self.mem[self.mem_reader_offset].load(Ordering::Relaxed);
        let new_state = current_id & 0b011;

        // We use swap instead of CAS because of the following two reasons:
        // 1. the reader's new state is independent of the current shared state.
        // 2. In SPSC, only the reader clears NEW_DATA, so it cannot go 1->0 between
        // the load() above and this swap().
        // The old_state is used to determine which buffer was acquired, since
        // state loaded by the initial load() might be stale by the time we reach this point.
        let old_state = self.mem[self.mem_state_offset].swap(new_state, Ordering::AcqRel);

        self.mem[self.mem_reader_offset].store(old_state & 0b011, Ordering::Relaxed);

        true
    }

    #[inline]
    pub fn read(&self, offset: usize) -> i32 {
        debug_assert!(
            offset < self.buffer_capacity,
            "TripleBufferReader.read | offset {} out of bounds",
            offset
        );
        let base = self.mem_reader_base();
        self.mem[base + offset].load(Ordering::Relaxed)
    }

    #[inline]
    pub fn read_batch(&self, offset: usize, out: &mut [i32]) {
        debug_assert!(
            offset + out.len() <= self.buffer_capacity,
            "TripleBufferReader.read_batch | [offset, out.len()) [{}, {}) out of bounds",
            offset,
            out.len(),
        );
        let base = self.mem_reader_base() + offset;

        for i in 0..out.len() {
            out[i] = self.mem[base + i].load(Ordering::Relaxed)
        }
    }
}
