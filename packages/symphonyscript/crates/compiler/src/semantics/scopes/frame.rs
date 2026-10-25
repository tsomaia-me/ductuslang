use std::collections::BTreeMap;

use super::frame_kind::FrameKind;
use crate::semantics::ir::{Loc, UnknownNames};

#[derive(Clone, Debug)]
pub(super) struct Frame {
    pub names: BTreeMap<String, Loc>,
    pub parent: Option<usize>,
    pub kind: FrameKind,
    pub unknown: UnknownNames,
}
