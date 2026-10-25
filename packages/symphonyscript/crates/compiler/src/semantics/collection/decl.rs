use symphonyscript_core::{EnumId, NativeId, NodeKind, WireKind};

use super::builtin_name::BuiltinName;
use crate::semantics::ir::{FnId, InputId, LetId, PatternId, PlacementId, StateId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Decl {
    Input(InputId),
    Let(LetId),
    State(StateId),
    Fn(FnId),
    Native(NativeId),
    Pattern(PatternId),
    Instance(PlacementId),
    Builtin(BuiltinName),
    Node(NodeKind),
    Wire(WireKind),
    Enum(EnumId),
}
