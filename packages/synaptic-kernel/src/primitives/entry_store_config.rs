use crate::primitives::slot_allocator::SlotAllocator;

#[derive(Clone, Copy)]
pub struct EntryStoreConfig {
    pub core_stride: usize,
    pub meta_stride: usize,
    pub attr_stride: usize,
    pub capacity: u32,
}

impl EntryStoreConfig {
    pub fn size_on_mem(&self) -> usize {
        SlotAllocator::calculate_size_on_mem(self.capacity as usize)
            + self.capacity as usize * self.attr_stride
    }

    pub fn size_on_tb(&self) -> usize {
        self.capacity as usize * (self.core_stride + self.meta_stride)
    }
}
