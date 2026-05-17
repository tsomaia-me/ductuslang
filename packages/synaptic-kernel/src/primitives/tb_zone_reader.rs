use crate::primitives::triple_buffer_reader::TripleBufferReader;

/// Consumer-side view into a fixed-size structural block on the triple buffer.
///
/// Provides safe, offset-based read-only access to a specific `[i32; STRIDE]` sequence.
///
/// # Threading
/// Consumer thread only. Delegates back to the underlying `TripleBufferReader`.
///
/// # Encapsulation
/// - Read-only: structural mutation is strictly prohibited on the reading plane.
/// - Typically instantiated on-the-fly and short-lived.
pub struct TbZoneReader<'a> {
    tb: &'a TripleBufferReader,
    pub stride: usize,
    tb_start_offset: usize,
}

impl<'a> TbZoneReader<'a> {
    #[inline]
    pub fn new(tb: &'a TripleBufferReader, stride: usize, tb_start_offset: usize) -> Self {
        let tb_end_offset = tb_start_offset + stride;

        assert!(
            tb_end_offset <= tb.buffer_capacity(),
            "TbZoneReader::new | range [{}..{}] exceeds buffer capacity {}",
            tb_start_offset,
            stride,
            tb.buffer_capacity(),
        );

        TbZoneReader {
            tb,
            stride,
            tb_start_offset,
        }
    }

    #[inline]
    pub fn tb_start_offset(&self) -> usize {
        self.tb_start_offset
    }

    #[inline]
    pub fn tb_end_offset(&self) -> usize {
        self.tb_start_offset + self.stride
    }

    #[inline]
    pub fn read(&self, offset: usize) -> i32 {
        debug_assert!(
            offset < self.stride,
            "TbZoneReader.read | offset {} out of bounds",
            offset
        );
        self.tb.read(self.tb_start_offset + offset)
    }

    #[inline]
    pub fn read_all(&self, out: &mut [i32]) {
        debug_assert_eq!(
            out.len(),
            self.stride,
            "TbZoneReader::read_all | out.len() {} must be equal to pre-configured stride {}",
            out.len(),
            self.stride
        );

        self.tb.read_batch(self.tb_start_offset, out)
    }
}
