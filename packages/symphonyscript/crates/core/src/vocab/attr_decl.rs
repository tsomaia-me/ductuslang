use alloc::string::String;

use super::attr_default::AttrDefault;
use super::attr_unit::AttrUnit;
use crate::types::Ty;

#[derive(Clone, PartialEq, Debug)]
pub struct AttrDecl {
    pub name: String,
    pub ty: Ty,
    pub default: Option<AttrDefault>,
    pub unit: Option<AttrUnit>,
    pub flag: Option<char>,
    pub doc: Option<String>,
}
