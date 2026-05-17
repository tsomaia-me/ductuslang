use crate::primitives::triple_buffer_writer::TripleBufferWriter;

/// Producer-side view into a fixed-size structural block on the triple buffer.
///
/// Provides safe, offset-based read and write access to a specific `[i32; STRIDE]` sequence.
///
/// # Threading
/// Producer thread only. Delegates back to the underlying `TripleBufferWriter`.
///
/// # Encapsulation
/// - Typically instantiated on-the-fly and short-lived.
pub struct TbZoneWriter<'a> {
    pub stride: usize,
    tb: &'a TripleBufferWriter,
    tb_start_offset: usize,
}

impl<'a> TbZoneWriter<'a> {
    #[inline]
    pub fn new(tb: &'a TripleBufferWriter, stride: usize, tb_start_offset: usize) -> Self {
        let tb_end_offset = tb_start_offset + stride;

        assert!(
            tb_end_offset <= tb.buffer_capacity(),
            "TbZoneWriter::new | range [{}..{}] exceeds buffer capacity {}",
            tb_start_offset,
            stride,
            tb.buffer_capacity(),
        );

        TbZoneWriter {
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
            "TbZoneWriter.read | offset {} out of bounds",
            offset
        );
        self.tb.read(self.tb_start_offset + offset)
    }

    #[inline]
    pub fn write(&self, offset: usize, value: i32) {
        debug_assert!(
            offset < self.stride,
            "TbZoneWriter.write | offset {} out of bounds",
            offset
        );
        self.tb.write(self.tb_start_offset + offset, value)
    }

    #[inline]
    pub fn read_all(&self, out: &mut [i32]) {
        debug_assert_eq!(
            out.len(),
            self.stride,
            "TbZoneWriter::read_all | out.len() {} must be equal to pre-configured stride {}",
            out.len(),
            self.stride
        );

        self.tb.read_batch(self.tb_start_offset, out)
    }

    #[inline]
    pub fn write_all(&self, data: &[i32]) {
        debug_assert_eq!(
            data.len(),
            self.stride,
            "TbZoneWriter::write_all | data.len() {} must be equal to pre-configured stride {}",
            data.len(),
            self.stride
        );

        self.tb.write_batch(self.tb_start_offset, data);
    }
}
