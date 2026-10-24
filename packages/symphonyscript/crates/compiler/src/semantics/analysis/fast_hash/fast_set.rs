use std::collections::HashSet;
use std::hash::BuildHasherDefault;

use super::fast_hasher::FastHasher;

pub(crate) type FastSet<K> = HashSet<K, BuildHasherDefault<FastHasher>>;
