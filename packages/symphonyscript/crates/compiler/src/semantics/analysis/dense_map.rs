use super::dense_id::DenseId;

pub(crate) struct DenseMap<K, V> {
    slots: Vec<Option<(K, V)>>,
}

impl<K, V> Default for DenseMap<K, V> {
    fn default() -> Self {
        Self { slots: Vec::new() }
    }
}

impl<K: DenseId, V> DenseMap<K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        self.slots
            .get(key.index())?
            .as_ref()
            .map(|(_, value)| value)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let index = key.index();
        if self.slots.len() <= index {
            self.slots.resize_with(index + 1, || None);
        }
        let slot = self.slots.get_mut(index)?;
        slot.replace((key, value)).map(|(_, old)| old)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.slots.get_mut(key.index())?.take().map(|(_, old)| old)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.slots.iter().flatten().map(|(key, value)| (key, value))
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(key, _)| key)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, value)| value)
    }
}

impl<'a, K: DenseId, V> IntoIterator for &'a DenseMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Box<dyn Iterator<Item = (&'a K, &'a V)> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}

impl<K: DenseId, V> Extend<(K, V)> for DenseMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, items: I) {
        for (key, value) in items {
            self.insert(key, value);
        }
    }
}
