use crate::semantics::analysis::c_val::CVal;

pub(crate) enum LoopFound {
    Value(CVal),
    Missing,
    Pending,
}
