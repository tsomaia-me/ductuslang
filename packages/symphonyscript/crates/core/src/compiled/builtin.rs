#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Builtin {
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
}

impl Builtin {
    pub fn name(self) -> &'static str {
        match self {
            Builtin::Abs => "abs",
            Builtin::Min => "min",
            Builtin::Max => "max",
            Builtin::Clamp => "clamp",
            Builtin::Floor => "floor",
            Builtin::Ceil => "ceil",
            Builtin::Round => "round",
            Builtin::Sqrt => "sqrt",
            Builtin::Sin => "sin",
            Builtin::Cos => "cos",
            Builtin::Tan => "tan",
            Builtin::Exp => "exp",
            Builtin::Log => "log",
            Builtin::Lerp => "lerp",
            Builtin::Rand => "rand",
            Builtin::Len => "len",
            Builtin::Bytes => "bytes",
        }
    }
}
