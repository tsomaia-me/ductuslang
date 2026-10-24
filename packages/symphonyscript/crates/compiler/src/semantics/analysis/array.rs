use super::ids::NodeId;
use super::shape::Shape;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Array {
    pub shape: Shape,
    pub elements: Vec<NodeId>,
}
