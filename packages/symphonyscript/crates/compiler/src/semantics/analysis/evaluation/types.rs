use symphonyscript_core::Scalar;

use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::sig::{Kind, Sig, Width, common, is_integer, meet_flexible};

pub(super) fn of_scalar(scalar: Scalar) -> EvalTy {
    match scalar {
        Scalar::I32 => EvalTy::I32,
        Scalar::I64 => EvalTy::I64,
        Scalar::F32 => EvalTy::F32,
        Scalar::F64 => EvalTy::F64,
        Scalar::Bool => EvalTy::Bool,
        Scalar::Enum(id) => EvalTy::Enum(id),
        Scalar::Str => EvalTy::Str,
    }
}

pub(super) fn is_int_ty(ty: EvalTy) -> bool {
    matches!(ty, EvalTy::Exact | EvalTy::I32 | EvalTy::I64)
}

pub(super) fn is_float_ty(ty: EvalTy) -> bool {
    matches!(ty, EvalTy::F32 | EvalTy::F64)
}

pub(crate) fn own_type(sig: &Sig, ty: EvalTy) -> EvalTy {
    match sig.width {
        Width::Fixed(scalar) => of_scalar(scalar),
        _ => match sig.kind {
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

pub(super) enum Pair {
    Ty(EvalTy),
    FlexibleFloat,
}

pub(super) fn pair_type(a: &Sig, b: &Sig) -> Pair {
    match (a.kind, b.kind) {
        (Kind::Bool, _) => return Pair::Ty(EvalTy::Bool),
        (Kind::Str, _) => return Pair::Ty(EvalTy::Str),
        (Kind::Enum(id), _) | (_, Kind::Enum(id)) => return Pair::Ty(EvalTy::Enum(id)),
        _ => {}
    }
    let fixed = |sig: &Sig| match sig.width {
        Width::Fixed(scalar) => Some(scalar),
        _ => None,
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

pub(super) fn int_bits(ty: EvalTy) -> Option<u32> {
    match ty {
        EvalTy::I32 => Some(32),
        EvalTy::I64 => Some(64),
        _ => None,
    }
}

pub(super) fn _unused(scalar: Scalar) -> bool {
    is_integer(scalar)
}
