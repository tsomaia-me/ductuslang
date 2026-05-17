use crate::primitives::triple_buffer_def::{TripleBufferDef, TripleBufferId};
use crate::primitives::triple_buffer_reader_registry::TripleBufferReaderRegistry;
use crate::primitives::triple_buffer_writer::TripleBufferWriter;
use crate::primitives::types::AtomicBuffer;
use std::sync::Arc;

/// Fixed-size registry of `N` triple-buffer writers with user-assigned IDs in [0, N-1] range
/// plus one default TB.
///
/// The default TB is accessed via `TripleBufferId::DEFAULT`.
///
/// # ID Semantics
/// IDs form a permutation of `[0, N-1]`. The user assigns each TB an ID in that range,
/// in any order. The registry validates uniqueness and range at construction-time.
/// No gaps, no empty slots.
///
/// `TripleBufferId::DEFAULT` (`u16::MAX`) is reserved for the default TB.
///
/// # Threading
/// Producer-side only. The consumer uses `TripleBufferReaderRegistry`.
///
/// # Memory Layout (MEM plane)
/// ```text
/// Offset                  Size
/// start                   size(default)
/// start + size(default)   size(defs[0].buffer_capacity
/// ...
///
/// start = mem_start_offset
/// size(x) = TripleBufferWriter::calculate_size_on_mem(x)
/// ```
/// Two cache-hot array reads per lookup.
#[derive(Clone)]
pub struct TripleBufferWriterRegistry<const N: usize> {
    mem_start_offset: usize,
    mem_end_offset: usize,
    id_index: [Option<u16>; N],
    defs: [TripleBufferDef; N],
    default_tb: TripleBufferWriter,
    tbs: [TripleBufferWriter; N],
}

impl<const N: usize> TripleBufferWriterRegistry<N> {
    pub fn new(
        mem: AtomicBuffer,
        defs: [TripleBufferDef; N],
        mem_start_offset: usize,
        default_tb_capacity: u32,
    ) -> Self {
        Self::create(mem, defs, mem_start_offset, default_tb_capacity, false)
    }

    pub fn bind(
        mem: AtomicBuffer,
        defs: [TripleBufferDef; N],
        mem_start_offset: usize,
        default_tb_capacity: u32,
    ) -> Self {
        Self::create(mem, defs, mem_start_offset, default_tb_capacity, true)
    }

    pub fn create(
        mem: AtomicBuffer,
        defs: [TripleBufferDef; N],
        mem_start_offset: usize,
        default_tb_capacity: u32,
        bind: bool,
    ) -> Self {
        const { assert!(N > 0 && N < u16::MAX as usize) };

        let mut offsets: [usize; N] = [0; N];
        let mut cursor = mem_start_offset
            + TripleBufferWriter::calculate_size_on_mem(default_tb_capacity as usize);

        for i in 0..N {
            offsets[i] = cursor;
            cursor += TripleBufferWriter::calculate_size_on_mem(defs[i].buffer_capacity);
        }

        assert!(
            cursor <= mem.len(),
            "TripleBufferWriterRegistry::create | range [{}..{}] out of AtomicBuffer bounds [0; {}]",
            mem_start_offset,
            cursor,
            mem.len(),
        );

        let mut id_index: [Option<u16>; N] = [None; N];

        for i in 0..N {
            let id = defs[i].id;

            assert!(
                (id.0 as usize) < N,
                "TripleBufferWriterRegistry::create | id {} out of bounds [0-{}]",
                id,
                N - 1,
            );

            assert_eq!(
                id_index[id.0 as usize], None,
                "TripleBufferWriterRegistry::create | duplicate id {}",
                id
            );

            id_index[id.0 as usize] = Some(i as u16);
        }

        let default_tb = TripleBufferWriter::create(
            Arc::clone(&mem),
            mem_start_offset,
            default_tb_capacity,
            bind,
        );
        let tbs: [TripleBufferWriter; N] = std::array::from_fn(|i| {
            TripleBufferWriter::create(
                Arc::clone(&mem),
                offsets[i],
                defs[i].buffer_capacity as u32,
                bind,
            )
        });

        let mem_end_offset = tbs[N - 1].mem_end_offset();

        TripleBufferWriterRegistry {
            mem_start_offset,
            mem_end_offset,
            id_index,
            defs,
            default_tb,
            tbs,
        }
    }

    pub fn calculate_size_on_mem(default_tb_capacity: usize, defs: &[TripleBufferDef; N]) -> usize {
        let mut size: usize = TripleBufferWriter::calculate_size_on_mem(default_tb_capacity);

        for i in 0..N {
            size += TripleBufferWriter::calculate_size_on_mem(defs[i].buffer_capacity)
        }

        size
    }

    pub fn to_reader(&self) -> TripleBufferReaderRegistry<N> {
        TripleBufferReaderRegistry::<N>::bind(
            self.id_index,
            self.default_tb.to_reader(),
            self.tbs.clone().map(|tb| tb.to_reader()),
            self.mem_start_offset,
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
    pub fn len(&self) -> usize {
        self.mem_end_offset - self.mem_start_offset
    }

    #[inline]
    pub fn get(&self, id: TripleBufferId) -> &TripleBufferWriter {
        if id == TripleBufferId::DEFAULT {
            return &self.default_tb;
        }

        debug_assert!(
            (id.0 as usize) < N || id == TripleBufferId::DEFAULT,
            "TripleBufferWriterRegistry::get | id {} out of bounds [0-{}]",
            id,
            N - 1,
        );

        let index = self.id_index[id.0 as usize]
            .expect("TripleBufferWriterRegistry::get | id_index entry was None - construction invariant violated");
        &self.tbs[index as usize]
    }

    pub fn copy_metadata_regions_from<const M: usize>(
        &self,
        source: &TripleBufferWriterRegistry<M>,
    ) {
        debug_assert!(
            M <= N,
            "TripleBufferWriterRegistry.copy_from | source N {} cannot be greater than destination N {}",
            M,
            N,
        );

        debug_assert!(
            source.default_tb.buffer_capacity() <= self.default_tb.buffer_capacity(),
            "TripleBufferWriterRegistry.copy_metadata_regions_from | source.default_tb.buffer_capacity {} cannot be greater than destination.default_tb.buffer_capacity {}",
            source.default_tb.buffer_capacity(),
            self.default_tb.buffer_capacity(),
        );

        for i in 0..M {
            let id = source.defs[i].id;
            let source_tb = source.get(id);
            let dest_tb = self.get(id);

            debug_assert!(
                source_tb.buffer_capacity() <= dest_tb.buffer_capacity(),
                "TripleBufferWriterRegistry.copy_metadata_regions_from | source_tb.buffer_capacity {} cannot be greater than dest_tb.buffer_capacity {}",
                source_tb.buffer_capacity(),
                dest_tb.buffer_capacity(),
            );
        }

        self.default_tb.copy_metadata_from(&source.default_tb);

        for i in 0..M {
            let id = source.defs[i].id;
            self.get(id).copy_metadata_from(&source.get(id));
        }
    }

    #[inline]
    pub(crate) fn index_of(&self, id: TripleBufferId) -> u16 {
        debug_assert!(
            id != TripleBufferId::DEFAULT,
            "TripleBufferWriterRegistry::index_of | index_of may not be called on TripleBufferId::DEFAULT"
        );
        self.id_index[id.0 as usize]
            .expect("TripleBufferWriterRegistry::index_of | id_index entry was None - construction invariant violated")
    }
}
