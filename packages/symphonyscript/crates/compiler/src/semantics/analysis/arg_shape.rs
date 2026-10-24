use super::arg_sig::ArgSig;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ArgShape {
    Default,
    Scalar(ArgSig),
    Array(Vec<u32>, Vec<ArgSig>),
}
