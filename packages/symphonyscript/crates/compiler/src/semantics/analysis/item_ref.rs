use super::ids::{InstanceId, UseId, WireId};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) enum ItemRef {
    Instance(InstanceId),
    Wire(WireId),
    Use(UseId),
}
