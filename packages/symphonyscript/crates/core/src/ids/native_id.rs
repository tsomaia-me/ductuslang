use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use super::vocabulary_token::VocabularyToken;

#[derive(Clone, Copy, Debug)]
pub struct NativeId {
    vocabulary: VocabularyToken,
    index: u32,
}

impl NativeId {
    pub(crate) fn new(vocabulary: VocabularyToken, index: u32) -> Self {
        Self { vocabulary, index }
    }

    pub(crate) fn vocabulary(self) -> VocabularyToken {
        self.vocabulary
    }

    pub fn index(self) -> usize {
        self.index as usize
    }
}

impl PartialEq for NativeId {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for NativeId {}

impl PartialOrd for NativeId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NativeId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl Hash for NativeId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}
