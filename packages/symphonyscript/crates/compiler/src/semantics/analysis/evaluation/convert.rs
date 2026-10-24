use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) fn convert(value: CVal, target: EvalTy) -> CVal {
    let target = match target {
        EvalTy::DefaultF32 => EvalTy::F32,
        EvalTy::DefaultF64 => EvalTy::F64,
        other => other,
    };
    match (value, target) {
        (CVal::Int(value), EvalTy::I32) => {
            CVal::Int(value.clamp(i128::from(i32::MIN), i128::from(i32::MAX)))
        }
        (CVal::Int(value), EvalTy::I64) => {
            CVal::Int(value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)))
        }
        (CVal::Int(value), EvalTy::F32) => CVal::F32(value as f32),
        (CVal::Int(value), EvalTy::F64) => CVal::F64(value as f64),
        (CVal::F32(value), EvalTy::I32) => CVal::Int(i128::from(value as i32)),
        (CVal::F32(value), EvalTy::I64) => CVal::Int(i128::from(value as i64)),
        (CVal::F64(value), EvalTy::I32) => CVal::Int(i128::from(value as i32)),
        (CVal::F64(value), EvalTy::I64) => CVal::Int(i128::from(value as i64)),
        (CVal::F32(value), EvalTy::F64) => CVal::F64(f64::from(value)),
        (CVal::F64(value), EvalTy::F32) => CVal::F32(value as f32),
        (value, _) => value,
    }
}
