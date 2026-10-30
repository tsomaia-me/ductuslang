use super::layout::Layout;
use super::sig::Sig;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct LoopTypes {
    pub accumulator: (Sig, Option<Layout>),
    pub runtime: bool,
}
