use alloc::string::String;
use alloc::vec::Vec;

use super::attr_draft::AttrDraft;
use super::count::Count;
use crate::ids::{NodeKind, WireKind};

pub(crate) struct NodeDraft {
    pub(crate) name: String,
    pub(crate) doc: Option<String>,
    pub(crate) top_level: Option<Count>,
    pub(crate) entry: bool,
    pub(crate) children: Vec<(NodeKind, Count)>,
    pub(crate) incoming: Vec<(WireKind, Count)>,
    pub(crate) outgoing: Vec<(WireKind, Count)>,
    pub(crate) attrs: Vec<AttrDraft>,
}
