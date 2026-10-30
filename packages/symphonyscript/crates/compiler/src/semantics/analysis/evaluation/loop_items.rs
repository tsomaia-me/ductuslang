use crate::semantics::analysis::c_array::CArray;
use crate::semantics::analysis::c_val::CVal;

pub(crate) enum LoopItems {
    Range { start: i128, count: usize },
    Array(CArray),
}

impl LoopItems {
    pub fn count(&self) -> usize {
        match self {
            LoopItems::Range { count, .. } => *count,
            LoopItems::Array(array) => array.len(),
        }
    }

    pub fn item(&self, position: usize) -> CVal {
        match self {
            LoopItems::Range { start, .. } => i128::try_from(position)
                .ok()
                .and_then(|position| start.checked_add(position))
                .map_or(CVal::Error, CVal::Int),
            LoopItems::Array(array) => array.get(position).cloned().unwrap_or(CVal::Error),
        }
    }
}
