use super::ids::{BlockId, ExprId};

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct WhenArm {
    pub guard: ExprId,
    pub body: BlockId,
}
