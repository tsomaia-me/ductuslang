use alloc::vec::Vec;

use super::binding::Binding;
use super::entry::Entry;
use crate::ids::{AttrId, NodeKind};
use crate::source::{Path, Span};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Instance {
    pub kind: NodeKind,
    pub path: Path,
    pub span: Span,
    pub expanded_from: Vec<Span>,
    pub attrs: Vec<(AttrId, Binding)>,
    pub gate: Option<Binding>,
    pub sequence: Vec<Entry>,
}
