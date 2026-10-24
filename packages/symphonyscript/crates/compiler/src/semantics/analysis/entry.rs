use super::ids::{InstanceId, WireId};

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Entry {
    Child(InstanceId),
    Bundle(Vec<InstanceId>),
    Wire(WireId),
}
