use crate::metadata::mem_metadata_reader::MemMetadataReader;
use crate::primitives::types::AtomicBuffer;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Producer-side global metadata storage backed by a shared `AtomicBuffer`.
///
/// Provides a flat, power-of-2 sized array of `i32` slots for graph-level configuration
/// and/or statistics. Lives on the `mem` (direct) plane (non-triple-buffered), meaning
/// writes are immediately visible to the reader without requiring a `publish()`.
///
/// # Threading
/// Producer thread only. All atomic operations use `Relaxed` ordering.
///
/// # Memory Layout
/// ```text
/// Offset      Size        Field
/// -------------------------------------
/// 0           N           metadat_region
///
/// N = capacity (power of 2)
/// ```
///
/// # Constraints
/// - 0-based offset indexing.
/// - Use `to_reader()` to create the paired `MemMetadataReader`.
#[derive(Clone)]
pub struct MemMetadataWriter {
    mem: AtomicBuffer,
    mem_start_offset: usize,
    mem_end_offset: usize,
    capacity: usize,
}

impl MemMetadataWriter {
    pub fn new(mem: AtomicBuffer, mem_start_offset: usize, capacity: usize) -> Self {
        Self::create(mem, mem_start_offset, capacity, false)
    }

    pub fn bind(mem: AtomicBuffer, mem_start_offset: usize, capacity: usize) -> Self {
        Self::create(mem, mem_start_offset, capacity, true)
    }

    pub fn create(mem: AtomicBuffer, mem_start_offset: usize, capacity: usize, bind: bool) -> Self {
        assert!(
            capacity > 0,
            "MemMetadataWriter::create | capacity {} must be positive",
            capacity
        );
        assert_eq!(
            capacity & (capacity - 1),
            0,
            "MemMetadataWriter::create | capacity {} must be power of 2",
            capacity
        );

        let mem_end_offset = mem_start_offset + capacity;

        assert!(
            mem_end_offset <= mem.len(),
            "MemMetadataWriter::create | range [{}..{}] exceeds AtomicBuffer boundaries",
            mem_start_offset,
            mem.len()
        );

        if !bind {
            for i in 0..capacity {
                mem[mem_start_offset + i].store(0, Ordering::Relaxed);
            }
        }

        MemMetadataWriter {
            mem,
            mem_start_offset,
            mem_end_offset,
            capacity,
        }
    }

    #[inline]
    pub fn calculate_size_on_mem(capacity: usize) -> usize {
        capacity
    }

    pub fn to_reader(&self) -> MemMetadataReader {
        MemMetadataReader::bind(Arc::clone(&self.mem), self.mem_start_offset, self.capacity)
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
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn write(&self, offset: usize, value: i32) {
        debug_assert!(
            offset < self.capacity,
            "MemMetadataWriter.write | offset {} out of bounds",
            offset
        );
        self.mem[self.mem_start_offset + offset].store(value, Ordering::Relaxed)
    }

    #[inline]
    pub fn read(&self, offset: usize) -> i32 {
        debug_assert!(
            offset < self.capacity,
            "MemMetadataWriter.read | offset {} out of bounds",
            offset
        );
        self.mem[self.mem_start_offset + offset].load(Ordering::Relaxed)
    }

    pub fn copy_from(&self, source: &MemMetadataWriter) {
        debug_assert!(
            source.capacity <= self.capacity,
            "MemMetadataWriter.copy_from | source.capacity {} cannot be greater than destination.capacity {}",
            source.capacity,
            self.capacity,
        );

        for i in 0..source.capacity {
            self.write(i, source.read(i));
        }
    }
}
