use alloc::string::String;
use alloc::vec::Vec;

use super::attr_draft::AttrDraft;
use crate::ids::NodeKind;

pub(crate) struct WireDraft {
    pub(crate) name: String,
    pub(crate) doc: Option<String>,
    pub(crate) connects: Vec<(NodeKind, NodeKind)>,
    pub(crate) cyclic: bool,
    pub(crate) attrs: Vec<AttrDraft>,
}
