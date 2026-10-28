use super::convert::convert;
use super::ctx::Ctx;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::c_array::CArray;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::map_leaves::map_leaves;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::some::some;

type SliceBounds = (Option<(NodeId, bool)>, Option<(NodeId, bool)>);

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

    pub(super) fn join(
        &mut self,
        node: NodeId,
        (lhs, rhs): (NodeId, NodeId),
        typed: (EvalTy, Option<EvalTy>),
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let left = self.typed_child(lhs, typed, ctx, need)?;
        let right = self.typed_child(rhs, typed, ctx, need)?;
        let (left, right) = (some!(left), some!(right));
        Ok(Some(match (left, right) {
            (CVal::Array(left), CVal::Array(right)) => self.joined(node, &left, &right)?,
            (CVal::Error, _) | (_, CVal::Error) => CVal::Error,
            _ => {
                self.error_at_node(
                    Code::ArrayOperand,
                    node,
                    "`+` joins two arrays, and one of these is a single value",
                )?;
                CVal::Error
            }
        }))
    }

    pub(super) fn sliced(
        &mut self,
        base: NodeId,
        (lower, upper): SliceBounds,
        typed: (EvalTy, Option<EvalTy>),
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let value = self.typed_child(base, typed, ctx, need)?;
        let start = self.position_of(lower, ctx, need)?;
        let end = self.position_of(upper, ctx, need)?;
        let (value, start, end) = (some!(value), some!(start), some!(end));
        let array = match value {
            CVal::Array(array) => array,
            CVal::Error => return Ok(Some(CVal::Error)),
            CVal::Int(_)
            | CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_) => {
                self.error_at_node(Code::NotAnArray, base, "only an array is sliced")?;
                return Ok(Some(CVal::Error));
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
            return Ok(Some(CVal::Error));
        };
        let end = end.max(start);
        let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
            return Ok(Some(CVal::Error));
        };
        Ok(Some(CVal::Array(CArray::slice(&array, start, end))))
    }

    fn position_of(
        &mut self,
        bound: Option<(NodeId, bool)>,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<(i128, bool)>>, Internal> {
        let Some((node, from_end)) = bound else {
            return Ok(Some(None));
        };
        let sig = some!(self.sig(node, need));
        let ty = super::own_type::own_type(&sig, EvalTy::Exact);
        Ok(Some(match some!(self.child(node, ty, ctx, need)?) {
            CVal::Int(value) if i64::try_from(value).is_err() => {
                self.error_at_node(
                    crate::diagnostics::Code::IndexOutOfRange,
                    node,
                    crate::semantics::analysis::outside_i64::outside_i64(
                        crate::semantics::analysis::i64_position::I64Position::SliceBound,
                        value,
                    ),
                )?;
                return Ok(Some(None));
            }
            CVal::Int(value) => Some((value, from_end)),
            CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_)
            | CVal::Array(_)
            | CVal::Error => return Ok(Some(None)),
        }))
    }

    pub(super) fn length(
        &mut self,
        operand: NodeId,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        Ok(Some(
            match some!(self.child(operand, EvalTy::Default, ctx, need)?) {
                CVal::Array(array) => CVal::Int(i128::try_from(array.len()).unwrap_or(i128::MAX)),
                CVal::Error => CVal::Error,
                CVal::Int(_)
                | CVal::F32(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_) => {
                    self.error_at_node(Code::NotAnArray, operand, "`len` takes an array")?;
                    CVal::Error
                }
            },
        ))
    }

    pub(super) fn bytes(
        &mut self,
        operand: NodeId,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let sig = some!(self.sig(operand, need));
        let ty = match sig.kind {
            Kind::Str => EvalTy::Str,
            _ => super::own_type::own_type(&sig, EvalTy::Exact),
        };
        let value = some!(self.child(operand, ty, ctx, need)?);
        Ok(Some(match value {
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
        }))
    }
}
