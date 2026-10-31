use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::eval_ty::EvalTy;

const NO_ENTRY: u32 = u32::MAX;

#[derive(Default)]
pub(crate) struct FrameValues {
    heads: Vec<u32>,
    entries: Vec<FrameEntry>,
}

struct FrameEntry {
    slot: u32,
    ty: EvalTy,
    value: CVal,
    next: u32,
}

impl FrameValues {
    pub fn get(&self, slot: u32, ty: EvalTy) -> Option<&CVal> {
        let mut at = *self.heads.get(index(slot))?;
        while let Some(entry) = self.entries.get(index(at)) {
            if entry.ty == ty {
                return Some(&entry.value);
            }
            at = entry.next;
        }
        None
    }

    pub fn insert(&mut self, slot: u32, ty: EvalTy, value: CVal) {
        let position = index(slot);
        if self.heads.len() <= position {
            self.heads.resize(position.saturating_add(1), NO_ENTRY);
        }
        let Some(head) = self.heads.get(position).copied() else {
            return;
        };
        let mut at = head;
        while let Some(entry) = self.entries.get_mut(index(at)) {
            if entry.ty == ty {
                entry.value = value;
                return;
            }
            at = entry.next;
        }
        let Ok(added) = u32::try_from(self.entries.len()) else {
            return;
        };
        self.entries.push(FrameEntry {
            slot,
            ty,
            value,
            next: head,
        });
        if let Some(head) = self.heads.get_mut(position) {
            *head = added;
        }
    }

    pub fn clear(&mut self) {
        for entry in self.entries.drain(..) {
            if let Some(head) = self.heads.get_mut(index(entry.slot)) {
                *head = NO_ENTRY;
            }
        }
    }
}

fn index(slot: u32) -> usize {
    usize::try_from(slot).unwrap_or(usize::MAX)
}
