use symphonyscript_core::Builtin;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum BuiltinName {
    Function(Builtin),
    Pi,
}

const FUNCTIONS: [Builtin; 17] = [
    Builtin::Abs,
    Builtin::Min,
    Builtin::Max,
    Builtin::Clamp,
    Builtin::Floor,
    Builtin::Ceil,
    Builtin::Round,
    Builtin::Sqrt,
    Builtin::Sin,
    Builtin::Cos,
    Builtin::Tan,
    Builtin::Exp,
    Builtin::Log,
    Builtin::Lerp,
    Builtin::Rand,
    Builtin::Len,
    Builtin::Bytes,
];

impl BuiltinName {
    pub fn all() -> impl Iterator<Item = BuiltinName> {
        FUNCTIONS
            .into_iter()
            .map(BuiltinName::Function)
            .chain([BuiltinName::Pi])
    }

    pub fn name(self) -> &'static str {
        match self {
            BuiltinName::Function(builtin) => builtin.name(),
            BuiltinName::Pi => "pi",
        }
    }
}
