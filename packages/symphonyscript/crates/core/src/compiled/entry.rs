use alloc::vec::Vec;

use crate::ids::{InstanceId, WireId};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Entry {
    Child(InstanceId),
    Bundle(Vec<InstanceId>),
    Wire(WireId),
}
