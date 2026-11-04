use symphonyscript_core::Scalar;

use crate::semantics::ir::flat::SCALARS;
use crate::semantics::operators::EvalTy;

fn rank(ty: EvalTy) -> u8 {
    match ty {
        EvalTy::U8 => 1,
        EvalTy::I32 => 2,
        EvalTy::I64 => 3,
        EvalTy::F32 => 4,
        EvalTy::F64 => 5,
        EvalTy::Exact
        | EvalTy::Default
        | EvalTy::DefaultF32
        | EvalTy::DefaultF64
        | EvalTy::Bool
        | EvalTy::Enum(_)
        | EvalTy::Str
        | EvalTy::Node => 0,
    }
}

pub(crate) fn is_fixed_float(ty: EvalTy) -> bool {
    matches!(ty, EvalTy::F32 | EvalTy::F64)
}

pub(crate) fn float_for(ty: EvalTy) -> EvalTy {
    match ty {
        EvalTy::U8 | EvalTy::I32 => EvalTy::F32,
        EvalTy::I64 => EvalTy::F64,
        EvalTy::Exact
        | EvalTy::Default
        | EvalTy::F32
        | EvalTy::F64
        | EvalTy::DefaultF32
        | EvalTy::DefaultF64
        | EvalTy::Bool
        | EvalTy::Enum(_)
        | EvalTy::Str
        | EvalTy::Node => ty,
    }
}

pub(crate) fn combined(a: Option<EvalTy>, b: Option<EvalTy>, float: bool) -> Option<EvalTy> {
    let joined = match (a, b) {
        (None, None) => return None,
        (Some(one), None) | (None, Some(one)) => one,
        (Some(x), Some(y)) => {
            let (x, y) = if is_fixed_float(x) || is_fixed_float(y) {
                (float_for(x), float_for(y))
            } else {
                (x, y)
            };
            if rank(x) >= rank(y) { x } else { y }
        }
    };
    Some(if float { float_for(joined) } else { joined })
}

pub(crate) fn widens(own: EvalTy, target: EvalTy) -> bool {
    match own {
        EvalTy::U8 => true,
        EvalTy::I32 => !matches!(target, EvalTy::U8),
        EvalTy::I64 => matches!(target, EvalTy::I64 | EvalTy::F64 | EvalTy::DefaultF64),
        EvalTy::F32 => matches!(
            target,
            EvalTy::F32 | EvalTy::F64 | EvalTy::DefaultF32 | EvalTy::DefaultF64
        ),
        EvalTy::F64 => matches!(target, EvalTy::F64 | EvalTy::DefaultF64),
        EvalTy::Exact
        | EvalTy::Default
        | EvalTy::DefaultF32
        | EvalTy::DefaultF64
        | EvalTy::Bool
        | EvalTy::Enum(_)
        | EvalTy::Str
        | EvalTy::Node => true,
    }
}

pub(crate) fn target_ty(code: u32) -> Option<EvalTy> {
    Some(match SCALARS.get(code as usize)? {
        Scalar::U8 => EvalTy::U8,
        Scalar::I32 => EvalTy::I32,
        Scalar::I64 => EvalTy::I64,
        Scalar::F32 => EvalTy::F32,
        Scalar::F64 => EvalTy::F64,
        Scalar::Bool => EvalTy::Bool,
        Scalar::Enum(id) => EvalTy::Enum(*id),
        Scalar::Str => EvalTy::Str,
    })
}
