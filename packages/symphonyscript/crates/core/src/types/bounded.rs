use alloc::vec::Vec;

#[derive(Clone, PartialEq, Debug)]
pub struct Bounded<T, const N: usize> {
    items: Vec<T>,
}

impl<T, const N: usize> Bounded<T, N> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn from_items(items: impl IntoIterator<Item = T>) -> Option<Self> {
        let items: Vec<T> = items.into_iter().collect();
        (items.len() <= N).then_some(Self { items })
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn into_items(self) -> Vec<T> {
        self.items
    }
}

impl<T, const N: usize> Default for Bounded<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
