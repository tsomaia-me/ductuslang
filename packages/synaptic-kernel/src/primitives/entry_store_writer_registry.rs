use crate::primitives::entry_store_def::{EntryStoreDef, EntryStoreId};
use crate::primitives::entry_store_reader_registry::EntryStoreReaderRegistry;
use crate::primitives::entry_store_writer::EntryStoreWriter;
use crate::primitives::triple_buffer_def::TripleBufferId;
use crate::primitives::triple_buffer_writer_registry::TripleBufferWriterRegistry;
use crate::primitives::types::AtomicBuffer;
use std::sync::Arc;

/// Fixed-size registry of `N` entry store writers with user-assigned IDs in [0, N-1] range.
///
/// # ID Semantics
/// IDs form a permutation of `[0, N-1]`. The user assigns each TB an ID in that range,
/// in any order. The registry validates uniqueness and range at construction-time.
/// No gaps, no empty slots.
///
/// # Threading
/// Producer-side only. The consumer uses `EntryStoreReaderRegistry`.
#[derive(Clone)]
pub struct EntryStoreWriterRegistry<const TB_COUNT: usize, const STORE_COUNT: usize> {
    id_index: [Option<u16>; STORE_COUNT],
    defs: [EntryStoreDef; STORE_COUNT],
    stores: [EntryStoreWriter; STORE_COUNT],
    mem_start_offset: usize,
    mem_end_offset: usize,
    default_tb_start_offset: usize,
    default_tb_end_offset: usize,
    extra_tb_start_offsets: [usize; TB_COUNT],
    extra_tb_end_offsets: [usize; TB_COUNT],
}

impl<const TB_COUNT: usize, const STORE_COUNT: usize>
    EntryStoreWriterRegistry<TB_COUNT, STORE_COUNT>
{
    pub fn new(
        mem: AtomicBuffer,
        tb_registry: TripleBufferWriterRegistry<TB_COUNT>,
        defs: [EntryStoreDef; STORE_COUNT],
        mem_start_offset: usize,
        default_tb_start_offset: usize,
        extra_tb_start_offsets: [usize; TB_COUNT],
    ) -> Self {
        Self::create(
            mem,
            tb_registry,
            defs,
            mem_start_offset,
            default_tb_start_offset,
            extra_tb_start_offsets,
            false,
        )
    }

    pub fn bind(
        mem: AtomicBuffer,
        tb_registry: TripleBufferWriterRegistry<TB_COUNT>,
        defs: [EntryStoreDef; STORE_COUNT],
        mem_start_offset: usize,
        default_tb_start_offset: usize,
        extra_tb_start_offsets: [usize; TB_COUNT],
    ) -> Self {
        Self::create(
            mem,
            tb_registry,
            defs,
            mem_start_offset,
            default_tb_start_offset,
            extra_tb_start_offsets,
            true,
        )
    }

    pub fn create(
        mem: AtomicBuffer,
        tb_registry: TripleBufferWriterRegistry<TB_COUNT>,
        defs: [EntryStoreDef; STORE_COUNT],
        mem_start_offset: usize,
        default_tb_start_offset: usize,
        extra_tb_start_offsets: [usize; TB_COUNT],
        bind: bool,
    ) -> Self {
        const { assert!(STORE_COUNT > 0 && STORE_COUNT < u16::MAX as usize) };

        let mut mem_start_offsets: [usize; STORE_COUNT] = [0; STORE_COUNT];
        let mut tb_start_offsets: [usize; STORE_COUNT] = [0; STORE_COUNT];
        let mut mem_cursor = mem_start_offset;
        let mut default_tb_cursor: usize = default_tb_start_offset;
        let mut extra_tb_cursors: [usize; TB_COUNT] = extra_tb_start_offsets;
        let mut id_index: [Option<u16>; STORE_COUNT] = [None; STORE_COUNT];

        for i in 0..STORE_COUNT {
            let def = defs[i];
            let id = defs[i].id;

            assert!(
                (id.0 as usize) < STORE_COUNT,
                "EntryStoreWriterRegistry::create | id {} out of bounds [0-{}]",
                id,
                STORE_COUNT - 1,
            );

            assert_eq!(
                id_index[id.0 as usize], None,
                "EntryStoreWriterRegistry::create | duplicate id {}",
                id
            );

            mem_start_offsets[i] = mem_cursor;
            mem_cursor += def.size_on_mem();
            id_index[id.0 as usize] = Some(i as u16);

            if def.tb_id == TripleBufferId::DEFAULT {
                tb_start_offsets[i] = default_tb_cursor;
                default_tb_cursor += def.size_on_tb();

                assert!(
                    default_tb_cursor <= tb_registry.get(def.tb_id).buffer_capacity(),
                    "EntryStoreWriterRegistry::create | Store {} out of Triple Buffer {} bounds [0; {}]",
                    def.id,
                    def.tb_id,
                    tb_registry.get(def.tb_id).buffer_capacity(),
                );
            } else {
                let index = tb_registry.index_of(def.tb_id) as usize;
                tb_start_offsets[i] = extra_tb_cursors[index];
                extra_tb_cursors[index] += def.size_on_tb();

                assert!(
                    extra_tb_cursors[index] <= tb_registry.get(def.tb_id).buffer_capacity(),
                    "EntryStoreWriterRegistry::create | Store {} out of Triple Buffer {} bounds [0; {}]",
                    def.id,
                    def.tb_id,
                    tb_registry.get(def.tb_id).buffer_capacity(),
                );
            }
        }

        assert!(
            mem_cursor <= mem.len(),
            "EntryStoreWriterRegistry::create | range [{}..{}] out of AtomicBuffer bounds [0; {}]",
            mem_start_offset,
            mem_cursor,
            mem.len(),
        );

        let stores: [EntryStoreWriter; STORE_COUNT] = std::array::from_fn(|i| {
            let def = defs[i];
            let tb = tb_registry.get(defs[i].tb_id).clone();

            EntryStoreWriter::create(
                Arc::clone(&mem),
                tb,
                def.config(),
                mem_start_offsets[i],
                tb_start_offsets[i],
                bind,
            )
        });

        EntryStoreWriterRegistry {
            mem_start_offset,
            mem_end_offset: mem_cursor,
            default_tb_start_offset,
            default_tb_end_offset: default_tb_cursor,
            extra_tb_start_offsets,
            extra_tb_end_offsets: extra_tb_cursors,
            id_index,
            defs,
            stores,
        }
    }

    pub fn calculate_size_on_mem(defs: &[EntryStoreDef; STORE_COUNT]) -> usize {
        let mut size: usize = 0;

        for i in 0..STORE_COUNT {
            size += defs[i].size_on_mem()
        }

        size
    }

    pub fn calculate_size_on_default_tb(defs: &[EntryStoreDef; STORE_COUNT]) -> usize {
        Self::calculate_size_on_tb_for(TripleBufferId::DEFAULT, defs)
    }

    pub fn calculate_size_on_tb_for(
        tb_id: TripleBufferId,
        defs: &[EntryStoreDef; STORE_COUNT],
    ) -> usize {
        let mut size: usize = 0;

        for i in 0..STORE_COUNT {
            let def = defs[i];

            if def.tb_id == tb_id {
                size += defs[i].size_on_tb()
            }
        }

        size
    }

    pub fn to_reader(&self) -> EntryStoreReaderRegistry<TB_COUNT, STORE_COUNT> {
        EntryStoreReaderRegistry::<TB_COUNT, STORE_COUNT>::bind(
            self.id_index,
            self.stores.clone().map(|a| a.to_reader()),
            self.mem_start_offset,
            self.mem_end_offset,
            self.default_tb_start_offset,
            self.default_tb_end_offset,
            self.extra_tb_start_offsets,
            self.extra_tb_end_offsets,
        )
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
    pub fn get(&self, id: EntryStoreId) -> &EntryStoreWriter {
        debug_assert!(
            (id.0 as usize) < STORE_COUNT,
            "EntryStoreWriterRegistry::get | id {} out of bounds [0-{}]",
            id,
            STORE_COUNT - 1,
        );

        let index = self.id_index[id.0 as usize]
            .expect("EntryStoreWriterRegistry::get | id_index entry was None - construction invariant violated");
        &self.stores[index as usize]
    }

    pub fn publish_all(&self) {
        for i in 0..self.stores.len() {
            self.stores[i].publish()
        }
    }

    pub fn copy_from<const TB_COUNT_M: usize, const STORE_COUNT_M: usize>(
        &self,
        source: &EntryStoreWriterRegistry<TB_COUNT_M, STORE_COUNT_M>,
    ) {
        debug_assert!(
            TB_COUNT_M <= TB_COUNT,
            "EntryStoreWriterRegistry.copy_from | source TB_COUNT {} cannot be greater than destination TB_COUNT {}",
            TB_COUNT_M,
            TB_COUNT,
        );

        debug_assert!(
            STORE_COUNT_M <= STORE_COUNT,
            "EntryStoreWriterRegistry.copy_from | source STORE_COUNT {} cannot be greater than destination STORE_COUNT {}",
            STORE_COUNT_M,
            STORE_COUNT,
        );

        for i in 0..STORE_COUNT_M {
            let id = source.defs[i].id;
            let source_store = source.get(id);
            let dest_store = self.get(id);

            debug_assert!(
                source_store.capacity() <= dest_store.capacity(),
                "EntryStoreWriterRegistry.copy_from | source_store.capacity {} cannot be greater than dest_store.capacity {}",
                source_store.capacity(),
                dest_store.capacity(),
            );
        }

        for i in 0..STORE_COUNT_M {
            let id = source.defs[i].id;
            self.get(id).copy_from(&source.get(id));
        }
    }
}
