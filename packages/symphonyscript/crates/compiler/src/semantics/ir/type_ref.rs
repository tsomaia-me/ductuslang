use symphonyscript_core::Scalar;

use super::ident::Ident;
use super::loc::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum TypeBase {
    Scalar(Scalar),
    Enum(Ident),
    Error,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct TypeRef {
    pub base: TypeBase,
    pub lengths: Vec<(Option<u32>, Loc)>,
    pub loc: Loc,
}
