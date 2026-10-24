use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use super::fast_hasher::FastHasher;

pub(crate) type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<FastHasher>>;
