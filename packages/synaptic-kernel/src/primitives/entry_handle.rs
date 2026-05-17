use crate::primitives::mem_zone_writer::MemZoneWriter;
use crate::primitives::tb_zone_view::TbZoneView;
use crate::primitives::tb_zone_writer::TbZoneWriter;

/// Producer-side safe facade for an entry spanning three zones: `core` and `meta`
/// on the triple-buffer plane, `attr` on the mem plane.
///
/// Wraps two `TbZoneWriter`s (core and metadata) and a `MemZoneWriter` (attributes)
/// to provide a strict interface over the raw atomic memory block.
///
/// # Threading
/// Producer thread only. Delegates back to the underlying `TbZoneWriter`s and `MemZoneWriter`.
///
/// # Constraints
/// - Treats core zone as readonly. meta zone stays writable as it belongs to user domain.
pub struct EntryHandle<'a> {
    core: TbZoneView<'a>,
    meta: TbZoneWriter<'a>,
    attr: MemZoneWriter<'a>,
}

impl<'a> EntryHandle<'a> {
    pub fn new(
        core: TbZoneView<'a>,
        meta: TbZoneWriter<'a>,
        attributes: MemZoneWriter<'a>,
    ) -> Self {
        EntryHandle {
            core,
            meta,
            attr: attributes,
        }
    }

    #[inline]
    pub fn core_read(&self, offset: usize) -> i32 {
        self.core.read(offset)
    }

    #[inline]
    pub fn core_read_all(&self, out: &mut [i32]) {
        self.core.read_all(out)
    }

    #[inline]
    pub fn meta_read(&self, offset: usize) -> i32 {
        self.meta.read(offset)
    }

    #[inline]
    pub fn meta_write(&self, offset: usize, value: i32) {
        self.meta.write(offset, value)
    }

    #[inline]
    pub fn meta_read_all(&self, out: &mut [i32]) {
        self.meta.read_all(out)
    }

    #[inline]
    pub fn meta_write_all(&self, data: &[i32]) {
        self.meta.write_all(data)
    }

    #[inline]
    pub fn attr_read(&self, offset: usize) -> i32 {
        self.attr.read(offset)
    }

    #[inline]
    pub fn attr_write(&self, offset: usize, value: i32) {
        self.attr.write(offset, value)
    }

    #[inline]
    pub fn attr_and(&self, offset: usize, value: i32) -> i32 {
        self.attr.and(offset, value)
    }

    #[inline]
    pub fn attr_or(&self, offset: usize, value: i32) -> i32 {
        self.attr.or(offset, value)
    }

    #[inline]
    pub fn attr_read_all(&self, out: &mut [i32]) {
        self.attr.read_all(out)
    }

    #[inline]
    pub fn attr_write_all(&self, data: &[i32]) {
        self.attr.write_all(data)
    }

    #[inline]
    pub fn attr_clear_all(&self) {
        self.attr.clear()
    }
}
