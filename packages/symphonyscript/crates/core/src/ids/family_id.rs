use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use super::vocabulary_token::VocabularyToken;

#[derive(Clone, Copy, Debug)]
pub struct FamilyId {
    vocabulary: VocabularyToken,
    index: u32,
}

impl FamilyId {
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

impl PartialEq for FamilyId {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for FamilyId {}

impl PartialOrd for FamilyId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FamilyId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl Hash for FamilyId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}
