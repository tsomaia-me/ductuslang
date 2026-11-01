use std::cell::RefCell;
use std::rc::Rc;

use super::arg_fact::ArgFact;
use super::lay::Lay;
use super::template_seen::TemplateSeen;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::ir::FnId;

#[derive(Debug)]
pub(crate) struct CallInstr {
    pub node: NodeId,
    pub own: EvalTy,
    pub function: FnId,
    pub facts: Rc<[ArgFact]>,
    pub count: u32,
    pub lay: Lay,
    pub seen: RefCell<Option<TemplateSeen>>,
}
