use crate::errors::slot_allocator_error::SlotAllocatorError;
use crate::primitives::entry_handle::EntryHandle;
use crate::primitives::entry_store_config::EntryStoreConfig;
use crate::primitives::entry_store_reader::EntryStoreReader;
use crate::primitives::entry_writer::EntryWriter;
use crate::primitives::mem_zone_writer::MemZoneWriter;
use crate::primitives::slot::SlotId;
use crate::primitives::slot_allocator::SlotAllocator;
use crate::primitives::tb_zone_view::TbZoneView;
use crate::primitives::tb_zone_writer::TbZoneWriter;
use crate::primitives::triple_buffer_writer::TripleBufferWriter;
use crate::primitives::types::AtomicBuffer;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Clone)]
pub struct EntryStoreWriter {
    mem: AtomicBuffer,
    tb: TripleBufferWriter,
    allocator: SlotAllocator,
    config: EntryStoreConfig,
    mem_start_offset: usize,
    mem_attrs_start_offset: usize,
    mem_end_offset: usize,
    tb_start_offset: usize,
    tb_end_offset: usize,
}

impl EntryStoreWriter {
    pub fn new(
        mem: AtomicBuffer,
        tb: TripleBufferWriter,
        config: EntryStoreConfig,
        mem_start_offset: usize,
        tb_start_offset: usize,
    ) -> Self {
        Self::create(mem, tb, config, mem_start_offset, tb_start_offset, false)
    }

    pub fn bind(
        mem: AtomicBuffer,
        tb: TripleBufferWriter,
        config: EntryStoreConfig,
        mem_start_offset: usize,
        tb_start_offset: usize,
    ) -> Self {
        Self::create(mem, tb, config, mem_start_offset, tb_start_offset, true)
    }

    pub fn create(
        mem: AtomicBuffer,
        tb: TripleBufferWriter,
        config: EntryStoreConfig,
        mem_start_offset: usize,
        tb_start_offset: usize,
        bind: bool,
    ) -> Self {
        let capacity = config.capacity;
        let mem_end_offset = mem_start_offset + Self::calculate_size_on_mem(&config);
        let tb_end_offset = tb_start_offset + Self::calculate_size_on_tb(&config);

        assert!(
            mem_end_offset <= mem.len(),
            "EntryStoreWriter::create | range [{}..{}] exceeds AtomicBuffer boundaries",
            mem_start_offset,
            mem.len(),
        );
        assert!(
            tb_end_offset <= tb.buffer_capacity(),
            "EntryStoreWriter::new | range [{}..{}] exceeds buffer capacity {}",
            tb_start_offset,
            Self::calculate_size_on_tb(&config),
            tb.buffer_capacity(),
        );

        let allocator = SlotAllocator::create(Arc::clone(&mem), mem_start_offset, capacity, bind);
        let mem_attrs_start_offset = allocator.mem_end_offset();

        EntryStoreWriter {
            mem,
            tb,
            config,
            allocator,
            mem_start_offset,
            mem_attrs_start_offset,
            mem_end_offset,
            tb_start_offset,
            tb_end_offset,
        }
    }

    pub fn calculate_size_on_mem(config: &EntryStoreConfig) -> usize {
        SlotAllocator::calculate_size_on_mem(config.capacity as usize)
            + config.capacity as usize * config.attr_stride
    }

    pub fn calculate_size_on_tb(config: &EntryStoreConfig) -> usize {
        config.capacity as usize * (config.core_stride + config.meta_stride)
    }

    #[inline]
    pub(crate) fn calculate_struct_zone_base(
        tb_start_offset: usize,
        slot: SlotId,
        core_stride: usize,
        meta_stride: usize,
    ) -> usize {
        debug_assert!(
            slot.to_usize() > 0,
            "EntryStoreWriter::calculate_struct_zone_base | slot {} out of bounds",
            slot
        );
        tb_start_offset + (slot.to_usize() - 1) * (core_stride + meta_stride)
    }

    #[inline]
    pub(crate) fn calculate_attr_zone_base(
        mem_attrs_start_offset: usize,
        slot: SlotId,
        attr_stride: usize,
    ) -> usize {
        debug_assert!(
            slot.to_usize() > 0,
            "EntryStoreWriter::calculate_attr_zone_base | slot {} out of bounds",
            slot
        );
        mem_attrs_start_offset + ((slot.to_usize() - 1) * attr_stride)
    }

    pub fn to_reader(&self) -> EntryStoreReader {
        EntryStoreReader::bind(
            Arc::clone(&self.mem),
            self.tb.to_reader(),
            self.allocator.to_staging_buffer_reader(),
            self.config.clone(),
            self.mem_start_offset,
            self.mem_attrs_start_offset,
            self.mem_end_offset,
            self.tb_start_offset,
            self.tb_end_offset,
        )
    }

    pub fn config(&self) -> EntryStoreConfig {
        self.config
    }

    pub fn len(&self) -> usize {
        self.allocator.alloc_count()
    }

    pub fn mem_start_offset(&self) -> usize {
        self.mem_start_offset
    }

    pub fn mem_end_offset(&self) -> usize {
        self.mem_end_offset
    }

    pub fn mem_staging_buffer_start_offset(&self) -> usize {
        self.allocator.mem_staging_buffer_start_offset()
    }

    pub fn tb_start_offset(&self) -> usize {
        self.tb_start_offset
    }

    pub fn tb_end_offset(&self) -> usize {
        self.tb_end_offset
    }

    pub fn capacity(&self) -> usize {
        self.config.capacity as usize
    }

    pub fn utilization(&self) -> f32 {
        self.allocator.utilization()
    }

    pub fn is_active_slot(&self, slot: SlotId) -> bool {
        self.allocator.is_active(slot)
    }

    #[inline]
    pub fn get(&'_ self, slot: SlotId) -> EntryWriter<'_> {
        debug_assert!(
            self.allocator.is_active(slot),
            "EntryStoreWriter.get | attempted to read inactive slot {}",
            slot
        );

        let tb_start_offset = self.get_entry_tb_base(slot);
        let mem_start_offset = self.get_entry_mem_base(slot);

        EntryWriter::new(
            TbZoneWriter::new(&self.tb, self.config.core_stride, tb_start_offset),
            TbZoneWriter::new(
                &self.tb,
                self.config.meta_stride,
                tb_start_offset + self.config.core_stride,
            ),
            MemZoneWriter::new(&self.mem, self.config.attr_stride, mem_start_offset),
        )
    }

    #[inline]
    pub fn get_handle(&'_ self, slot: SlotId) -> EntryHandle<'_> {
        debug_assert!(
            self.allocator.is_active(slot),
            "EntryStoreWriter.get | attempted to read inactive slot {}",
            slot
        );

        let tb_start_offset = self.get_entry_tb_base(slot);
        let mem_start_offset = self.get_entry_mem_base(slot);

        EntryHandle::new(
            TbZoneView::new(&self.tb, self.config.core_stride, tb_start_offset),
            TbZoneWriter::new(
                &self.tb,
                self.config.meta_stride,
                tb_start_offset + self.config.core_stride,
            ),
            MemZoneWriter::new(&self.mem, self.config.attr_stride, mem_start_offset),
        )
    }

    pub fn insert(&self) -> Option<SlotId> {
        let result = self.allocator.alloc();

        if result.is_none() {
            return None;
        }

        let new_slot = result.unwrap();
        let tb_start_offset = Self::calculate_struct_zone_base(
            self.tb_start_offset,
            new_slot,
            self.config.core_stride,
            self.config.meta_stride,
        );

        for i in 0..(self.config.core_stride + self.config.meta_stride) {
            self.tb.write(tb_start_offset + i, 0)
        }

        self.get(new_slot).attr_clear_all();

        Some(new_slot)
    }

    pub fn remove(&self, slot: SlotId) -> Result<(), SlotAllocatorError> {
        self.allocator.defer_free(slot)
    }

    pub fn publish(&self) {
        self.allocator.publish()
    }

    pub fn copy_from(&self, source: &Self) {
        debug_assert!(
            source.config.capacity <= self.config.capacity,
            "EntryStoreWriter.copy_from | source.config.capacity {} cannot be greater than destination.config.capacity {}",
            source.config.capacity,
            self.config.capacity,
        );

        self.allocator.copy_from(&source.allocator);
        self.tb.copy_region_from(
            &source.tb,
            source.tb_start_offset,
            self.tb_start_offset,
            Self::calculate_size_on_tb(&source.config),
        );

        for i in 0..source.config.capacity as usize * source.config.attr_stride {
            self.mem[self.mem_attrs_start_offset + i].store(
                source.mem[source.mem_attrs_start_offset + i].load(Ordering::Relaxed),
                Ordering::Relaxed,
            )
        }
    }

    fn get_entry_tb_base(&self, slot: SlotId) -> usize {
        let tb_start_offset = Self::calculate_struct_zone_base(
            self.tb_start_offset,
            slot,
            self.config.core_stride,
            self.config.meta_stride,
        );
        let tb_end_offset = tb_start_offset + self.config.core_stride + self.config.meta_stride;

        debug_assert!(
            tb_end_offset <= self.tb.buffer_capacity(),
            "EntryStoreWriter.get | range [{}..{}] exceeds buffer capacity {}",
            tb_start_offset,
            self.config.core_stride + self.config.meta_stride,
            self.tb.buffer_capacity(),
        );

        tb_start_offset
    }

    fn get_entry_mem_base(&self, slot: SlotId) -> usize {
        let mem_start_offset = Self::calculate_attr_zone_base(
            self.mem_attrs_start_offset,
            slot,
            self.config.attr_stride,
        );

        debug_assert!(
            mem_start_offset + self.config.attr_stride <= self.mem_end_offset,
            "EntryStoreWriter.get | slot {} out of bounds",
            slot,
        );

        mem_start_offset
    }
}
