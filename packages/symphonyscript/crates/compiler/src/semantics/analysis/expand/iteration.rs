use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::Loc;

pub(in crate::semantics::analysis) enum Iteration {
    Range {
        next: i128,
        end: i128,
        loc: Loc,
    },
    Values {
        values: Vec<Val>,
        next: usize,
        loc: Loc,
    },
    Elements {
        base: NodeId,
        length: usize,
        next: usize,
        loc: Loc,
    },
}

impl Iteration {
    pub(in crate::semantics::analysis) fn is_empty(&self) -> bool {
        match self {
            Iteration::Range { next, end, .. } => next >= end,
            Iteration::Values { values, next, .. } => *next >= values.len(),
            Iteration::Elements { length, next, .. } => next >= length,
        }
    }

    pub(in crate::semantics::analysis) fn loc(&self) -> Loc {
        match self {
            Iteration::Range { loc, .. }
            | Iteration::Values { loc, .. }
            | Iteration::Elements { loc, .. } => *loc,
        }
    }
}
