use std::ops::Deref;

use crate::semantics::counted_walk::CountedWalk;

pub(crate) const ROOT_CHAIN: u32 = 0;

#[derive(Default, Debug)]
pub(crate) struct CallChain {
    calls: Vec<u32>,
    ids: Vec<u32>,
}

impl Deref for CallChain {
    type Target = [u32];

    fn deref(&self) -> &[u32] {
        &self.calls
    }
}

impl Clone for CallChain {
    fn clone(&self) -> Self {
        CountedWalk::CoreCallChain.step_by(self.calls.len());
        CallChain {
            calls: self.calls.clone(),
            ids: self.ids.clone(),
        }
    }
}

impl CallChain {
    pub fn id(&self) -> u32 {
        self.ids.last().copied().unwrap_or(ROOT_CHAIN)
    }

    pub fn caller_id(&self) -> u32 {
        self.ids
            .len()
            .checked_sub(2)
            .and_then(|at| self.ids.get(at))
            .copied()
            .unwrap_or(ROOT_CHAIN)
    }

    pub fn pop(&mut self) -> Option<u32> {
        self.ids.pop();
        self.calls.pop()
    }

    pub fn truncate(&mut self, len: usize) {
        self.calls.truncate(len);
        self.ids.truncate(len);
    }

    pub fn id_at(&self, len: usize) -> u32 {
        len.checked_sub(1)
            .and_then(|at| self.ids.get(at))
            .copied()
            .unwrap_or(ROOT_CHAIN)
    }
}

#[derive(Default)]
pub(crate) struct ChainArena {
    links: Vec<(u32, u32)>,
    ids: super::column::PairColumn<u32>,
}

impl ChainArena {
    fn intern(&mut self, parent: u32, call: u32) -> u32 {
        if let Some(id) = self.ids.get(&(parent, call)) {
            return *id;
        }
        self.links.push((parent, call));
        let id = u32::try_from(self.links.len()).unwrap_or(u32::MAX);
        self.ids.insert((parent, call), id);
        id
    }

    pub fn path(&mut self, items: impl Iterator<Item = u32>) -> u32 {
        items.fold(ROOT_CHAIN, |parent, item| self.intern(parent, item))
    }

    pub fn push(&mut self, chain: &mut CallChain, call: u32) {
        CountedWalk::CoreCallChain.step();
        let id = self.intern(chain.id(), call);
        chain.calls.push(call);
        chain.ids.push(id);
    }

    pub fn chain_at(&self, id: u32) -> CallChain {
        let mut ids = Vec::new();
        let mut at = id;
        while at != ROOT_CHAIN {
            ids.push(at);
            at = self
                .links
                .get(at as usize - 1)
                .map_or(ROOT_CHAIN, |(parent, _)| *parent);
        }
        ids.reverse();
        CountedWalk::CoreCallChain.step_by(ids.len());
        let calls = ids
            .iter()
            .filter_map(|id| self.links.get(*id as usize - 1).map(|(_, call)| *call))
            .collect();
        CallChain { calls, ids }
    }

    pub fn restore(&self, chain: &mut CallChain, len: usize, id: u32) {
        if chain.len() >= len && chain.id_at(len) == id {
            chain.truncate(len);
        } else {
            *chain = self.chain_at(id);
        }
    }
}
