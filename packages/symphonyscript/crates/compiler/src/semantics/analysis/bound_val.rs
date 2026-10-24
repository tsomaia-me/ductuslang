use std::rc::Rc;

use super::bound::Bound;
use super::shape::Shape;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum BoundVal {
    Scalar(Bound),
    Array(Rc<(Shape, Vec<Bound>)>),
    Error,
}
