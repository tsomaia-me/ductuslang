use alloc::string::String;
use alloc::vec::Vec;

use super::attr_decl::AttrDecl;
use super::count::Count;
use crate::ids::{AttrId, NodeKind, WireKind};

#[derive(Clone, PartialEq, Debug)]
pub struct NodeKindDecl {
    pub name: String,
    pub attrs: Vec<AttrDecl>,
    pub default_attr: Option<AttrId>,
    pub top_level: Option<Count>,
    pub entry: bool,
    pub children: Vec<(NodeKind, Count)>,
    pub incoming: Vec<(WireKind, Count)>,
    pub outgoing: Vec<(WireKind, Count)>,
    pub doc: Option<String>,
}
