use symphonyscript_core::{EnumId, NodeKind, WireKind};

use super::builtin_name::BuiltinName;
use crate::semantics::ir::{FnId, InputId, LetId, PatternId, PlacementId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Decl {
    Input(InputId),
    Let(LetId),
    Fn(FnId),
    Pattern(PatternId),
    Instance(PlacementId),
    Builtin(BuiltinName),
    Node(NodeKind),
    Wire(WireKind),
    Enum(EnumId),
}
