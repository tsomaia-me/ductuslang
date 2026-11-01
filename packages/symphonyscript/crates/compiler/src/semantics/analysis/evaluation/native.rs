use std::rc::Rc;

use symphonyscript_core::{NativeArg, NativeId, NativeOut, NativeType, Scalar, Ty, Value};

use crate::semantics::Internal;
use crate::semantics::analysis::c_array::CArray;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;

enum Argument {
    Value(Value),
    Elements(Vec<Value>),
    Text(Rc<str>),
}

impl Engine<'_> {
    pub(super) fn native_value(
        &mut self,
        native: NativeId,
        values: Vec<CVal>,
    ) -> Result<CVal, Internal> {
        let decl = self
            .vocabulary
            .native(native)
            .ok_or(Internal::missed("native function"))?;
        let signature = &decl.signature;
        let mut arguments = Vec::with_capacity(values.len());
        let mut rows_held = Vec::with_capacity(values.len());
        for (value, param) in values.into_iter().zip(&signature.params) {
            rows_held.push(match &value {
                CVal::Array(array) => array.len(),
                _ => 0,
            });
            let ty = native_ty(param);
            let array_param = !ty.is_scalar() || !matches!(param, NativeType::Value(_));
            let argument = match value {
                CVal::Str(text) if ty.scalar == Scalar::Str => Argument::Text(text),
                CVal::Array(array) if array_param => {
                    let mut elements = Vec::new();
                    for leaf in array.leaves() {
                        match value_of(&leaf, ty.scalar) {
                            Some(value) => elements.push(value),
                            None => return Ok(CVal::Error),
                        }
                    }
                    Argument::Elements(elements)
                }
                leaf if !array_param => match value_of(&leaf, ty.scalar) {
                    Some(value) => Argument::Value(value),
                    None => return Ok(CVal::Error),
                },
                _ => return Ok(CVal::Error),
            };
            arguments.push(argument);
        }
        let views: Vec<NativeArg<'_>> = arguments
            .iter()
            .map(|argument| match argument {
                Argument::Value(value) => NativeArg::Value(*value),
                Argument::Elements(elements) => NativeArg::Elements(elements),
                Argument::Text(text) => NativeArg::Text(text),
            })
            .collect();
        let result = native_ty(&signature.result);
        let variable = result.capacity.is_some()
            || matches!(
                signature.result,
                NativeType::Open(_) | NativeType::CapacityOf(..)
            );
        let inner: Vec<usize> = if variable {
            result.lengths.clone()
        } else {
            result
                .lengths
                .split_last()
                .map_or_else(Vec::new, |(_, inner)| inner.to_vec())
        }
        .iter()
        .map(|length| usize::try_from(*length).unwrap_or(usize::MAX))
        .collect();
        let row: usize = inner.iter().product();
        let slots = match &signature.result {
            NativeType::Value(ty) => ty.element_count(),
            NativeType::CapacityOf(_, position) => {
                let rows = match signature.params.get(*position) {
                    Some(NativeType::Value(ty)) => ty
                        .capacity
                        .or_else(|| ty.lengths.last().copied())
                        .map_or(0, |rows| usize::try_from(rows).unwrap_or(usize::MAX)),
                    Some(NativeType::Open(_)) => rows_held.get(*position).copied().unwrap_or(0),
                    Some(NativeType::CapacityOf(..)) | None => 0,
                };
                rows.saturating_mul(row.max(1))
            }
            NativeType::Open(_) => 0,
        };
        let mut buffer = if variable {
            Vec::new()
        } else {
            vec![zero(result.scalar); slots]
        };
        let mut text = String::new();
        let text_out = (result.scalar == Scalar::Str).then_some(&mut text);
        let mut out = if variable {
            NativeOut::growing(&mut buffer, slots, text_out)
        } else {
            NativeOut::new(&mut buffer, text_out)
        };
        (decl.function)(&views, &mut out);
        let written = out.len();
        #[cfg(test)]
        SLOTS.with(|held| held.set(held.get() + buffer.capacity() as u64));
        if result.scalar == Scalar::Str {
            return Ok(CVal::Str(Rc::from(text.as_str())));
        }
        if result.is_scalar() && !variable {
            return Ok(buffer.first().map_or(CVal::Error, cval_of));
        }
        let live = if variable {
            written - written % row.max(1)
        } else {
            slots
        };
        let mut level: Vec<CVal> = buffer.iter().take(live).map(cval_of).collect();
        for width in inner {
            if width == 0 {
                return Ok(CVal::Error);
            }
            level = level
                .chunks(width)
                .map(|chunk| CVal::Array(CArray::new(chunk.to_vec())))
                .collect();
        }
        Ok(CVal::Array(CArray::new(level)))
    }
}

pub(crate) fn native_ty(native: &NativeType) -> &Ty {
    match native {
        NativeType::Value(ty) | NativeType::Open(ty) | NativeType::CapacityOf(ty, _) => ty,
    }
}

fn zero(scalar: Scalar) -> Value {
    match scalar {
        Scalar::U8 => Value::U8(0),
        Scalar::I32 => Value::I32(0),
        Scalar::I64 => Value::I64(0),
        Scalar::F32 => Value::F32(0.0),
        Scalar::F64 => Value::F64(0.0),
        Scalar::Bool => Value::Bool(false),
        Scalar::Enum(id) => Value::Enum(id, 0),
        Scalar::Str => Value::Bool(false),
    }
}

fn value_of(value: &CVal, scalar: Scalar) -> Option<Value> {
    Some(match (value, scalar) {
        (CVal::Int(value), Scalar::U8) => Value::U8(u8::try_from(*value).ok()?),
        (CVal::Int(value), Scalar::I32) => Value::I32(i32::try_from(*value).ok()?),
        (CVal::Int(value), Scalar::I64) => Value::I64(i64::try_from(*value).ok()?),
        (CVal::F32(value), Scalar::F32) => Value::F32(*value),
        (CVal::F64(value), Scalar::F64) => Value::F64(*value),
        (CVal::Bool(value), Scalar::Bool) => Value::Bool(*value),
        (CVal::Enum(id, index), Scalar::Enum(_)) => Value::Enum(*id, *index),
        _ => return None,
    })
}

fn cval_of(value: &Value) -> CVal {
    match *value {
        Value::U8(value) => CVal::Int(i128::from(value)),
        Value::I32(value) => CVal::Int(i128::from(value)),
        Value::I64(value) => CVal::Int(i128::from(value)),
        Value::F32(value) => CVal::F32(symphonyscript_core::canonical_f32(value)),
        Value::F64(value) => CVal::F64(symphonyscript_core::canonical_f64(value)),
        Value::Bool(value) => CVal::Bool(value),
        Value::Enum(id, index) => CVal::Enum(id, index),
        Value::Str(_) => CVal::Error,
    }
}

#[cfg(test)]
thread_local! {
    static SLOTS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn native_slots() -> u64 {
    SLOTS.with(std::cell::Cell::get)
}
