use super::ids::NodeId;
use super::layout::Layout;
use super::sig::Sig;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Slot {
    pub sig: Sig,
    pub layout: Option<Layout>,
    pub capacity: Option<u32>,
    pub source: Option<NodeId>,
    pub annotated: bool,
}
