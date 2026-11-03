#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SymbolTable {
    first: u32,
    slots: Vec<u32>,
    hashes: Vec<u64>,
    starts: Vec<u32>,
    text: String,
}

impl SymbolTable {
    pub fn starting_at(first: u32) -> Self {
        Self {
            first,
            ..Self::default()
        }
    }

    pub fn first(&self) -> u32 {
        self.first
    }

    pub fn end(&self) -> u32 {
        self.first
            .saturating_add(u32::try_from(self.hashes.len()).unwrap_or(u32::MAX))
    }

    pub fn text(&self, sym: u32) -> Option<&str> {
        let local = usize::try_from(sym.checked_sub(self.first)?).ok()?;
        let start = usize::try_from(*self.starts.get(local)?).ok()?;
        let end = match self.starts.get(local + 1) {
            Some(end) => usize::try_from(*end).ok()?,
            None => self.text.len(),
        };
        self.text.get(start..end)
    }

    pub fn find(&self, text: &str, hash: u64) -> Option<u32> {
        let mask = self.slots.len().checked_sub(1)?;
        let mut slot = usize::try_from(hash).unwrap_or(usize::MAX) & mask;
        loop {
            let held = *self.slots.get(slot)?;
            let local = held.checked_sub(1)?;
            let sym = self.first.saturating_add(local);
            if self.hashes.get(local as usize) == Some(&hash) && self.text(sym) == Some(text) {
                return Some(sym);
            }
            slot = (slot + 1) & mask;
        }
    }

    pub fn insert(&mut self, text: &str, hash: u64) -> u32 {
        if (self.hashes.len() + 1) * 2 > self.slots.len() {
            self.grow();
        }
        let local = u32::try_from(self.hashes.len()).unwrap_or(u32::MAX);
        self.hashes.push(hash);
        self.starts
            .push(u32::try_from(self.text.len()).unwrap_or(u32::MAX));
        self.text.push_str(text);
        self.place(local, hash);
        self.first.saturating_add(local)
    }

    fn place(&mut self, local: u32, hash: u64) {
        let Some(mask) = self.slots.len().checked_sub(1) else {
            return;
        };
        let mut slot = usize::try_from(hash).unwrap_or(usize::MAX) & mask;
        while let Some(held) = self.slots.get_mut(slot) {
            if *held == 0 {
                *held = local + 1;
                return;
            }
            slot = (slot + 1) & mask;
        }
    }

    fn grow(&mut self) {
        let size = (self.slots.len() * 2).max(16);
        self.slots = vec![0; size];
        for local in 0..self.hashes.len() {
            if let Some(hash) = self.hashes.get(local).copied() {
                self.place(u32::try_from(local).unwrap_or(u32::MAX), hash);
            }
        }
    }
}
