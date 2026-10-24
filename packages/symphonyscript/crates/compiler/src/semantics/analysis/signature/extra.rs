use crate::semantics::analysis::sig::Sig;

pub(super) enum Extra {
    None,
    Read(Sig),
    ToRead { reactive: bool },
}
