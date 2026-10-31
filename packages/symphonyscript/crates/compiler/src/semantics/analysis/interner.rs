use std::hash::BuildHasher;

use super::fast_hash::FastMap;
use super::interned::Interned;
use crate::semantics::counted_walk::CountedWalk;

pub(crate) struct Interner<K> {
    keys: Vec<K>,
    first: FastMap<u64, u32>,
    next: Vec<Option<u32>>,
}

impl<K> Default for Interner<K> {
    fn default() -> Self {
        Interner {
            keys: Vec::new(),
            first: FastMap::default(),
            next: Vec::new(),
        }
    }
}

impl<K: Interned> Interner<K> {
    pub fn intern(&mut self, parts: K::Parts<'_>) -> u32 {
        CountedWalk::InternerIntern.step();
        let hash = self.first.hasher().hash_one(parts);
        let mut probe = self.first.get(&hash).copied();
        while let Some(id) = probe {
            CountedWalk::InternerProbe.step();
            let index = usize::try_from(id).unwrap_or(usize::MAX);
            if self.keys.get(index).is_some_and(|key| key.holds(parts)) {
                return id;
            }
            probe = self.next.get(index).copied().flatten();
        }
        let id = u32::try_from(self.keys.len()).unwrap_or(u32::MAX);
        self.keys.push(K::owned(parts));
        self.next.push(self.first.insert(hash, id));
        id
    }

    pub fn key(&self, id: u32) -> Option<&K> {
        self.keys.get(usize::try_from(id).ok()?)
    }
}
