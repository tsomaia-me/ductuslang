use super::parsed::Parsed;

pub(crate) struct Descendants<'p> {
    parsed: &'p Parsed,
    pending: Vec<u32>,
}

impl<'p> Descendants<'p> {
    pub(crate) fn of(parsed: &'p Parsed, node: u32) -> Self {
        Self {
            parsed,
            pending: vec![node],
        }
    }
}

impl Iterator for Descendants<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let node = self.pending.pop()?;
        let tree = self.parsed.tree();
        let mut child = tree.last_child(node);
        while let Some(found) = child {
            self.pending.push(found);
            child = tree.previous_sibling(node, found);
        }
        Some(node)
    }
}
