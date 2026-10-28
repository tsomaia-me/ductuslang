use std::rc::Rc;

use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::call_key::CallKey;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::feed::Feed;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::ir::FnId;

pub(crate) struct Frame {
    pub key: CallKey,
    pub function: FnId,
    pub args: Vec<CVal>,
    pub call: NodeId,
    pub root: NodeId,
    pub ty: EvalTy,
    pub expected: EvalTy,
    pub feeds: Vec<Feed>,
    pub memo: FastMap<(NodeId, EvalTy), CVal>,
    pub unbounded: u64,
    pub faults: FastSet<u32>,
    pub templates: FastSet<ScopeId>,
    pub checking: bool,
    pub check_open: Option<Rc<FastSet<FnId>>>,
}
