use std::rc::Rc;

use super::ids::{NodeId, ScopeId};
use super::item_ref::ItemRef;
use super::query::Query;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::{Ident, Loc};

#[derive(Debug)]
pub(crate) struct FrameRead {
    pub start: NodeId,
    pub arms: Vec<(ItemRef, NodeId)>,
    pub open: Vec<(ItemRef, Rc<[Query]>)>,
    pub name: Ident,
    pub loc: Loc,
    pub position: usize,
    pub scope: ScopeId,
}

impl Clone for FrameRead {
    fn clone(&self) -> Self {
        CountedWalk::FrameReadClone.step();
        FrameRead {
            start: self.start,
            arms: self.arms.clone(),
            open: self.open.clone(),
            name: self.name.clone(),
            loc: self.loc,
            position: self.position,
            scope: self.scope,
        }
    }
}
