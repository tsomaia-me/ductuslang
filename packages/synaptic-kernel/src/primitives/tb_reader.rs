use crate::primitives::triple_buffer_reader::TripleBufferReader;

/// Consumer-side triple buffer reader backed by a shared `AtomicBuffer`.
///
/// # Threading
/// Consumer thread only. Delegates back to the underlying `TripleBufferReader`.
#[derive(Clone)]
pub struct TbReader<'a> {
    tb: &'a TripleBufferReader,
}

impl<'a> TbReader<'a> {
    #[inline]
    pub fn bind(tb: &'a TripleBufferReader) -> Self {
        TbReader { tb }
    }

    #[inline]
    pub fn buffer_capacity(&self) -> usize {
        self.tb.buffer_capacity()
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
