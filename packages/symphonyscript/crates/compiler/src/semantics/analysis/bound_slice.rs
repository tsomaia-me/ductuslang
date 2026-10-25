use std::rc::Rc;

use super::bound::Bound;
use super::query::Query;
use super::shape::Shape;
use crate::semantics::ir::Loc;

#[derive(Clone)]
pub(crate) struct BoundSlice {
    pub query: Query,
    pub loc: Loc,
    pub array: Rc<(Shape, Vec<Bound>)>,
    pub strides: Rc<[usize]>,
    pub level: usize,
    pub offset: usize,
    pub shape: Shape,
}
