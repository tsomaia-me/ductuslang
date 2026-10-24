use symphonyscript_core::Scalar;

use super::binding_kind::BindingKind;
use super::ids::NodeId;
use crate::semantics::ir::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Binding {
    pub kind: BindingKind,
    pub init: NodeId,
    pub annotation: Option<Scalar>,
    pub name: String,
    pub loc: Loc,
}
