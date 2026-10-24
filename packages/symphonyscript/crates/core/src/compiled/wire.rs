use alloc::vec::Vec;

use super::binding::Binding;
use super::target::Target;
use crate::ids::{AttrId, InstanceId, WireKind};
use crate::source::{Path, Span};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Wire {
    pub kind: WireKind,
    pub path: Path,
    pub span: Span,
    pub expanded_from: Vec<Span>,
    pub source: InstanceId,
    pub target: Target,
    pub attrs: Vec<(AttrId, Binding)>,
    pub gate: Option<Binding>,
}
