use std::collections::BTreeMap;

use crate::semantics::ir::Loc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum FrameKind {
    Locals,
    Body,
    ForVar,
}

#[derive(Clone, Debug)]
pub(super) struct Frame {
    pub names: BTreeMap<String, Loc>,
    pub parent: Option<usize>,
    pub kind: FrameKind,
}
