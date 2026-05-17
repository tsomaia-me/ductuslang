use crate::primitives::mem_zone_reader::MemZoneReader;
use crate::primitives::tb_zone_reader::TbZoneReader;

/// Consumer-side structural facade for an entry spanning three zones: `core` and `meta`
/// on the triple-buffer plane, `attr` on the mem plane.
///
/// Wraps two `TbZoneReader`s (core and metadata) and a `MemZoneReader` (attributes)
/// to provide a strict read-only interface over the raw atomic memory block.
///
/// # Threading
/// Consumer thread only. Delegates back to the underlying `TbZoneReader`s and `MemZoneReader`.
///
/// # Encapsulation
/// - Read-only: structural mutation is strictly prohibited on the reading plane.
pub struct EntryReader<'a> {
    core: TbZoneReader<'a>,
    meta: TbZoneReader<'a>,
    attr: MemZoneReader<'a>,
}

impl<'a> EntryReader<'a> {
    pub fn new(
        core: TbZoneReader<'a>,
        meta: TbZoneReader<'a>,
        attributes: MemZoneReader<'a>,
    ) -> Self {
        EntryReader {
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
    pub fn meta_read_all(&self, out: &mut [i32]) {
        self.meta.read_all(out)
    }

    #[inline]
    pub fn attr_read(&self, offset: usize) -> i32 {
        self.attr.read(offset)
    }

    #[inline]
    pub fn attr_read_all(&self, out: &mut [i32]) {
        self.attr.read_all(out)
    }
}
