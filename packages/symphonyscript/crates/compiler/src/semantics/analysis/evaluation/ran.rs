use crate::semantics::analysis::c_val::CVal;

pub(crate) enum Ran {
    Value(CVal),
    Suspended(u32),
    Abandoned,
}
