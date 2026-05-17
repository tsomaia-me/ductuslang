use crate::primitives::triple_buffer_writer::TripleBufferWriter;

/// Producer-side triple buffer writer backed by a shared `AtomicBuffer`.
///
/// # Threading
/// Producer thread only. Delegates back to the underlying `TripleBufferWriter`.
#[derive(Clone)]
pub struct TbWriter<'a> {
    tb: &'a TripleBufferWriter,
}

impl<'a> TbWriter<'a> {
    #[inline]
    pub fn bind(tb: &'a TripleBufferWriter) -> Self {
        TbWriter { tb }
    }

    #[inline]
    pub fn buffer_capacity(&self) -> usize {
        self.tb.buffer_capacity()
    }

    #[inline]
    pub fn write(&self, offset: usize, value: i32) {
        self.tb.write(offset, value);
    }

    #[inline]
    pub fn write_batch(&self, offset: usize, data: &[i32]) {
        self.tb.write_batch(offset, data);
    }

    #[inline]
    pub fn read(&self, offset: usize) -> i32 {
        self.tb.read(offset)
    }

    #[inline]
    pub fn read_batch<const T: usize>(&self, offset: usize, out: &mut [i32]) {
        self.tb.read_batch(offset, out)
    }
}
