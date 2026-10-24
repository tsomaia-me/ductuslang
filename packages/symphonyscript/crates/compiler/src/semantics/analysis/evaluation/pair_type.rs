use super::of_scalar::of_scalar;
use super::pair::Pair;
use crate::semantics::analysis::common::common;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::float_for::float_for;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::meet_flexible::meet_flexible;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::width::Width;

pub(super) fn pair_type(a: &Sig, b: &Sig) -> Pair {
    match (a.kind, b.kind) {
        (Kind::Bool, _) => return Pair::Ty(EvalTy::Bool),
        (Kind::Str, _) => return Pair::Ty(EvalTy::Str),
        (Kind::Enum(id), _) | (_, Kind::Enum(id)) => return Pair::Ty(EvalTy::Enum(id)),
        _ => {}
    }
    let fixed = |sig: &Sig| match sig.width {
        Width::Fixed(scalar) => Some(scalar),
        Width::Widened(scalar) => Some(float_for(scalar)),
        Width::Flexible | Width::Adaptive => None,
    };
    match (fixed(a), fixed(b)) {
        (Some(x), Some(y)) => Pair::Ty(of_scalar(common(x, y).unwrap_or(x))),
        (Some(x), None) => Pair::Ty(of_scalar(meet_flexible(x, b.kind))),
        (None, Some(y)) => Pair::Ty(of_scalar(meet_flexible(y, a.kind))),
        (None, None) => {
            if a.kind == Kind::Float || b.kind == Kind::Float {
                Pair::FlexibleFloat
            } else {
                Pair::Ty(EvalTy::Exact)
            }
        }
    }
}
