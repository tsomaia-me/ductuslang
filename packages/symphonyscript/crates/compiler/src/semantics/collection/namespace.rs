use std::collections::{BTreeMap, BTreeSet};

use super::decl::Decl;

#[derive(Clone, Debug, Default)]
pub(crate) struct Namespace {
    names: BTreeMap<String, Decl>,
    duplicated: BTreeSet<String>,
}

impl Namespace {
    pub fn get(&self, name: &str) -> Option<Decl> {
        self.names.get(name).copied()
    }

    pub fn insert(&mut self, name: String, decl: Decl) -> Option<Decl> {
        self.names.insert(name, decl)
    }

    pub fn duplicate(&mut self, name: &str) {
        self.names.remove(name);
        self.duplicated.insert(name.to_string());
    }

    pub fn is_duplicated(&self, name: &str) -> bool {
        self.duplicated.contains(name)
    }
}
