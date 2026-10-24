use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) fn ty_name(ty: EvalTy) -> &'static str {
    match ty {
        EvalTy::I32 => "i32",
        EvalTy::I64 => "i64",
        EvalTy::F32 | EvalTy::DefaultF32 => "f32",
        EvalTy::F64 | EvalTy::DefaultF64 => "f64",
        EvalTy::Bool => "bool",
        EvalTy::Exact | EvalTy::Default | EvalTy::Enum(_) | EvalTy::Str | EvalTy::Node => {
            "the type"
        }
    }
}
