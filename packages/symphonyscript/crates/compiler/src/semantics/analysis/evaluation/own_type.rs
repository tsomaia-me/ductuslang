use super::is_float_ty::is_float_ty;
use super::of_scalar::of_scalar;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::float_for::float_for;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::width::Width;

pub(crate) fn own_type(sig: &Sig, ty: EvalTy) -> EvalTy {
    match sig.width {
        Width::Fixed(scalar) => of_scalar(scalar),
        Width::Widened(scalar) => {
            if is_float_ty(ty) {
                ty
            } else {
                of_scalar(float_for(scalar))
            }
        }
        Width::Flexible | Width::Adaptive => match sig.kind {
            Kind::Int => {
                if is_float_ty(ty) {
                    ty
                } else {
                    EvalTy::Exact
                }
            }
            Kind::Float => {
                if is_float_ty(ty) {
                    ty
                } else {
                    EvalTy::Default
                }
            }
            Kind::Bool => EvalTy::Bool,
            Kind::Str => EvalTy::Str,
            Kind::Node => EvalTy::Node,
            Kind::Enum(id) => EvalTy::Enum(id),
            Kind::Bare | Kind::Any => ty,
            Kind::Error => ty,
        },
    }
}
