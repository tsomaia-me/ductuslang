use crate::primitives::lut_def::{LutDef, LutId};
use crate::primitives::lut_reader_registry::LutReaderRegistry;
use crate::primitives::lut_writer::LutWriter;
use crate::primitives::triple_buffer_def::TripleBufferId;
use crate::primitives::triple_buffer_writer_registry::TripleBufferWriterRegistry;

/// Fixed-size registry of `N` LUT writers with user-assigned IDs in [0, N-1] range.
///
/// # ID Semantics
/// IDs form a permutation of `[0, N-1]`. The user assigns each TB an ID in that range,
/// in any order. The registry validates uniqueness and range at construction-time.
/// No gaps, no empty slots.
///
/// # Threading
/// Producer-side only. The consumer uses `LutReaderRegistry`.
#[derive(Clone)]
pub struct LutWriterRegistry<const TB_COUNT: usize, const LUT_COUNT: usize> {
    id_index: [Option<u16>; LUT_COUNT],
    defs: [LutDef; LUT_COUNT],
    tables: [LutWriter; LUT_COUNT],
    default_tb_start_offset: usize,
    default_tb_end_offset: usize,
    extra_tb_start_offsets: [usize; TB_COUNT],
    extra_tb_end_offsets: [usize; TB_COUNT],
}

impl<const TB_COUNT: usize, const LUT_COUNT: usize> LutWriterRegistry<TB_COUNT, LUT_COUNT> {
    pub fn new(
        tb_registry: TripleBufferWriterRegistry<TB_COUNT>,
        defs: [LutDef; LUT_COUNT],
        default_tb_start_offset: usize,
        extra_tb_start_offsets: [usize; TB_COUNT],
    ) -> Self {
        Self::create(
            tb_registry,
            defs,
            default_tb_start_offset,
            extra_tb_start_offsets,
            false,
        )
    }

    pub fn bind(
        tb_registry: TripleBufferWriterRegistry<TB_COUNT>,
        defs: [LutDef; LUT_COUNT],
        default_tb_start_offset: usize,
        extra_tb_start_offsets: [usize; TB_COUNT],
    ) -> Self {
        Self::create(
            tb_registry,
            defs,
            default_tb_start_offset,
            extra_tb_start_offsets,
            true,
        )
    }

    pub fn create(
        tb_registry: TripleBufferWriterRegistry<TB_COUNT>,
        defs: [LutDef; LUT_COUNT],
        default_tb_start_offset: usize,
        extra_tb_start_offsets: [usize; TB_COUNT],
        bind: bool,
    ) -> Self {
        const { assert!(LUT_COUNT > 0 && LUT_COUNT < u16::MAX as usize) };

        let mut tb_start_offsets: [usize; LUT_COUNT] = [0; LUT_COUNT];
        let mut default_tb_cursor: usize = default_tb_start_offset;
        let mut extra_tb_cursors: [usize; TB_COUNT] = extra_tb_start_offsets;
        let mut id_index: [Option<u16>; LUT_COUNT] = [None; LUT_COUNT];

        for i in 0..LUT_COUNT {
            let def = defs[i];
            let id = defs[i].id;

            assert!(
                (id.0 as usize) < LUT_COUNT,
                "LutWriterRegistry::create | id {} out of bounds [0-{}]",
                id,
                LUT_COUNT - 1,
            );

            assert_eq!(
                id_index[id.0 as usize],
                None,
                "LutWriterRegistry::create | duplicate id {}",
                id
            );

            id_index[id.0 as usize] = Some(i as u16);

            if def.tb_id == TripleBufferId::DEFAULT {
                tb_start_offsets[i] = default_tb_cursor;
                default_tb_cursor += def.len();

                assert!(
                    default_tb_cursor <= tb_registry.get(def.tb_id).buffer_capacity(),
                    "LutWriterRegistry::create | LUT {} out of Triple Buffer {} bounds [0; {}]",
                    def.id,
                    def.tb_id,
                    tb_registry.get(def.tb_id).buffer_capacity(),
                );
            } else {
                let index = tb_registry.index_of(def.tb_id) as usize;
                tb_start_offsets[i] = extra_tb_cursors[index];
                extra_tb_cursors[index] += def.len();

                assert!(
                    extra_tb_cursors[index] <= tb_registry.get(def.tb_id).buffer_capacity(),
                    "LutWriterRegistry::create | LUT {} out of Triple Buffer {} bounds [0; {}]",
                    def.id,
                    def.tb_id,
                    tb_registry.get(def.tb_id).buffer_capacity(),
                );
            }
        }

        let tables: [LutWriter; LUT_COUNT] = std::array::from_fn(|i| {
            let def = defs[i];
            let tb = tb_registry.get(defs[i].tb_id).clone();

            LutWriter::create(tb, def.len(), tb_start_offsets[i], bind)
        });

        LutWriterRegistry {
            id_index,
            defs,
            tables,
            default_tb_start_offset,
            default_tb_end_offset: default_tb_cursor,
            extra_tb_start_offsets,
            extra_tb_end_offsets: extra_tb_cursors,
        }
    }

    pub fn calculate_size_on_default_tb(defs: &[LutDef; LUT_COUNT]) -> usize {
        Self::calculate_size_on_tb_for(TripleBufferId::DEFAULT, defs)
    }

    pub fn calculate_size_on_tb_for(tb_id: TripleBufferId, defs: &[LutDef; LUT_COUNT]) -> usize {
        let mut size: usize = 0;

        for i in 0..LUT_COUNT {
            let def = defs[i];

            if def.tb_id == tb_id {
                size += defs[i].len()
            }
        }

        size
    }

    pub fn to_reader(&self) -> LutReaderRegistry<TB_COUNT, LUT_COUNT> {
        LutReaderRegistry::<TB_COUNT, LUT_COUNT>::bind(
            self.id_index,
            self.tables.clone().map(|a| a.to_reader()),
            self.default_tb_start_offset,
            self.default_tb_end_offset,
            self.extra_tb_start_offsets,
            self.extra_tb_end_offsets,
        )
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
    pub fn get(&self, id: LutId) -> &LutWriter {
        debug_assert!(
            (id.0 as usize) < LUT_COUNT,
            "LutWriterRegistry::get | id {} out of bounds [0-{}]",
            id,
            LUT_COUNT - 1,
        );

        let index = self.id_index[id.0 as usize]
            .expect("LutWriterRegistry::get | id_index entry was None - construction invariant violated");
        &self.tables[index as usize]
    }

    pub fn copy_from<const TB_COUNT_M: usize, const LUT_COUNT_M: usize>(
        &self,
        source: &LutWriterRegistry<TB_COUNT_M, LUT_COUNT_M>,
    ) {
        debug_assert!(
            TB_COUNT_M <= TB_COUNT,
            "LutWriterRegistry.copy_from | source TB_COUNT {} cannot be greater than destination TB_COUNT {}",
            TB_COUNT_M,
            TB_COUNT,
        );

        debug_assert!(
            LUT_COUNT_M <= LUT_COUNT,
            "LutWriterRegistry.copy_from | source LUT_COUNT {} cannot be greater than destination LUT_COUNT {}",
            LUT_COUNT_M,
            LUT_COUNT,
        );

        for i in 0..LUT_COUNT_M {
            let id = source.defs[i].id;
            let source_lut = source.get(id);
            let dest_lut = self.get(id);

            debug_assert!(
                source_lut.len() <= dest_lut.len(),
                "LutWriterRegistry.copy_from | source_lut.len {} cannot be greater than dest_lut.len {}",
                source_lut.len(),
                dest_lut.len(),
            );
        }

        for i in 0..LUT_COUNT_M {
            let id = source.defs[i].id;
            self.get(id).copy_from(&source.get(id));
        }
    }
}
