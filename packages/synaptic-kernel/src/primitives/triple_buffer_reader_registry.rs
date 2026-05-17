use crate::primitives::triple_buffer_def::TripleBufferId;
use crate::primitives::triple_buffer_reader::TripleBufferReader;

/// Fixed-size registry of `N` triple-buffer readers with user-assigned IDs in [0, N-1] range.
///
/// # ID Semantics
/// IDs form a permutation of `[0, N-1]`. The user assigns each TB an ID in that range,
/// in any order. The registry validates uniqueness and range at construction-time.
/// No gaps, no empty slots.
///
/// `TripleBufferId::DEFAULT` (`u16::MAX`) is reserved for the default TB.
///
/// # Threading
/// Consumer-side only. The producer uses `TripleBufferWriterRegistry`.
///
/// # Memory Layout
/// Shares backing region with `TripleBufferWriterRegistry`. See its layout.
#[derive(Clone)]
pub struct TripleBufferReaderRegistry<const N: usize> {
    id_index: [Option<u16>; N],
    default_tb: TripleBufferReader,
    tbs: [TripleBufferReader; N],
    mem_start_offset: usize,
    mem_end_offset: usize,
}

impl<const N: usize> TripleBufferReaderRegistry<N> {
    pub(crate) fn bind(
        id_index: [Option<u16>; N],
        default_tb: TripleBufferReader,
        tbs: [TripleBufferReader; N],
        mem_start_offset: usize,
    ) -> Self {
        const { assert!(N > 0 && N < u16::MAX as usize) };

        let mem_end_offset = tbs[N - 1].mem_end_offset();

        TripleBufferReaderRegistry {
            id_index,
            default_tb,
            tbs,
            mem_start_offset,
            mem_end_offset,
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
    pub fn len(&self) -> usize {
        self.mem_end_offset - self.mem_start_offset
    }

    #[inline]
    pub fn get(&self, id: TripleBufferId) -> &TripleBufferReader {
        if id == TripleBufferId::DEFAULT {
            return &self.default_tb;
        }

        debug_assert!(
            (id.0 as usize) < N || id == TripleBufferId::DEFAULT,
            "TripleBufferReaderRegistry::get | id {} out of bounds [0-{}]",
            id,
            N - 1,
        );

        let index = self.id_index[id.0 as usize]
            .expect("TripleBufferReaderRegistry::get | id_index entry was None - construction invariant violated");
        &self.tbs[index as usize]
    }
}
