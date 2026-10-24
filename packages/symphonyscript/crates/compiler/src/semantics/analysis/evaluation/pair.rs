use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) enum Pair {
    Ty(EvalTy),
    FlexibleFloat,
}
