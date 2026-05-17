use crate::constants::NODE_STRIDE;
use crate::primitives::entry_store_config::EntryStoreConfig;

#[derive(Clone, Copy)]
pub struct NodeStoreConfig {
    pub meta_stride: usize,
    pub attr_stride: usize,
    pub capacity: u32,
}

impl NodeStoreConfig {
    pub fn to_entry_store_config(&self) -> EntryStoreConfig {
        EntryStoreConfig {
            core_stride: NODE_STRIDE,
            meta_stride: self.meta_stride,
            attr_stride: self.attr_stride,
            capacity: self.capacity,
        }
    }
}
