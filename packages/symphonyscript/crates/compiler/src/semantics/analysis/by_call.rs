use super::ids::CallId;

pub(crate) struct ByCall<V> {
    slots: Vec<Option<V>>,
}

impl<V> Default for ByCall<V> {
    fn default() -> Self {
        ByCall { slots: Vec::new() }
    }
}

impl<V> ByCall<V> {
    pub fn get(&self, id: &CallId) -> Option<&V> {
        self.slots.get(position(*id))?.as_ref()
    }

    pub fn contains_key(&self, id: &CallId) -> bool {
        self.get(id).is_some()
    }

    pub fn insert(&mut self, id: CallId, value: V) -> Option<V> {
        let at = position(id);
        if self.slots.len() <= at {
            self.slots.resize_with(at.saturating_add(1), || None);
        }
        self.slots.get_mut(at)?.replace(value)
    }

    #[cfg(test)]
    pub fn iter(&self) -> impl Iterator<Item = (CallId, &V)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(at, slot)| Some((CallId(super::index::index(at)), slot.as_ref()?)))
    }
}

fn position(id: CallId) -> usize {
    usize::try_from(id.0).unwrap_or(usize::MAX)
}
