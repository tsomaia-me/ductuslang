use crate::primitives::triple_buffer_reader::TripleBufferReader;
use crate::primitives::types::AtomicBuffer;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Producer-side of an SPSC triple buffer backed by shared `AtomicBuffer`.
///
/// Three buffers rotate between producer, consumer and shared handoff.
/// The producer writes into a private buffer, then calls `publish()` to
/// atomically hand it off. After publish, the producer receives a stale buffer back and syncs
/// it from the published buffer via element-wise atomic copies.
///
/// # Threading
/// Producer thread only. `publish()` uses `AcqRel` on the state swap:
/// - `Release`: all buffer writes are visible before handoff.
/// - `Acquire`: the recycled buffer is fully released by the consumer before
///   the writer's `sync()` memcopy.
///
/// All other atomics use `Relaxed`.
///
/// # Memory Layout
/// ```text
/// Offset          Size            Field
/// -----------------------------------------------------
/// 0               1               state
/// 1               1               writer_buffer_id
/// 2               1               published_buffer_id
/// 3               1               reader_buffer_id
/// 4               3 * N           buffers
///
/// N = buffer_capacity
/// ```
///
/// # State Encoding
/// `state` packs two values into a single `i32`:
/// - bits 0-1: shared buffer ID (0, 1, or 2).
/// - bit 2: NEW_DATA flag (1 = unread data available)
///
/// # Initial State
/// - Producer owns buffer 0, consumer owns buffer 2, shared holds buffer 1
/// - `state` = `0b001` (buffer 1, no new data).
///
/// # Constraints
/// - `buffer_capacity` must be positive.
/// - Use `to_reader()` to create the paired `TripleBufferReader`.
///
#[derive(Clone)]
pub struct TripleBufferWriter {
    mem: AtomicBuffer,
    mem_start_offset: usize,
    mem_state_offset: usize,
    mem_writer_offset: usize,
    mem_published_offset: usize,
    mem_reader_offset: usize,
    buffer_bases: [usize; 3],
    buffer_capacity: usize,
    mem_end_offset: usize,
}

impl TripleBufferWriter {
    pub fn new(mem: AtomicBuffer, mem_start_offset: usize, buffer_capacity: u32) -> Self {
        TripleBufferWriter::create(mem, mem_start_offset, buffer_capacity, false)
    }

    pub fn bind(mem: AtomicBuffer, mem_start_offset: usize, buffer_capacity: u32) -> Self {
        TripleBufferWriter::create(mem, mem_start_offset, buffer_capacity, true)
    }

    pub fn create(
        mem: AtomicBuffer,
        mem_start_offset: usize,
        buffer_capacity: u32,
        bind: bool,
    ) -> Self {
        assert!(
            buffer_capacity > 0,
            "TripleBufferWriter::new | buffer_capacity {} must be positive",
            buffer_capacity
        );

        let mem_state_offset = mem_start_offset;
        let mem_writer_offset = mem_start_offset + 1;
        let mem_published_offset = mem_start_offset + 2;
        let mem_reader_offset = mem_start_offset + 3;
        let mem_buffers_base = mem_start_offset + 4;
        let buffer_bases: [usize; 3] = [
            mem_buffers_base,
            mem_buffers_base + buffer_capacity as usize,
            mem_buffers_base + buffer_capacity as usize * 2,
        ];
        let mem_end_offset = mem_buffers_base + buffer_capacity as usize * 3;

        assert!(
            mem_end_offset <= mem.len(),
            "TripleBufferWriter::new | range [{}..{}] exceeds AtomicBuffer boundaries",
            mem_start_offset,
            buffer_capacity * 3
        );

        let writer = TripleBufferWriter {
            mem: Arc::clone(&mem),
            mem_start_offset,
            mem_state_offset,
            mem_writer_offset,
            mem_reader_offset,
            mem_published_offset,
            buffer_bases,
            buffer_capacity: buffer_capacity as usize,
            mem_end_offset,
        };

        if !bind {
            mem[mem_writer_offset].store(0, Ordering::Relaxed);
            mem[mem_state_offset].store(0b001, Ordering::Relaxed);
            mem[mem_published_offset].store(0, Ordering::Relaxed);
            mem[mem_reader_offset].store(2, Ordering::Relaxed);
        } else {
            // Synchronize with the last publish() before reading its results.
            mem[mem_state_offset].load(Ordering::Acquire);
            let published_index = mem[mem_published_offset].load(Ordering::Relaxed);
            let writer_index = mem[mem_writer_offset].load(Ordering::Relaxed);
            writer.sync(published_index as usize, writer_index as usize);
        }

        writer
    }

    #[inline]
    pub fn calculate_size_on_mem(capacity: usize) -> usize {
        4 + capacity * 3
    }

    pub fn to_reader(&self) -> TripleBufferReader {
        TripleBufferReader::bind(
            Arc::clone(&self.mem),
            self.mem_start_offset,
            self.buffer_capacity,
        )
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
    pub fn mem_writer_base(&self) -> usize {
        let buffer_id = self.mem[self.mem_writer_offset].load(Ordering::Relaxed) as usize;
        self.buffer_bases[buffer_id]
    }

    pub fn publish(&self) {
        let current_id = self.mem[self.mem_writer_offset].load(Ordering::Relaxed);
        let new_state = (current_id & 0b011) | 0b100;

        // We use swap instead of CAS because of the following two reasons:
        // 1. the writer's new state is independent of the current shared state
        // - we unconditionally publish our buffer and set NEW_DATA.
        // 2. In SPSC, no competing writers exist, so swap is safe and retry-free.
        let old_state = self.mem[self.mem_state_offset].swap(new_state, Ordering::AcqRel);
        let writer_new_buffer_id = old_state & 0b011;

        self.mem[self.mem_writer_offset].store(writer_new_buffer_id, Ordering::Relaxed);
        self.mem[self.mem_published_offset].store(current_id, Ordering::Relaxed);
        self.sync(current_id as usize, writer_new_buffer_id as usize);
    }

    fn sync(&self, published_index: usize, writer_index: usize) {
        if published_index == writer_index {
            return;
        }

        let published_buffer_index = self.buffer_bases[published_index];
        let writer_buffer_index = self.buffer_bases[writer_index];

        for i in 0..self.buffer_capacity {
            self.mem[writer_buffer_index + i].store(
                self.mem[published_buffer_index + i].load(Ordering::Relaxed),
                Ordering::Relaxed,
            );
        }
    }

    #[inline]
    pub fn write(&self, offset: usize, value: i32) {
        debug_assert!(
            offset < self.buffer_capacity,
            "TripleBufferWriter.write | offset {} out of bounds",
            offset
        );
        let base = self.mem_writer_base();
        self.mem[base + offset].store(value, Ordering::Relaxed)
    }

    #[inline]
    pub fn read(&self, offset: usize) -> i32 {
        debug_assert!(
            offset < self.buffer_capacity,
            "TripleBufferWriter.read | offset {} out of bounds",
            offset
        );
        let base = self.mem_writer_base();
        self.mem[base + offset].load(Ordering::Relaxed)
    }

    #[inline]
    pub fn write_batch(&self, offset: usize, data: &[i32]) {
        debug_assert!(
            offset + data.len() <= self.buffer_capacity,
            "TripleBufferWriter.write_batch | [offset, out.len()) [{}, {}) out of bounds",
            offset,
            data.len(),
        );
        let base = self.mem_writer_base() + offset;

        for i in 0..data.len() {
            self.mem[base + i].store(data[i], Ordering::Relaxed)
        }
    }

    #[inline]
    pub fn read_batch(&self, offset: usize, out: &mut [i32]) {
        debug_assert!(
            offset + out.len() <= self.buffer_capacity,
            "TripleBufferWriter.read_batch | [offset, out.len()) [{}, {}) out of bounds",
            offset,
            out.len(),
        );
        let base = self.mem_writer_base() + offset;

        for i in 0..out.len() {
            out[i] = self.mem[base + i].load(Ordering::Relaxed)
        }
    }

    pub fn copy_metadata_from(&self, source: &TripleBufferWriter) {
        debug_assert!(
            source.buffer_capacity <= self.buffer_capacity,
            "TripleBufferWriter.copy_metadata_from | source.buffer_capacity {} cannot be greater than destination.buffer_capacity {}",
            source.buffer_capacity,
            self.buffer_capacity,
        );

        self.mem[self.mem_state_offset].store(
            source.mem[source.mem_state_offset].load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        self.mem[self.mem_writer_offset].store(
            source.mem[source.mem_writer_offset].load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        self.mem[self.mem_published_offset].store(
            source.mem[source.mem_published_offset].load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        self.mem[self.mem_reader_offset].store(
            source.mem[source.mem_reader_offset].load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
    }

    pub fn copy_region_from(
        &self,
        source: &TripleBufferWriter,
        source_offset: usize,
        destination_offset: usize,
        count: usize,
    ) {
        debug_assert!(
            source.buffer_capacity <= self.buffer_capacity,
            "TripleBufferWriter.copy_region_from | source.buffer_capacity {} cannot be greater than destination.buffer_capacity {}",
            source.buffer_capacity,
            self.buffer_capacity,
        );
        debug_assert!(
            destination_offset + count <= self.buffer_capacity,
            "TripleBufferWriter.copy_region_from | destination range [{}..{}] exceeds buffer_capacity {}",
            destination_offset,
            count,
            self.buffer_capacity,
        );
        debug_assert!(
            source_offset + count <= source.buffer_capacity,
            "TripleBufferWriter.copy_region_from | source range [{}..{}] exceeds buffer_capacity {}",
            source_offset,
            count,
            source.buffer_capacity,
        );

        for i in 0..3 {
            let self_base = self.buffer_bases[i] + destination_offset;
            let source_base = source.buffer_bases[i] + source_offset;
            for k in 0..count {
                self.mem[self_base + k].store(
                    source.mem[source_base + k].load(Ordering::Relaxed),
                    Ordering::Relaxed,
                );
            }
        }
    }
}
