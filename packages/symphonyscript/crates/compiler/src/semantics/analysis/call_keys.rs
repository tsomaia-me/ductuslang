use std::hash::BuildHasher;

use super::call_key::CallKey;
use super::call_parts::CallParts;
use super::fast_hash::FastMap;
use super::ids::CallId;
use crate::semantics::counted_walk::CountedWalk;

#[derive(Default)]
pub(crate) struct CallKeys {
    keys: Vec<CallKey>,
    first: FastMap<u64, CallId>,
    next: Vec<Option<CallId>>,
}

impl CallKeys {
    pub fn intern(&mut self, parts: CallParts<'_>) -> CallId {
        CountedWalk::CallKeysIntern.step();
        let hash = self.first.hasher().hash_one(parts);
        let mut probe = self.first.get(&hash).copied();
        while let Some(id) = probe {
            CountedWalk::CallKeysProbe.step();
            let index = usize::try_from(id.0).unwrap_or(usize::MAX);
            if self.keys.get(index).is_some_and(|key| key.parts() == parts) {
                return id;
            }
            probe = self.next.get(index).copied().flatten();
        }
        let id = CallId(u32::try_from(self.keys.len()).unwrap_or(u32::MAX));
        self.keys.push(parts.owned());
        self.next.push(self.first.insert(hash, id));
        id
    }

    pub fn key(&self, id: CallId) -> Option<&CallKey> {
        self.keys.get(usize::try_from(id.0).ok()?)
    }
}
