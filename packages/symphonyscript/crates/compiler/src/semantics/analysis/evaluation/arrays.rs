use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::c_array::CArray;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::map_leaves::map_leaves;
use crate::semantics::operators::convert;

impl Engine<'_> {
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
