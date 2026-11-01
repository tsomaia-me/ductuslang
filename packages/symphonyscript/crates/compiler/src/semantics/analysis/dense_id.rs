use super::ids::{NodeId, ScopeId};

pub(crate) trait DenseId: Copy {
    fn index(self) -> usize;
}

impl DenseId for NodeId {
    fn index(self) -> usize {
        NodeId::index(self)
    }
}

impl DenseId for ScopeId {
    fn index(self) -> usize {
        ScopeId::index(self)
    }
}

impl DenseId for crate::semantics::ir::ExprId {
    fn index(self) -> usize {
        crate::semantics::ir::ExprId::index(self)
    }
}
