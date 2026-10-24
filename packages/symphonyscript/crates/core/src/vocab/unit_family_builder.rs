use alloc::rc::Rc;
use alloc::string::String;
use core::cell::RefCell;

use super::family_decl::FamilyDecl;
use super::unit_decl::UnitDecl;
use crate::ids::FamilyId;

pub struct UnitFamilyBuilder {
    draft: Rc<RefCell<FamilyDecl>>,
    id: FamilyId,
    last_unit: usize,
}

impl UnitFamilyBuilder {
    pub(crate) fn new(draft: Rc<RefCell<FamilyDecl>>, id: FamilyId) -> Self {
        Self {
            draft,
            id,
            last_unit: 0,
        }
    }

    pub fn unit(mut self, name: &str, scale: f64) -> Self {
        let mut draft = self.draft.borrow_mut();
        draft.units.push(UnitDecl {
            name: String::from(name),
            scale,
            doc: None,
        });
        self.last_unit = draft.units.len() - 1;
        drop(draft);
        self
    }

    pub fn doc(self, text: &str) -> Self {
        if let Some(unit) = self.draft.borrow_mut().units.get_mut(self.last_unit) {
            unit.doc = Some(String::from(text));
        }
        self
    }

    pub fn id(&self) -> FamilyId {
        self.id
    }
}

impl From<UnitFamilyBuilder> for FamilyId {
    fn from(builder: UnitFamilyBuilder) -> Self {
        builder.id
    }
}

impl From<&UnitFamilyBuilder> for FamilyId {
    fn from(builder: &UnitFamilyBuilder) -> Self {
        builder.id
    }
}
