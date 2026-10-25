use core::hash::{Hash, Hasher};
use core::mem::discriminant;

use crate::ids::{EnumId, StrId};

#[derive(Clone, Copy, Debug)]
pub enum Value {
    U8(u8),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
    Enum(EnumId, u16),
    Str(StrId),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::U8(a), Value::U8(b)) => a == b,
            (Value::I32(a), Value::I32(b)) => a == b,
            (Value::I64(a), Value::I64(b)) => a == b,
            (Value::F32(a), Value::F32(b)) => a.to_bits() == b.to_bits(),
            (Value::F64(a), Value::F64(b)) => a.to_bits() == b.to_bits(),
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Enum(a, x), Value::Enum(b, y)) => a == b && x == y,
            (Value::Str(a), Value::Str(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        discriminant(self).hash(state);
        match self {
            Value::U8(v) => v.hash(state),
            Value::I32(v) => v.hash(state),
            Value::I64(v) => v.hash(state),
            Value::F32(v) => v.to_bits().hash(state),
            Value::F64(v) => v.to_bits().hash(state),
            Value::Bool(v) => v.hash(state),
            Value::Enum(e, v) => {
                e.hash(state);
                v.hash(state);
            }
            Value::Str(s) => s.hash(state),
        }
    }
}
