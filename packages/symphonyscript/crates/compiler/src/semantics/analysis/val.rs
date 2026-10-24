use std::rc::Rc;

use super::array::Array;
use super::ids::NodeId;
use super::shape::Shape;

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

    pub fn slice(&self, index: usize) -> Val {
        let Val::Array(array) = self else {
            return Val::Error;
        };
        let Some(outer) = array.shape.outer() else {
            return Val::Error;
        };
        let inner = array.shape.inner();
        let stride = inner.count();
        if index >= outer as usize {
            return Val::Error;
        }
        let start = index * stride;
        let elements = array
            .elements
            .get(start..start + stride)
            .map(<[NodeId]>::to_vec)
            .unwrap_or_default();
        if inner.is_scalar() {
            match elements.first() {
                Some(node) => Val::Scalar(*node),
                None => Val::Error,
            }
        } else {
            Val::array(inner, elements)
        }
    }
}
