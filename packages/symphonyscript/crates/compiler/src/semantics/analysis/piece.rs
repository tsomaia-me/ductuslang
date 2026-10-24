use super::ids::NodeId;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Piece {
    Text(String),
    Value(NodeId),
}
