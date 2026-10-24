use alloc::vec;
use alloc::vec::Vec;

use super::expr_error::ExprError;
use super::expr_node::ExprNode;
use super::expr_ref::ExprRef;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Expr {
    nodes: Vec<ExprNode>,
    root: ExprRef,
}

impl Expr {
    pub fn leaf(node: ExprNode) -> Result<Expr, ExprError> {
        let root = ExprRef(0);
        operands_precede(&node, root)?;
        Ok(Expr {
            nodes: vec![node],
            root,
        })
    }

    pub fn push(&mut self, node: ExprNode) -> Result<ExprRef, ExprError> {
        let reference = u32::try_from(self.nodes.len())
            .map(ExprRef)
            .map_err(|_| ExprError::TooManyNodes)?;
        operands_precede(&node, reference)?;
        self.nodes.push(node);
        self.root = reference;
        Ok(reference)
    }

    pub fn nodes(&self) -> &[ExprNode] {
        &self.nodes
    }

    pub fn root(&self) -> ExprRef {
        self.root
    }
}

fn operands_precede(node: &ExprNode, own: ExprRef) -> Result<(), ExprError> {
    match node.children().into_iter().find(|operand| *operand >= own) {
        Some(operand) => Err(ExprError::ForwardOperand { node: own, operand }),
        None => Ok(()),
    }
}
