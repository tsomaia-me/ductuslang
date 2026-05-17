use crate::primitives::staging_buffer_writer::StagingBufferWriter;
use crate::primitives::types::AtomicBuffer;
use std::sync::atomic::Ordering;

/// Consumer-side handle for the generation-gated staging buffer.
///
/// Does not read entries from the ring buffer directly. Its sole purpose is to call `ack()`,
/// which advances `reader_ack_generation` so the writer's `drain()` knows which entries are
/// safe to reclaim.
///
/// # Threading
/// Consumer thread only. `ack()` stores with `Release` ordering to synchronize
/// against the writer's `Acquire` read in `StagingBufferWriter::drain()`.
///
/// # Memory Layout
/// Shares the same backing region as `StagingBufferWriter`. See its layout.
///
/// # Constraints
/// - Created exclusively via `StagingBufferWriter::to_reader()`.
#[derive(Clone)]
pub struct StagingBufferReader {
    mem: AtomicBuffer,
    capacity: usize,
    mem_start_offset: usize,
    mem_writer_generation_offset: usize,
    mem_reader_ack_generation_offset: usize,
    mem_end_offset: usize,
}

impl StagingBufferReader {
    pub(crate) fn bind(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        assert!(
            capacity > 0,
            "StagingBufferReader::bind | capacity {} must be positive",
            capacity
        );
        assert_eq!(
            capacity & (capacity - 1),
            0,
            "StagingBufferReader::bind | capacity {} must be power of 2",
            capacity
        );

        let mem_writer_generation_offset = mem_start_offset;
        let mem_reader_ack_generation_offset = mem_start_offset + 1;
        let mem_end_offset =
            mem_start_offset + StagingBufferWriter::calculate_size_on_mem(capacity as usize);

        debug_assert!(
            mem_end_offset <= mem.len(),
            "StagingBufferReader::bind | range [{}..{}] exceeds AtomicBuffer boundaries",
            mem_start_offset,
            mem.len()
        );

        StagingBufferReader {
            mem,
            mem_start_offset,
            mem_writer_generation_offset,
            mem_reader_ack_generation_offset,
            mem_end_offset,
            capacity: capacity as usize,
        }
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

    pub fn writer_generation(&self) -> usize {
        self.mem[self.mem_writer_generation_offset].load(Ordering::Relaxed) as usize
    }

    pub fn ack(&self) {
        self.mem[self.mem_reader_ack_generation_offset]
            .store(self.writer_generation() as i32 - 1, Ordering::Release);
    }
}
