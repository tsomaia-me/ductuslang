use super::ids::NodeId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ForIterable {
    Range(NodeId, NodeId),
    Values(NodeId),
}

impl ForIterable {
    pub fn nodes(self) -> Vec<NodeId> {
        match self {
            ForIterable::Range(start, end) => vec![start, end],
            ForIterable::Values(values) => vec![values],
        }
    }
}
