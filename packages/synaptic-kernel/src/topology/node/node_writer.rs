use crate::primitives::entry_writer::EntryWriter;
use crate::primitives::slot::SlotId;

/// Producer-side structural facade for a graph node on the triple buffer.
///
/// Wraps a `EntryWriter` to provide a strict interface over
/// the raw atomic memory block.
///
/// # Threading
/// Producer thread only. Delegates back to the underlying `EntryWriter`.
///
/// # Core Layout (8x i32)
/// - `0`: `kind` (shifted by 24 bits) combined with potential future internal flags (lower 24 bits).
/// - `1`: `next_ptr`
/// - `2`: `prev_ptr`
/// - `3`: `outgoing_synapse_head`
/// - `4`: `outgoing_synapse_tail`
/// - `5`: `incoming_synapse_head`
/// - `6`: `incoming_synapse_tail`
/// - `7`: (Reserved for future use)
///
/// Followed by `META_STRIDE` `i32` slots for custom topology metadata.
///
/// # Encapsulation
/// - All mutation methods (`set_*`) are `pub` except meta setters.
///   Only the kernel can mutate active topology, enforcing structural graph invariants.
pub struct NodeWriter<'a> {
    entry: EntryWriter<'a>,
}

impl<'a> NodeWriter<'a> {
    pub fn new(entry_writer: EntryWriter<'a>) -> Self {
        NodeWriter {
            entry: entry_writer,
        }
    }

    #[inline]
    pub fn get_kind(&self) -> i32 {
        (self.entry.core_read(0) as u32 >> 24) as i32
    }

    #[inline]
    pub fn set_kind(&self, value: i32) {
        debug_assert!(
            value >= 0 && value < 256,
            "NodeWriter.set_kind | kind {} out of bounds [0, 256)",
            value
        );
        let bitmask = self.entry.core_read(0) & ((1 << 24) - 1);
        self.entry.core_write(0, bitmask | value << 24)
    }

    #[inline]
    pub fn get_next_ptr(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(1))
    }

    #[inline]
    pub fn set_next_ptr(&self, value: Option<SlotId>) {
        self.entry.core_write(1, SlotId::option_to_i32(value))
    }

    #[inline]
    pub fn get_prev_ptr(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(2))
    }

    #[inline]
    pub fn set_prev_ptr(&self, value: Option<SlotId>) {
        self.entry.core_write(2, SlotId::option_to_i32(value))
    }

    #[inline]
    pub fn get_outgoing_synapse_head(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(3))
    }

    #[inline]
    pub fn set_outgoing_synapse_head(&self, value: Option<SlotId>) {
        self.entry.core_write(3, SlotId::option_to_i32(value))
    }

    #[inline]
    pub fn get_outgoing_synapse_tail(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(4))
    }

    #[inline]
    pub fn set_outgoing_synapse_tail(&self, value: Option<SlotId>) {
        self.entry.core_write(4, SlotId::option_to_i32(value))
    }

    #[inline]
    pub fn get_incoming_synapse_head(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(5))
    }

    #[inline]
    pub fn set_incoming_synapse_head(&self, value: Option<SlotId>) {
        self.entry.core_write(5, SlotId::option_to_i32(value))
    }

    #[inline]
    pub fn get_incoming_synapse_tail(&self) -> Option<SlotId> {
        SlotId::from_i32(self.entry.core_read(6))
    }

    #[inline]
    pub fn set_incoming_synapse_tail(&self, value: Option<SlotId>) {
        self.entry.core_write(6, SlotId::option_to_i32(value))
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
    pub fn set_meta(&self, offset: usize, value: i32) {
        self.entry.meta_write(offset, value)
    }

    #[inline]
    pub fn set_meta_all(&self, data: &[i32]) {
        self.entry.meta_write_all(data)
    }

    #[inline]
    pub fn attr_read(&self, offset: usize) -> i32 {
        self.entry.attr_read(offset)
    }

    #[inline]
    pub fn attr_write(&self, offset: usize, value: i32) {
        self.entry.attr_write(offset, value)
    }

    #[inline]
    pub fn attr_and(&self, offset: usize, value: i32) -> i32 {
        self.entry.attr_and(offset, value)
    }

    #[inline]
    pub fn attr_or(&self, offset: usize, value: i32) -> i32 {
        self.entry.attr_or(offset, value)
    }

    #[inline]
    pub fn attr_read_all(&self, out: &mut [i32]) {
        self.entry.attr_read_all(out)
    }

    #[inline]
    pub fn attr_write_all(&self, data: &[i32]) {
        self.entry.attr_write_all(data)
    }

    #[inline]
    pub fn attr_clear_all(&self) {
        self.entry.attr_clear_all()
    }
}
