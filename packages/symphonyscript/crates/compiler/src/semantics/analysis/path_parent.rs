use super::ids::{InstanceId, UseId};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) enum PathParent {
    Instance(InstanceId),
    Use(UseId),
}
