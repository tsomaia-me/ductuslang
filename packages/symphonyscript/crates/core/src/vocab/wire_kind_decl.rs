use alloc::string::String;
use alloc::vec::Vec;

use super::attr_decl::AttrDecl;
use crate::ids::{AttrId, NodeKind};

#[derive(Clone, PartialEq, Debug)]
pub struct WireKindDecl {
    pub name: String,
    pub attrs: Vec<AttrDecl>,
    pub default_attr: Option<AttrId>,
    pub connects: Vec<(NodeKind, NodeKind)>,
    pub cyclic: bool,
    pub doc: Option<String>,
}
