use crate::primitives::lut_def::LutId;
use crate::primitives::lut_reader::LutReader;

/// Fixed-size registry of `N` LUT Readers with user-assigned IDs in [0, N-1] range.
///
/// # ID Semantics
/// IDs form a permutation of `[0, N-1]`. The user assigns each TB an ID in that range,
/// in any order. The registry validates uniqueness and range at construction-time.
/// No gaps, no empty slots.
///
/// # Threading
/// Consumer-side only. The producer uses `LutWriterRegistry`.
#[derive(Clone)]
pub struct LutReaderRegistry<const TB_COUNT: usize, const LUT_COUNT: usize> {
    id_index: [Option<u16>; LUT_COUNT],
    tables: [LutReader; LUT_COUNT],
    default_tb_start_offset: usize,
    default_tb_end_offset: usize,
    extra_tb_start_offsets: [usize; TB_COUNT],
    extra_tb_end_offsets: [usize; TB_COUNT],
}

impl<const TB_COUNT: usize, const LUT_COUNT: usize> LutReaderRegistry<TB_COUNT, LUT_COUNT> {
    pub fn bind(
        id_index: [Option<u16>; LUT_COUNT],
        tables: [LutReader; LUT_COUNT],
        default_tb_start_offset: usize,
        default_tb_end_offset: usize,
        extra_tb_start_offsets: [usize; TB_COUNT],
        extra_tb_end_offsets: [usize; TB_COUNT],
    ) -> Self {
        const { assert!(LUT_COUNT > 0 && LUT_COUNT < u16::MAX as usize) };

        LutReaderRegistry {
            id_index,
            tables,
            default_tb_start_offset,
            default_tb_end_offset,
            extra_tb_start_offsets,
            extra_tb_end_offsets,
        }
    }

    #[inline]
    pub fn default_tb_start_offset(&self) -> usize {
        self.default_tb_start_offset
    }

    #[inline]
    pub fn default_tb_end_offset(&self) -> usize {
        self.default_tb_end_offset
    }

    #[inline]
    pub fn extra_tb_start_offsets(&self) -> [usize; TB_COUNT] {
        self.extra_tb_start_offsets
    }

    #[inline]
    pub fn extra_tb_end_offsets(&self) -> [usize; TB_COUNT] {
        self.extra_tb_end_offsets
    }

    #[inline]
    pub fn get(&self, id: LutId) -> &LutReader {
        debug_assert!(
            (id.0 as usize) < LUT_COUNT,
            "LutReaderRegistry::get | id {} out of bounds [0-{}]",
            id,
            LUT_COUNT - 1,
        );

        let index = self.id_index[id.0 as usize].expect(
            "LutReaderRegistry::get | id_index entry was None - construction invariant violated",
        );
        &self.tables[index as usize]
    }
}
