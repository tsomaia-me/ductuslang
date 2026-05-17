use crate::primitives::types::AtomicBuffer;
use std::sync::atomic::Ordering;

/// Consumer-side view into a single, fixed-size attribute block on a shared `AtomicBuffer`.
///
/// Provides 0-based read-only access to `STRIDE` elements.
///
/// # Threading
/// Consumer thread only. All atomic operations use `Relaxed` ordering.
pub struct MemZoneReader<'a> {
    pub stride: usize,
    mem: &'a AtomicBuffer,
    mem_start_offset: usize,
    mem_end_offset: usize,
}

impl<'a> MemZoneReader<'a> {
    #[inline]
    pub fn new(mem: &'a AtomicBuffer, stride: usize, mem_start_offset: usize) -> Self {
        let mem_end_offset = mem_start_offset + stride;

        assert!(
            mem_end_offset <= mem.len(),
            "MemZoneReader::new | range [{}..{}] exceeds AtomicBuffer boundaries",
            mem_start_offset,
            stride
        );

        MemZoneReader {
            mem,
            stride,
            mem_start_offset,
            mem_end_offset,
        }
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
    pub fn read(&self, offset: usize) -> i32 {
        debug_assert!(
            offset < self.stride,
            "MemZoneReader.read | offset {} out of bounds",
            offset
        );
        self.mem[self.mem_start_offset + offset].load(Ordering::Relaxed)
    }

    #[inline]
    pub fn read_all(&self, out: &mut [i32]) {
        debug_assert_eq!(
            out.len(),
            self.stride,
            "MemZoneReader::read_all | out.len() {} must be equal to pre-configured stride {}",
            out.len(),
            self.stride
        );

        for i in 0..self.stride {
            out[i] = self.read(i)
        }
    }
}
