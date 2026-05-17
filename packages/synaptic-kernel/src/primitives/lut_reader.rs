use crate::primitives::triple_buffer_reader::TripleBufferReader;

#[derive(Clone)]
pub struct LutReader {
    tb: TripleBufferReader,
    size: usize,
    tb_start_offset: usize,
    tb_end_offset: usize,
}
impl LutReader {
    pub fn bind(
        tb: TripleBufferReader,
        size: usize,
        tb_start_offset: usize,
        tb_end_offset: usize,
    ) -> Self {
        assert!(
            tb_end_offset <= tb.buffer_capacity(),
            "EntryStoreReader::bind | range [{}..{}] exceeds buffer capacity {}",
            tb_start_offset,
            Self::calculate_size_on_tb(size),
            tb.buffer_capacity(),
        );

        LutReader {
            tb,
            size,
            tb_start_offset,
            tb_end_offset,
        }
    }

    pub fn calculate_size_on_tb(size: usize) -> usize {
        size
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn tb_start_offset(&self) -> usize {
        self.tb_start_offset
    }

    pub fn tb_end_offset(&self) -> usize {
        self.tb_end_offset
    }

    #[inline]
    pub fn read(&'_ self, index: usize) -> i32 {
        debug_assert!(
            index < self.size,
            "LutReader.read | index {} out of bounds",
            index
        );

        self.tb.read(self.tb_start_offset + index)
    }

    #[inline]
    pub fn read_all(&self, out: &mut [i32]) {
        debug_assert!(
            out.len() <= self.size,
            "LutReader.read_all | out.len() {} exceeds size {}",
            out.len(),
            self.size,
        );

        self.tb.read_batch(self.tb_start_offset, out);
    }
}
