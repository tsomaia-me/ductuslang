use crate::semantics::analysis::sig::Sig;

pub(super) enum Extra {
    None,
    Read(Sig),
    ToRead { reactive: bool },
    History(Sig),
    Arms(Vec<(Option<bool>, Option<Vec<u32>>)>),
}
