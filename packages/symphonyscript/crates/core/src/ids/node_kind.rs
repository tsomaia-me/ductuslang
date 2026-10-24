use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use super::vocabulary_token::VocabularyToken;

#[derive(Clone, Copy, Debug)]
pub struct NodeKind {
    vocabulary: VocabularyToken,
    index: u32,
}

impl NodeKind {
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

impl PartialEq for NodeKind {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for NodeKind {}

impl PartialOrd for NodeKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl Hash for NodeKind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}
