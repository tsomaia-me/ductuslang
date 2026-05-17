use crate::primitives::entry_reader::EntryReader;
use crate::primitives::entry_store_config::EntryStoreConfig;
use crate::primitives::mem_zone_reader::MemZoneReader;
use crate::primitives::slot::SlotId;
use crate::primitives::slot_allocator::SlotAllocator;
use crate::primitives::staging_buffer_reader::StagingBufferReader;
use crate::primitives::tb_zone_reader::TbZoneReader;
use crate::primitives::triple_buffer_reader::TripleBufferReader;
use crate::primitives::types::AtomicBuffer;

#[derive(Clone)]
pub struct EntryStoreReader {
    mem: AtomicBuffer,
    tb: TripleBufferReader,
    staging_buffer_reader: StagingBufferReader,
    config: EntryStoreConfig,
    mem_start_offset: usize,
    mem_attrs_start_offset: usize,
    mem_end_offset: usize,
    tb_start_offset: usize,
    tb_end_offset: usize,
}
impl EntryStoreReader {
    pub fn bind(
        mem: AtomicBuffer,
        tb: TripleBufferReader,
        staging_buffer_reader: StagingBufferReader,
        config: EntryStoreConfig,
        mem_start_offset: usize,
        mem_attrs_start_offset: usize,
        mem_end_offset: usize,
        tb_start_offset: usize,
        tb_end_offset: usize,
    ) -> Self {
        assert!(
            mem_end_offset <= mem.len(),
            "EntryStoreReader::bind | range [{}..{}] exceeds AtomicBuffer boundaries",
            mem_start_offset,
            mem.len(),
        );
        assert!(
            tb_end_offset <= tb.buffer_capacity(),
            "EntryStoreReader::bind | range [{}..{}] exceeds buffer capacity {}",
            tb_start_offset,
            Self::calculate_size_on_tb(&config),
            tb.buffer_capacity(),
        );

        EntryStoreReader {
            mem,
            tb,
            staging_buffer_reader,
            config,
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

    #[inline]
    pub(crate) fn calculate_struct_zone_base(
        tb_start_offset: usize,
        slot: SlotId,
        config: &EntryStoreConfig,
    ) -> usize {
        debug_assert!(
            slot.to_usize() > 0,
            "EntryStoreReader::calculate_struct_zone_base | slot {} out of bounds",
            slot
        );
        tb_start_offset + (slot.to_usize() - 1) * (config.core_stride + config.meta_stride)
    }

    #[inline]
    pub(crate) fn calculate_attr_zone_base(
        mem_attrs_start_offset: usize,
        slot: SlotId,
        config: &EntryStoreConfig,
    ) -> usize {
        debug_assert!(
            slot.to_usize() > 0,
            "EntryStoreReader::calculate_attr_zone_base | slot {} out of bounds",
            slot
        );
        mem_attrs_start_offset + ((slot.to_usize() - 1) * config.attr_stride)
    }

    pub fn calculate_size_on_tb(config: &EntryStoreConfig) -> usize {
        config.capacity as usize * (config.core_stride + config.meta_stride)
    }

    pub fn mem_start_offset(&self) -> usize {
        self.mem_start_offset
    }

    pub fn mem_end_offset(&self) -> usize {
        self.mem_end_offset
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

    #[inline]
    pub fn get(&'_ self, slot: SlotId) -> EntryReader<'_> {
        let tb_start_offset = self.get_entry_tb_base(slot);
        let mem_start_offset = self.get_entry_mem_base(slot);

        EntryReader::new(
            TbZoneReader::new(&self.tb, self.config.core_stride, tb_start_offset),
            TbZoneReader::new(
                &self.tb,
                self.config.meta_stride,
                tb_start_offset + self.config.core_stride,
            ),
            MemZoneReader::new(&self.mem, self.config.attr_stride, mem_start_offset),
        )
    }

    pub fn ack_generation(&self) {
        self.staging_buffer_reader.ack()
    }

    fn get_entry_tb_base(&self, slot: SlotId) -> usize {
        let tb_start_offset =
            Self::calculate_struct_zone_base(self.tb_start_offset, slot, &self.config);
        let tb_end_offset = tb_start_offset + self.config.core_stride + self.config.meta_stride;

        debug_assert!(
            tb_end_offset <= self.tb.buffer_capacity(),
            "EntryStoreReader.get | range [{}..{}] exceeds buffer capacity {}",
            tb_start_offset,
            self.config.core_stride + self.config.meta_stride,
            self.tb.buffer_capacity(),
        );

        tb_start_offset
    }

    fn get_entry_mem_base(&self, slot: SlotId) -> usize {
        let mem_start_offset =
            Self::calculate_attr_zone_base(self.mem_attrs_start_offset, slot, &self.config);

        debug_assert!(
            mem_start_offset + self.config.attr_stride <= self.mem_end_offset,
            "EntryStoreReader.get | slot {} out of bounds",
            slot,
        );

        mem_start_offset
    }
}
