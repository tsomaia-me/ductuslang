use crate::primitives::entry_reader::EntryReader;
use crate::primitives::slot::SlotId;

/// Consumer-side structural facade for a graph node on the triple buffer.
///
/// Wraps a `EntryWriter` to provide a strict read-only interface over
/// the raw atomic memory block.
///
/// # Threading
/// Consumer thread only. Delegates back to the underlying `EntryReader`.
///
/// # Core Layout (8x i32)
/// Shares backing region with `NodeWriter`. See its layout.
///
/// # Encapsulation
/// - Read-only: structural mutation is strictly prohibited on the reading plane.
pub struct NodeReader<'a> {
    entry: EntryReader<'a>,
}

impl<'a> NodeReader<'a> {
    pub fn new(entry_reader: EntryReader<'a>) -> Self {
        NodeReader {
            entry: entry_reader,
        }
    }

    #[inline]
    pub fn get_kind(&self) -> i32 {
        (self.entry.core_read(0) as u32 >> 24) as i32
    }

    #[inline]
    pub fn get_next_ptr(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(1))
    }

    #[inline]
    pub fn get_prev_ptr(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(2))
    }

    #[inline]
    pub fn get_outgoing_synapse_head(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(3))
    }

    #[inline]
    pub fn get_outgoing_synapse_tail(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(4))
    }

    #[inline]
    pub fn get_incoming_synapse_head(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(5))
    }

    #[inline]
    pub fn get_incoming_synapse_tail(&self) -> Option<SlotId> {
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
