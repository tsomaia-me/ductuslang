use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::NodeId;

pub(crate) enum Checks {
    Unchecked,
    Running {
        start: NodeId,
        pending: Vec<NodeId>,
        log: Vec<(NodeId, EvalTy)>,
    },
    Done,
    Quiet,
}

impl Checks {
    pub(crate) fn quiet(&self) -> bool {
        match self {
            Checks::Running { .. } | Checks::Quiet => true,
            Checks::Unchecked | Checks::Done => false,
        }
    }
}
