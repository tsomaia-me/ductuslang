#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Builtin {
    Abs,
    Min,
    Max,
    Clamp,
    Floor,
    Ceil,
    Round,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Exp,
    Log,
    Lerp,
    Rand,
    Len,
    Bytes,
    Capacity,
}

impl Builtin {
    pub fn name(self) -> &'static str {
        match self {
            Builtin::Len => "size",
            Builtin::Bytes => "bytes",
            Builtin::Capacity => "capacity",
            other => other.core().map_or("", symphonyscript_core::Builtin::name),
        }
    }

    pub fn core(self) -> Option<symphonyscript_core::Builtin> {
        use symphonyscript_core::Builtin as Core;
        Some(match self {
            Builtin::Abs => Core::Abs,
            Builtin::Min => Core::Min,
            Builtin::Max => Core::Max,
            Builtin::Clamp => Core::Clamp,
            Builtin::Floor => Core::Floor,
            Builtin::Ceil => Core::Ceil,
            Builtin::Round => Core::Round,
            Builtin::Sqrt => Core::Sqrt,
            Builtin::Sin => Core::Sin,
            Builtin::Cos => Core::Cos,
            Builtin::Tan => Core::Tan,
            Builtin::Exp => Core::Exp,
            Builtin::Log => Core::Log,
            Builtin::Lerp => Core::Lerp,
            Builtin::Rand => Core::Rand,
            Builtin::Len | Builtin::Bytes | Builtin::Capacity => return None,
        })
    }
}
