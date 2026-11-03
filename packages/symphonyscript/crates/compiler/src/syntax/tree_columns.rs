use super::syntax_kind::SyntaxKind;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct TreeColumns {
    pub(crate) kinds: Vec<SyntaxKind>,
    pub(crate) first_tokens: Vec<u32>,
    pub(crate) end_tokens: Vec<u32>,
    pub(crate) first_descs: Vec<u32>,
}

impl TreeColumns {
    pub(crate) fn reserved(nodes: usize) -> Self {
        Self {
            kinds: Vec::with_capacity(nodes),
            first_tokens: Vec::with_capacity(nodes),
            end_tokens: Vec::with_capacity(nodes),
            first_descs: Vec::with_capacity(nodes),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.kinds.len()
    }

    pub(crate) fn push(&mut self, kind: SyntaxKind, tokens: (u32, u32), first_desc: u32) {
        self.kinds.push(kind);
        self.first_tokens.push(tokens.0);
        self.end_tokens.push(tokens.1);
        self.first_descs.push(first_desc);
    }

    pub(crate) fn first_desc(&self, node: u32) -> u32 {
        self.first_descs.get(node as usize).copied().unwrap_or(node)
    }

    pub(crate) fn tokens(&self, node: u32) -> (u32, u32) {
        let index = node as usize;
        (
            self.first_tokens.get(index).copied().unwrap_or(0),
            self.end_tokens.get(index).copied().unwrap_or(0),
        )
    }

    pub(crate) fn last_child_before(&self, node: u32, below: u32) -> Option<u32> {
        let floor = self.first_desc(node);
        (below > floor).then(|| below - 1)
    }

    pub(crate) fn previous_sibling(&self, parent: u32, child: u32) -> Option<u32> {
        self.last_child_before(parent, self.first_desc(child))
    }

    pub(crate) fn last_child(&self, node: u32) -> Option<u32> {
        self.last_child_before(node, node)
    }
}
