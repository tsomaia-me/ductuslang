use alloc::rc::Rc;
use alloc::string::String;
use core::cell::RefCell;

use super::enum_decl::EnumDecl;
use crate::ids::EnumId;

pub struct EnumBuilder {
    draft: Rc<RefCell<EnumDecl>>,
    id: EnumId,
}

impl EnumBuilder {
    pub(crate) fn new(draft: Rc<RefCell<EnumDecl>>, id: EnumId) -> Self {
        Self { draft, id }
    }

    pub fn doc(self, text: &str) -> Self {
        self.draft.borrow_mut().doc = Some(String::from(text));
        self
    }

    pub fn id(&self) -> EnumId {
        self.id
    }
}

impl From<EnumBuilder> for EnumId {
    fn from(builder: EnumBuilder) -> Self {
        builder.id
    }
}

impl From<&EnumBuilder> for EnumId {
    fn from(builder: &EnumBuilder) -> Self {
        builder.id
    }
}
