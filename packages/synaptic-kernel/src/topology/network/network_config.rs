use crate::constants::SYNAPSE_STRIDE;
use crate::primitives::entry_store_config::EntryStoreConfig;
use crate::topology::node::node_store_config::NodeStoreConfig;

#[derive(Clone, Copy)]
pub struct NetworkConfig {
    pub node_capacity: u32,
    pub node_meta_stride: usize,
    pub node_attr_stride: usize,
    pub synapse_capacity: u32,
    pub synapse_meta_stride: usize,
    pub synapse_attr_stride: usize,
}

impl NetworkConfig {
    pub fn to_node_store_config(&self) -> NodeStoreConfig {
        NodeStoreConfig {
            meta_stride: self.node_meta_stride,
            attr_stride: self.node_attr_stride,
            capacity: self.node_capacity,
        }
    }

    pub fn to_synapse_entry_store_config(&self) -> EntryStoreConfig {
        EntryStoreConfig {
            core_stride: SYNAPSE_STRIDE,
            meta_stride: self.synapse_meta_stride,
            attr_stride: self.synapse_attr_stride,
            capacity: self.synapse_capacity,
        }
    }
}
