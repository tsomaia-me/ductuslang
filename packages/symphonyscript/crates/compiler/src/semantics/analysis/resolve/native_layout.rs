use symphonyscript_core::{NativeType, Scalar};

use crate::semantics::analysis::layout::Layout;

pub(crate) fn native_layout(native: &NativeType) -> Option<Layout> {
    match native {
        NativeType::Value(ty) => {
            if ty.scalar == Scalar::Str && ty.is_scalar() {
                return None;
            }
            match ty.capacity {
                Some(_) => Some(Layout::variable(ty.lengths.clone())),
                None if ty.lengths.is_empty() => None,
                None => Some(Layout::fixed(ty.lengths.clone())),
            }
        }
        NativeType::Open(ty) | NativeType::CapacityOf(ty, _) => {
            Some(Layout::variable(ty.lengths.clone()))
        }
    }
}
