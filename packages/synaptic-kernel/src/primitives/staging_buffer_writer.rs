use crate::errors::ring_buffer_error::RingBufferError;
use crate::primitives::ring_buffer::RingBuffer;
use crate::primitives::slot::SlotId;
use crate::primitives::staging_buffer_reader::StagingBufferReader;
use crate::primitives::types::AtomicBuffer;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Generation-gated SPSC staging buffer for deferred slot reclamation.
///
/// The producer pushes retired slot numbers into a ring buffer,
/// each stamped with the current `writer_generation`. On `publish()`, the generation advances.
/// `drain()` only yields entries whose generation has been acknowledged by the reader
/// via `StagingBufferReader::ack()`.
///
/// # Threading
/// Producer-side only (Producer Thread).
/// Reads `reader_ack_generation` with `Acquire`
/// to synchronize against the reader's `Release` in `ack()`.
/// All other atomics use `Relaxed`.
///
/// # Memory Layout
/// ```text
/// Offset          Size            Field
/// -----------------------------------------------------
/// 0               1               writer_generation
/// 1               1               reader_ack_generation
/// 2               ring_size(N)    ring_buffer
///
/// N = capacity (power of 2)
/// ring_size(N) = 3+2N (= `RingBuffer::<2>::calculate_size_on_mem(N)`)
/// ```
///
/// # Constraints
/// - `capacity` must be a power of 2.
/// - `writer_generation` starts at 1, `reader_ack_generation` starts at 0.
///   This initial differential prevents premature draining of pre-publish entries.
/// - Use `to_reader()` to create the paired `StagingBufferReader`.
#[derive(Clone)]
pub struct StagingBufferWriter {
    mem: AtomicBuffer,
    buffer: RingBuffer<2>,
    capacity: usize,
    mem_start_offset: usize,
    mem_writer_generation_offset: usize,
    mem_reader_ack_generation_offset: usize,
    mem_end_offset: usize,
}

pub struct StagingBufferWriterIterator<'a> {
    buffer: &'a RingBuffer<2>,
    ack_generation: i32,
}

impl<'a> Iterator for StagingBufferWriterIterator<'a> {
    type Item = SlotId;

    fn next(&mut self) -> Option<Self::Item> {
        match self.buffer.peek() {
            Some([data, generation]) => {
                if generation.wrapping_sub(self.ack_generation) > 0 {
                    return None;
                }

                self.buffer.read();
                SlotId::new(data as u32)
            }
            None => None,
        }
    }
}

impl StagingBufferWriter {
    pub fn new(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        Self::create(mem, mem_start_offset, capacity, false)
    }

    pub fn bind(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32) -> Self {
        Self::create(mem, mem_start_offset, capacity, true)
    }

    pub fn create(mem: AtomicBuffer, mem_start_offset: usize, capacity: u32, bind: bool) -> Self {
        assert!(
            capacity > 0,
            "StagingBufferWriter::create | capacity {} must be positive",
            capacity
        );
        assert_eq!(
            capacity & (capacity - 1),
            0,
            "StagingBufferWriter::create | capacity {} must be power of 2",
            capacity
        );

        let mem_writer_generation_offset = mem_start_offset;
        let mem_reader_ack_generation_offset = mem_start_offset + 1;
        let mem_list_start_offset = mem_start_offset + 2;
        let mem_end_offset =
            mem_list_start_offset + RingBuffer::<2>::calculate_size_on_mem(capacity as usize);

        assert!(
            mem_end_offset <= mem.len(),
            "StagingBufferWriter::create | range [{}..{}] exceeds AtomicBuffer boundaries",
            mem_start_offset,
            mem.len()
        );

        if !bind {
            mem[mem_writer_generation_offset].store(1, Ordering::Relaxed);
            mem[mem_reader_ack_generation_offset].store(0, Ordering::Relaxed);
        }

        let buffer =
            RingBuffer::<2>::create(Arc::clone(&mem), mem_list_start_offset, capacity, bind);

        StagingBufferWriter {
            mem,
            buffer,
            mem_start_offset,
            mem_writer_generation_offset,
            mem_reader_ack_generation_offset,
            mem_end_offset,
            capacity: capacity as usize,
        }
    }

    pub fn calculate_size_on_mem(capacity: usize) -> usize {
        2 + RingBuffer::<2>::calculate_size_on_mem(capacity)
    }

    pub fn to_reader(&self) -> StagingBufferReader {
        StagingBufferReader::bind(
            Arc::clone(&self.mem),
            self.mem_start_offset,
            self.capacity as u32,
        )
    }

    pub fn len(&self) -> usize {
        self.buffer.pending_count()
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

    pub fn writer_generation(&self) -> i32 {
        self.mem[self.mem_writer_generation_offset].load(Ordering::Relaxed)
    }

    pub fn reader_ack_generation(&self) -> i32 {
        self.mem[self.mem_reader_ack_generation_offset].load(Ordering::Acquire)
    }

    pub fn push(&self, slot: SlotId) -> Result<(), RingBufferError> {
        let len = self.len();

        debug_assert!(
            len < self.capacity,
            "StagingBufferWriter.push | buffer overflow",
        );

        // Relaxed: same reasoning as in `publish()` below: loom-verified.
        let generation_id = self.mem[self.mem_writer_generation_offset].load(Ordering::Relaxed);

        self.buffer.write([slot.to_i32(), generation_id])?;

        Ok(())
    }

    // `Relaxed` on the writer_generation read/write is intentional and
    // loom-verified (see tests/loom_staging_buffer.rs). Cross-thread
    // synchronization between writer and reader rides on the
    // reader_ack_generation Release (in StagingBufferReader::ack) /
    // Acquire (in `reader_ack_generation()` above) pair. The
    // writer_generation cell itself is a scalar i32 with no associated
    // payload that must become visible alongside it - torn reads cannot
    // occur, and a stale read just delays a drain by one cycle.
    // No need to be "tightened" to AcqRel: the cost is real, the benefit is zero.
    pub fn publish(&self) {
        self.mem[self.mem_writer_generation_offset].fetch_add(1, Ordering::Relaxed);
    }

    pub fn drain(&'_ self) -> StagingBufferWriterIterator<'_> {
        StagingBufferWriterIterator {
            buffer: &self.buffer,
            ack_generation: self.reader_ack_generation(),
        }
    }

    pub fn copy_from(&self, source: &Self) {
        debug_assert!(
            source.capacity <= self.capacity,
            "StagingBufferWriter.copy_from | source.capacity {} cannot be greater than destination.capacity {}",
            source.capacity,
            self.capacity,
        );

        self.mem[self.mem_writer_generation_offset].store(
            source.mem[source.mem_writer_generation_offset].load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        self.mem[self.mem_reader_ack_generation_offset].store(
            source.mem[source.mem_reader_ack_generation_offset].load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        self.buffer.copy_from(&source.buffer);
    }
}
