use crate::primitives::lut_reader::LutReader;
use crate::primitives::triple_buffer_writer::TripleBufferWriter;

#[derive(Clone)]
pub struct LutWriter {
    tb: TripleBufferWriter,
    size: usize,
    tb_start_offset: usize,
    tb_end_offset: usize,
}

impl LutWriter {
    pub fn new(tb: TripleBufferWriter, size: usize, tb_start_offset: usize) -> Self {
        Self::create(tb, size, tb_start_offset, false)
    }

    pub fn bind(tb: TripleBufferWriter, size: usize, tb_start_offset: usize) -> Self {
        Self::create(tb, size, tb_start_offset, true)
    }

    pub fn create(tb: TripleBufferWriter, size: usize, tb_start_offset: usize, bind: bool) -> Self {
        let tb_end_offset = tb_start_offset + Self::calculate_size_on_tb(size);

        assert!(
            tb_end_offset <= tb.buffer_capacity(),
            "EntryStoreWriter::new | range [{}..{}] exceeds buffer capacity {}",
            tb_start_offset,
            Self::calculate_size_on_tb(size),
            tb.buffer_capacity(),
        );

        if !bind {
            for i in 0..size {
                tb.write(tb_start_offset + i, 0);
            }
        }

        LutWriter {
            tb,
            size,
            tb_start_offset,
            tb_end_offset,
        }
    }

    pub fn calculate_size_on_tb(size: usize) -> usize {
        size
    }

    pub fn to_reader(&self) -> LutReader {
        LutReader::bind(
            self.tb.to_reader(),
            self.size,
            self.tb_start_offset,
            self.tb_end_offset,
        )
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
            "LutWriter.read | index {} out of bounds",
            index
        );

        self.tb.read(self.tb_start_offset + index)
    }

    #[inline]
    pub fn write(&'_ self, index: usize, value: i32) {
        debug_assert!(
            index < self.size,
            "LutWriter.write | index {} out of bounds",
            index
        );

        self.tb.write(self.tb_start_offset + index, value)
    }

    #[inline]
    pub fn read_all(&self, out: &mut [i32]) {
        debug_assert!(
            out.len() <= self.size,
            "LutWriter.read_all | out.len() {} exceeds size {}",
            out.len(),
            self.size,
        );

        self.tb.read_batch(self.tb_start_offset, out);
    }

    #[inline]
    pub fn write_all(&self, data: &[i32]) {
        debug_assert!(
            data.len() <= self.size,
            "LutWriter.write_all | data.len() {} exceeds size {}",
            data.len(),
            self.size,
        );

        self.tb.write_batch(self.tb_start_offset, data);
    }

    pub fn copy_from(&self, source: &Self) {
        debug_assert!(
            source.size <= self.size,
            "LutWriter.copy_from | source.size {} cannot be greater than destination.size {}",
            source.size,
            self.size,
        );

        self.tb.copy_region_from(
            &source.tb,
            source.tb_start_offset,
            self.tb_start_offset,
            Self::calculate_size_on_tb(source.size),
        );
    }
}
