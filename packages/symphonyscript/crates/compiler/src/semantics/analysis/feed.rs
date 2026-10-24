use symphonyscript_core::FamilyId;

use crate::semantics::ir::Loc;

pub(crate) type Feed = (FamilyId, usize, Loc);
