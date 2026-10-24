use symphonyscript_core::Builtin;

pub(super) fn is_float_builtin(builtin: Builtin) -> bool {
    !matches!(
        builtin,
        Builtin::Abs | Builtin::Min | Builtin::Max | Builtin::Clamp | Builtin::Rand
    )
}
