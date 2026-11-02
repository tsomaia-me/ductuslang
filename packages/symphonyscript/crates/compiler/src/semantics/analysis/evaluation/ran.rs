use crate::semantics::analysis::c_val::CVal;

pub(crate) enum Ran {
    Value(CVal),
    Suspended(u32),
    Abandoned,
    /// An untaken frame reached a call whose key is open at or below it, so it
    /// ends there and evaluates no later operand (ARCHITECTURE §7 stage 6).
    Ended,
}
