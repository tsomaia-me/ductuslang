use std::rc::Rc;

use super::program::Program;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::NodeId;

pub(crate) struct Resume {
    pub index: usize,
    pub frame: usize,
    pub node: NodeId,
    pub ty: EvalTy,
    pub program: Rc<Program>,
    pub pc: u32,
    pub base: u32,
}
