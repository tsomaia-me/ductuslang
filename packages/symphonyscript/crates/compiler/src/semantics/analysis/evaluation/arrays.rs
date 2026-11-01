use super::convert::convert;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::c_array::CArray;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::map_leaves::map_leaves;
use crate::semantics::analysis::resolve::NOT_AN_ARRAY;

impl Engine<'_> {
    pub(super) fn joined(
        &mut self,
        node: NodeId,
        left: &CArray,
        right: &CArray,
    ) -> Result<CVal, Internal> {
        match CArray::join(left, right) {
            Ok(array) => Ok(CVal::Array(array)),
            Err(Code::LengthLimit) => {
                let total = u128::try_from(left.len())
                    .unwrap_or(u128::MAX)
                    .saturating_add(u128::try_from(right.len()).unwrap_or(u128::MAX));
                self.error_at_node(
                    Code::LengthLimit,
                    node,
                    format!(
                        "this join holds {total} elements, and an array's length and capacity are below 2³¹"
                    ),
                )?;
                Ok(CVal::Error)
            }
            Err(code) => {
                self.error_at_node(
                    code,
                    node,
                    "the rows of the joined arrays have different shapes",
                )?;
                Ok(CVal::Error)
            }
        }
    }

    pub(super) fn slice_value(
        &mut self,
        base: NodeId,
        value: CVal,
        start: Option<(i128, bool)>,
        end: Option<(i128, bool)>,
    ) -> Result<CVal, Internal> {
        let array = match value {
            CVal::Array(array) => array,
            CVal::Error => return Ok(CVal::Error),
            CVal::Int(_)
            | CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_) => {
                self.error_at_node(Code::NotAnArray, base, "only an array is sliced")?;
                return Ok(CVal::Error);
            }
        };
        let length = i128::try_from(array.len()).unwrap_or(i128::MAX);
        let resolve = |bound: Option<(i128, bool)>, absent: i128| match bound {
            None => Some(absent),
            Some((value, from_end)) => {
                let position = if from_end {
                    length.checked_sub(value)?
                } else {
                    value
                };
                Some(position.clamp(0, length))
            }
        };
        let (Some(start), Some(end)) = (resolve(start, 0), resolve(end, length)) else {
            return Ok(CVal::Error);
        };
        let end = end.max(start);
        let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
            return Ok(CVal::Error);
        };
        Ok(CVal::Array(CArray::slice(&array, start, end)))
    }

    pub(super) fn bound_position(
        &mut self,
        node: NodeId,
        value: CVal,
        from_end: bool,
    ) -> Result<Option<(i128, bool)>, Internal> {
        Ok(match value {
            CVal::Int(value) if i64::try_from(value).is_err() => {
                self.error_at_node(
                    Code::IndexOutOfRange,
                    node,
                    crate::semantics::analysis::outside_i64::outside_i64(
                        crate::semantics::analysis::i64_position::I64Position::SliceBound,
                        value,
                    ),
                )?;
                None
            }
            CVal::Int(value) => Some((value, from_end)),
            CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_)
            | CVal::Array(_)
            | CVal::Error => None,
        })
    }

    pub(super) fn length_of(&mut self, operand: NodeId, value: CVal) -> Result<CVal, Internal> {
        Ok(match value {
            CVal::Array(array) => CVal::Int(i128::try_from(array.len()).unwrap_or(i128::MAX)),
            CVal::Error => CVal::Error,
            CVal::Int(_)
            | CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_) => {
                self.error_at_node(Code::NotAnArray, operand, NOT_AN_ARRAY)?;
                CVal::Error
            }
        })
    }

    pub(super) fn bytes_of(&mut self, operand: NodeId, value: CVal) -> Result<CVal, Internal> {
        Ok(match value {
            CVal::Str(text) => CVal::Array(CArray::new(
                text.bytes()
                    .map(|byte| CVal::Int(i128::from(byte)))
                    .collect(),
            )),
            CVal::Array(_) => map_leaves(value, &mut FastMap::default(), |leaf| {
                Ok::<CVal, Internal>(match leaf {
                    CVal::Int(_) => convert(leaf, EvalTy::U8),
                    _ => CVal::Error,
                })
            })?,
            CVal::Error => CVal::Error,
            CVal::Int(_)
            | CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Node(_) => {
                self.error_at_node(
                    Code::OperandType,
                    operand,
                    "`bytes` takes a string or an integer array",
                )?;
                CVal::Error
            }
        })
    }
}
