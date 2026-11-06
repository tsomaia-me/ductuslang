use super::core::{Core, Stage};
use crate::semantics::Internal;
use crate::semantics::ir::flat::{ABSENT, ItemRec, NONE};

impl Core<'_> {
    pub fn when_blocks(&self, item: u32) -> Vec<u32> {
        let Ok(ItemRec::When(rec)) = self.ir.item(item).copied() else {
            return Vec::new();
        };
        let mut blocks: Vec<u32> = self
            .ir
            .arms
            .get(rec.arms.0 as usize..rec.arms.1 as usize)
            .unwrap_or_default()
            .iter()
            .map(|arm| arm.1)
            .collect();
        blocks.push(rec.otherwise);
        blocks.retain(|block| *block != NONE && *block != ABSENT);
        blocks
    }

    pub fn when_guards(&self, item: u32) -> Vec<u32> {
        let Ok(ItemRec::When(rec)) = self.ir.item(item).copied() else {
            return Vec::new();
        };
        self.ir
            .arms
            .get(rec.arms.0 as usize..rec.arms.1 as usize)
            .unwrap_or_default()
            .iter()
            .map(|arm| arm.0)
            .filter(|guard| *guard != NONE)
            .collect()
    }

    pub fn when_items(&self, item: u32) -> Vec<u32> {
        let mut items = Vec::new();
        for block in self.when_blocks(item) {
            if let Some(block) = self.ir.blocks.get(block as usize) {
                items.extend(
                    self.ir
                        .block_items
                        .get(block.items.0 as usize..block.items.1 as usize)
                        .unwrap_or_default()
                        .iter()
                        .filter(|item| !self.ir.dropped(**item)),
                );
                items.extend(
                    self.ir
                        .members
                        .get(block.members.0 as usize..block.members.1 as usize)
                        .unwrap_or_default()
                        .iter()
                        .filter(|item| **item != NONE),
                );
            }
        }
        items
    }

    pub fn check_guards(&mut self) -> Stage<()> {
        for at in 0..self.guards.len() {
            let (guard, env) = *self.guards.get(at).ok_or(Internal::missed("a guard"))?;
            let saved = self.enter_env(env);
            self.feed_env = env;
            let checked = self.bind_gate(guard);
            self.feed_env = NONE;
            self.leave_env(saved);
            if let Err(declined) = checked? {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    pub fn when_members(&self, item: u32) -> Result<Vec<u32>, Internal> {
        let mut found = Vec::new();
        let mut items = self.when_items(item);
        items.reverse();
        while let Some(child) = items.pop() {
            match *self.ir.item(child)? {
                ItemRec::Line(members) => {
                    let mut inner: Vec<u32> = self
                        .ir
                        .members
                        .get(members.0 as usize..members.1 as usize)
                        .unwrap_or_default()
                        .to_vec();
                    inner.reverse();
                    items.extend(inner);
                }
                ItemRec::When(_) => {
                    let mut inner = self.when_items(child);
                    inner.reverse();
                    items.extend(inner);
                }
                ItemRec::Placement(_)
                | ItemRec::Wire(_)
                | ItemRec::Bundle(_)
                | ItemRec::For(_)
                | ItemRec::Paren(_) => found.push(child),
            }
        }
        Ok(found)
    }
}
