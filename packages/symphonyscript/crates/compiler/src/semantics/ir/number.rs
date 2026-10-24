use super::float_lit::FloatLit;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Number {
    Int(i128),
    Float(FloatLit),
}

impl Number {
    pub fn negated(&self) -> Option<Number> {
        match self {
            Number::Int(value) => value.checked_neg().map(Number::Int),
            Number::Float(value) => Some(Number::Float(value.negated())),
        }
    }
}
