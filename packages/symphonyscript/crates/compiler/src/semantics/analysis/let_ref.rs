use crate::semantics::ir::LetId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum LetRef {
    Top(LetId),
    Local(u32),
}
