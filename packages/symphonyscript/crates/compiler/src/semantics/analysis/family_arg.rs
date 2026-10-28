use super::arg_sig::ArgSig;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum FamilyArg {
    Default,
    Scalar(ArgSig),
    Array {
        rows: Vec<u32>,
        element: Option<ArgSig>,
    },
}
