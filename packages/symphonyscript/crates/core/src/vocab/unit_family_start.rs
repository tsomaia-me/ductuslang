use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use core::cell::RefCell;

use super::family_decl::FamilyDecl;
use super::unit_decl::UnitDecl;
use super::unit_family_builder::UnitFamilyBuilder;
use super::vocabulary_builder::VocabularyBuilder;
use crate::ids::FamilyId;

pub struct UnitFamilyStart<'a> {
    builder: &'a mut VocabularyBuilder,
    name: String,
    doc: Option<String>,
}

impl<'a> UnitFamilyStart<'a> {
    pub(crate) fn new(builder: &'a mut VocabularyBuilder, name: &str) -> Self {
        Self {
            builder,
            name: String::from(name),
            doc: None,
        }
    }

    pub fn doc(mut self, text: &str) -> Self {
        self.doc = Some(String::from(text));
        self
    }

    pub fn base(self, name: &str) -> UnitFamilyBuilder {
        let id = FamilyId::new(self.builder.token, self.builder.families.len() as u32);
        let draft = Rc::new(RefCell::new(FamilyDecl {
            name: self.name,
            units: vec![UnitDecl {
                name: String::from(name),
                scale: 1.0,
                doc: None,
            }],
            doc: self.doc,
        }));
        self.builder.families.push(Rc::clone(&draft));
        UnitFamilyBuilder::new(draft, id)
    }
}
