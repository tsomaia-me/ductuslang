use super::ids::{NodeId, ScopeId};
use super::item_ref::ItemRef;
use crate::semantics::ir::Ident;

#[derive(Clone, Debug)]
pub(crate) struct FrameRead {
    pub start: NodeId,
    pub arms: Vec<(ItemRef, NodeId)>,
    pub name: Ident,
    pub scope: ScopeId,
}
