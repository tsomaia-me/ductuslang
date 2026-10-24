use crate::semantics::ir::Ident;

pub(super) enum BindFault<'a> {
    TooMany { given: usize },
    Unknown(&'a Ident),
    Duplicate(&'a Ident),
}
