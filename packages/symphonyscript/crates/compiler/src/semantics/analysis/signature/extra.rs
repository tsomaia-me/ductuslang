use crate::semantics::analysis::sig::Sig;

pub(crate) enum Extra {
    None,
    Read(Sig),
    ToRead { reactive: bool },
    History(Sig),
    Arms(Vec<(Option<bool>, Option<Vec<u32>>)>),
}
