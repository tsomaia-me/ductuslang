use alloc::string::String;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct VocabularyStamp {
    pub name: String,
    pub hash: u64,
}
