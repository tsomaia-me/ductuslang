use std::collections::BTreeMap;

use super::decl::Decl;

#[derive(Clone, Debug, Default)]
pub(crate) struct Namespace {
    names: BTreeMap<String, Decl>,
}

impl Namespace {
    pub fn get(&self, name: &str) -> Option<Decl> {
        self.names.get(name).copied()
    }

    pub fn insert(&mut self, name: String, decl: Decl) -> Option<Decl> {
        self.names.insert(name, decl)
    }
}
