use std::rc::Rc;

use super::array::Array;
use super::ids::NodeId;
use super::shape::Shape;
use crate::semantics::Internal;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Val {
    Scalar(NodeId),
    Array(Rc<Array>),
    Error,
}

impl Val {
    pub fn array(shape: Shape, elements: Vec<NodeId>) -> Val {
        Val::Array(Rc::new(Array { shape, elements }))
    }

    pub fn nodes(&self) -> Vec<NodeId> {
        match self {
            Val::Scalar(node) => vec![*node],
            Val::Array(array) => array.elements.clone(),
            Val::Error => Vec::new(),
        }
    }

    pub fn shape(&self) -> Shape {
        match self {
            Val::Array(array) => array.shape.clone(),
            Val::Scalar(_) | Val::Error => Shape::scalar(),
        }
    }

    pub fn slice(&self, index: usize) -> Result<Val, Internal> {
        let missed = Internal::missed("array element");
        let Val::Array(array) = self else {
            return Err(missed);
        };
        let inner = array.shape.inner();
        let stride = inner.count();
        let start = index.checked_mul(stride).ok_or(missed)?;
        let elements = array
            .elements
            .get(start..start.checked_add(stride).ok_or(missed)?)
            .ok_or(missed)?
            .to_vec();
        Ok(if inner.is_scalar() {
            Val::Scalar(*elements.first().ok_or(missed)?)
        } else {
            Val::array(inner, elements)
        })
    }
}
