use std::rc::Rc;

use super::ids::{NodeId, ScopeId};
use super::item_ref::ItemRef;
use super::query::Query;
use crate::semantics::ir::{Ident, Loc};

#[derive(Clone, Debug)]
pub(crate) struct FrameRead {
    pub start: NodeId,
    pub arms: Vec<(ItemRef, NodeId)>,
    pub open: Vec<(ItemRef, Rc<[Query]>)>,
    pub name: Ident,
    pub loc: Loc,
    pub position: usize,
    pub scope: ScopeId,
}
