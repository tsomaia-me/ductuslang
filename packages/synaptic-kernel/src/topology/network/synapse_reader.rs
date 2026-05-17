use crate::primitives::entry_reader::EntryReader;
use crate::primitives::slot::SlotId;

/// Consumer-side structural facade for a graph synapse on the triple buffer.
///
/// Wraps a `EntryReader` to provide a strict read-only interface over
/// the raw atomic memory block.
///
/// # Threading
/// Consumer thread only. Delegates back to the underlying `EntryReader`.
///
/// # Core Layout (8x i32)
/// Shares backing region with `SynapseWriter`. See its layout.
///
/// # Encapsulation
/// - Read-only: structural mutation is strictly prohibited on the reading plane.
pub struct SynapseReader<'a> {
    entry: EntryReader<'a>,
}

impl<'a> SynapseReader<'a> {
    pub fn new(entry_reader: EntryReader<'a>) -> Self {
        SynapseReader {
            entry: entry_reader,
        }
    }

    #[inline]
    pub fn get_kind(&self) -> i32 {
        (self.entry.core_read(0) as u32 >> 24) as i32
    }

    #[inline]
    pub fn get_source_ptr(&self) -> SlotId {
        SlotId::from_i32(self.entry.core_read(1))
            .expect("SynapseReader::get_source_ptr | synapse is mid-construction or corrupted")
    }

    #[inline]
    pub fn get_target_ptr(&self) -> SlotId {
        SlotId::from_i32(self.entry.core_read(2))
            .expect("SynapseReader::get_target_ptr | synapse is mid-construction or corrupted")
    }

    #[inline]
    pub fn get_outgoing_next_ptr(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(3))
    }

    #[inline]
    pub fn get_outgoing_prev_ptr(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(4))
    }

    #[inline]
    pub fn get_incoming_next_ptr(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(5))
    }

    #[inline]
    pub fn get_incoming_prev_ptr(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(6))
    }

    #[inline]
    pub fn get_meta(&self, offset: usize) -> i32 {
        self.entry.meta_read(offset)
    }

    #[inline]
    pub fn get_meta_all(&self, out: &mut [i32]) {
        self.entry.meta_read_all(out)
    }

    #[inline]
    pub fn attr_read(&self, offset: usize) -> i32 {
        self.entry.attr_read(offset)
    }

    #[inline]
    pub fn attr_read_all(&self, out: &mut [i32]) {
        self.entry.attr_read_all(out)
    }
}
