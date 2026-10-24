use super::loc::Loc;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Ident {
    pub text: String,
    pub loc: Loc,
}
