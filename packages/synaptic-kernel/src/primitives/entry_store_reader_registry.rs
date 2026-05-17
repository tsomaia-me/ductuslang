use crate::primitives::entry_store_def::EntryStoreId;
use crate::primitives::entry_store_reader::EntryStoreReader;

/// Fixed-size registry of `N` Entry-store Readers with user-assigned IDs in [0, N-1] range.
///
/// # ID Semantics
/// IDs form a permutation of `[0, N-1]`. The user assigns each TB an ID in that range,
/// in any order. The registry validates uniqueness and range at construction-time.
/// No gaps, no empty slots.
///
/// # Threading
/// Consumer-side only. The producer uses `EntryStoreWriterRegistry`.
#[derive(Clone)]
pub struct EntryStoreReaderRegistry<const TB_COUNT: usize, const STORE_COUNT: usize> {
    id_index: [Option<u16>; STORE_COUNT],
    stores: [EntryStoreReader; STORE_COUNT],
    mem_start_offset: usize,
    mem_end_offset: usize,
    default_tb_start_offset: usize,
    default_tb_end_offset: usize,
    extra_tb_start_offsets: [usize; TB_COUNT],
    extra_tb_end_offsets: [usize; TB_COUNT],
}

impl<const TB_COUNT: usize, const STORE_COUNT: usize>
    EntryStoreReaderRegistry<TB_COUNT, STORE_COUNT>
{
    pub fn bind(
        id_index: [Option<u16>; STORE_COUNT],
        stores: [EntryStoreReader; STORE_COUNT],
        mem_start_offset: usize,
        mem_end_offset: usize,
        default_tb_start_offset: usize,
        default_tb_end_offset: usize,
        extra_tb_start_offsets: [usize; TB_COUNT],
        extra_tb_end_offsets: [usize; TB_COUNT],
    ) -> Self {
        const { assert!(STORE_COUNT > 0 && STORE_COUNT < u16::MAX as usize) };

        EntryStoreReaderRegistry {
            id_index,
            stores,
            mem_start_offset,
            mem_end_offset,
            default_tb_start_offset,
            default_tb_end_offset,
            extra_tb_start_offsets,
            extra_tb_end_offsets,
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
    pub fn len(&self) -> usize {
        self.mem_end_offset - self.mem_start_offset
    }

    #[inline]
    pub fn get(&self, id: EntryStoreId) -> &EntryStoreReader {
        debug_assert!(
            (id.0 as usize) < STORE_COUNT,
            "EntryStoreReaderRegistry::get | id {} out of bounds [0-{}]",
            id,
            STORE_COUNT - 1,
        );

        let index = self.id_index[id.0 as usize]
            .expect("EntryStoreReaderRegistry::get | id_index entry was None - construction invariant violated");
        &self.stores[index as usize]
    }
}
