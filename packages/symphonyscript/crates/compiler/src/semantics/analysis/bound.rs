use std::rc::Rc;

use super::ids::{BindingId, NodeId};
use super::shape::Shape;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Bound {
    Cell(BindingId),
    Value(NodeId),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum BoundVal {
    Scalar(Bound),
    Array(Rc<(Shape, Vec<Bound>)>),
    Error,
}
