use alloc::string::String;

use super::attr_default::AttrDefault;
use crate::ids::{EnumId, FamilyId};
use crate::types::Ty;

#[derive(Clone)]
pub(crate) struct AttrDraft {
    pub(crate) name: String,
    pub(crate) ty: Ty,
    pub(crate) default: Option<AttrDefault>,
    pub(crate) enum_default: Option<(EnumId, String)>,
    pub(crate) unit: Option<(FamilyId, String)>,
    pub(crate) flag: Option<char>,
    pub(crate) is_default: bool,
    pub(crate) doc: Option<String>,
}

impl AttrDraft {
    pub(crate) fn new(name: &str, ty: Ty) -> Self {
        Self {
            name: String::from(name),
            ty,
            default: None,
            enum_default: None,
            unit: None,
            flag: None,
            is_default: false,
            doc: None,
        }
    }
}
