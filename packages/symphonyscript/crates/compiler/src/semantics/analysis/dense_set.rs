use std::marker::PhantomData;

use super::dense_id::DenseId;

pub(crate) struct DenseSet<K> {
    members: Vec<bool>,
    key: PhantomData<K>,
}

impl<K> Default for DenseSet<K> {
    fn default() -> Self {
        Self {
            members: Vec::new(),
            key: PhantomData,
        }
    }
}

impl<K: DenseId> DenseSet<K> {
    pub fn contains(&self, key: &K) -> bool {
        self.members.get(key.index()).copied().unwrap_or(false)
    }

    pub fn insert(&mut self, key: K) -> bool {
        let index = key.index();
        if self.members.len() <= index {
            self.members.resize(index + 1, false);
        }
        match self.members.get_mut(index) {
            Some(member) => !std::mem::replace(member, true),
            None => false,
        }
    }
}
