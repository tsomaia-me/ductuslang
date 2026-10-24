use crate::semantics::analysis::val::Val;
use crate::semantics::ir::Loc;

pub(in crate::semantics::analysis) enum Iteration {
    Range { next: i128, end: i128, loc: Loc },
    Values { values: Vec<Val>, next: usize },
}
