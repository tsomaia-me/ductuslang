use alloc::vec::Vec;

use super::binding::Binding;
use crate::ids::InstanceId;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Target {
    Fixed(InstanceId),
    Select {
        candidates: Vec<InstanceId>,
        index: Binding,
    },
}
