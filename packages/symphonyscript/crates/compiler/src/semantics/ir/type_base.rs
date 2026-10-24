use symphonyscript_core::Scalar;

use super::ident::Ident;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum TypeBase {
    Scalar(Scalar),
    Enum(Ident),
    Error,
}
