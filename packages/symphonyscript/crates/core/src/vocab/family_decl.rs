use alloc::string::String;
use alloc::vec::Vec;

use super::unit_decl::UnitDecl;

#[derive(Clone, PartialEq, Debug)]
pub struct FamilyDecl {
    pub name: String,
    pub units: Vec<UnitDecl>,
    pub doc: Option<String>,
}

impl FamilyDecl {
    pub fn base(&self) -> Option<&UnitDecl> {
        self.units.first()
    }
}
