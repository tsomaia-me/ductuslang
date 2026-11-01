use std::hash::Hash;

use super::dense_id::DenseId;
use super::fast_hash::FastMap;

pub(crate) struct DensePairs<K, S, V> {
    first: Vec<Option<((K, S), V)>>,
    rest: FastMap<(K, S), V>,
}

impl<K, S, V> Default for DensePairs<K, S, V> {
    fn default() -> Self {
        Self {
            first: Vec::new(),
            rest: FastMap::default(),
        }
    }
}

impl<K: DenseId + Eq + Hash, S: Copy + Eq + Hash, V> DensePairs<K, S, V> {
    pub fn get(&self, key: &(K, S)) -> Option<&V> {
        match self.first.get(key.0.index()) {
            Some(Some((held, value))) if held.1 == key.1 => Some(value),
            Some(Some(_)) => self.rest.get(key),
            Some(None) | None => None,
        }
    }

    pub fn contains_key(&self, key: &(K, S)) -> bool {
        self.get(key).is_some()
    }

    pub fn insert(&mut self, key: (K, S), value: V) -> Option<V> {
        let index = key.0.index();
        if self.first.len() <= index {
            self.first.resize_with(index + 1, || None);
        }
        let slot = self.first.get_mut(index)?;
        match slot {
            Some((held, old)) if held.1 == key.1 => Some(std::mem::replace(old, value)),
            Some(_) => self.rest.insert(key, value),
            None => {
                *slot = Some((key, value));
                None
            }
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.first.iter().flatten().count() + self.rest.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&(K, S), &V)> {
        self.first
            .iter()
            .flatten()
            .map(|(key, value)| (key, value))
            .chain(self.rest.iter())
    }

    pub fn keys(&self) -> impl Iterator<Item = &(K, S)> {
        self.iter().map(|(key, _)| key)
    }
}

impl<'a, K: DenseId + Eq + Hash, S: Copy + Eq + Hash, V> IntoIterator for &'a DensePairs<K, S, V> {
    type Item = (&'a (K, S), &'a V);
    type IntoIter = Box<dyn Iterator<Item = (&'a (K, S), &'a V)> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}
