use std::hash::Hash;

pub(crate) trait Interned {
    type Parts<'a>: Copy + Hash
    where
        Self: 'a;

    fn holds(&self, parts: Self::Parts<'_>) -> bool;

    fn owned(parts: Self::Parts<'_>) -> Self;
}
