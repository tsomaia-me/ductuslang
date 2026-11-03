const INLINE: usize = 8;

pub(crate) struct ChildList {
    inline: [u32; INLINE],
    spilled: Vec<u32>,
    len: usize,
}

impl ChildList {
    pub(crate) fn new() -> Self {
        Self {
            inline: [0; INLINE],
            spilled: Vec::new(),
            len: 0,
        }
    }

    pub(crate) fn push(&mut self, child: u32) {
        if self.len < INLINE {
            if let Some(slot) = self.inline.get_mut(self.len) {
                *slot = child;
            }
        } else {
            if self.spilled.is_empty() {
                self.spilled.extend_from_slice(&self.inline);
            }
            self.spilled.push(child);
        }
        self.len += 1;
    }

    pub(crate) fn pop(&mut self) -> Option<u32> {
        self.len = self.len.checked_sub(1)?;
        if self.spilled.is_empty() {
            self.inline.get(self.len).copied()
        } else {
            self.spilled.pop()
        }
    }
}
