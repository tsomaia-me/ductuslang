use super::ids::NodeId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Unwritten {
    Init(NodeId),
    Cell,
    Fault,
}
