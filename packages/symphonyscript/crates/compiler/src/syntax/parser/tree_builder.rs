use super::checkpoint::Checkpoint;
use crate::syntax::syntax_kind::SyntaxKind;
use crate::syntax::tree_columns::TreeColumns;

struct Marker {
    kind: SyntaxKind,
    token: u32,
    node: u32,
    serial: u32,
}

pub(super) struct TreeBuilder {
    tree: TreeColumns,
    markers: Vec<Marker>,
    serial: u32,
    tokens: u32,
    renamed: Vec<(u32, SyntaxKind)>,
}

impl TreeBuilder {
    pub(super) fn new(bytes: usize) -> Self {
        Self {
            tree: TreeColumns::reserved(bytes / 4),
            markers: Vec::new(),
            serial: 0,
            tokens: 0,
            renamed: Vec::new(),
        }
    }

    pub(super) fn token(&mut self, kind: SyntaxKind, lexed: SyntaxKind) {
        if kind != lexed {
            self.renamed.push((self.tokens, kind));
        }
        self.tokens += 1;
    }

    pub(super) fn start_node(&mut self, kind: SyntaxKind) {
        self.open(kind, self.tokens, self.nodes());
    }

    pub(super) fn open_kind(&self) -> Option<SyntaxKind> {
        self.markers.last().map(|marker| marker.kind)
    }

    pub(super) fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            token: self.tokens,
            node: self.nodes(),
            serial: self.markers.last().map_or(0, |marker| marker.serial),
        }
    }

    pub(super) fn start_node_at(&mut self, checkpoint: Checkpoint, kind: SyntaxKind) {
        let open = checkpoint.serial == 0
            || self
                .markers
                .binary_search_by_key(&checkpoint.serial, |marker| marker.serial)
                .is_ok();
        let (token, node) = match self.markers.last() {
            _ if !open => (self.tokens, self.nodes()),
            Some(floor) if checkpoint.token < floor.token || checkpoint.node < floor.node => {
                (floor.token, floor.node)
            }
            _ => (checkpoint.token, checkpoint.node),
        };
        self.open(kind, token, node);
    }

    pub(super) fn finish_node(&mut self) {
        let Some(marker) = self.markers.pop() else {
            return;
        };
        crate::semantics::counted_walk::CountedWalk::ParseNode.step();
        self.tree
            .push(marker.kind, (marker.token, self.tokens), marker.node);
    }

    pub(super) fn finish(mut self) -> (TreeColumns, Vec<(u32, SyntaxKind)>) {
        while !self.markers.is_empty() {
            self.finish_node();
        }
        let whole = self.tree.len().checked_sub(1).is_some_and(|root| {
            let root = root as u32;
            self.tree.first_desc(root) == 0 && self.tree.tokens(root) == (0, self.tokens)
        });
        if !whole {
            crate::semantics::counted_walk::CountedWalk::ParseNode.step();
            self.tree.push(SyntaxKind::File, (0, self.tokens), 0);
        }
        (self.tree, self.renamed)
    }

    fn nodes(&self) -> u32 {
        u32::try_from(self.tree.len()).unwrap_or(u32::MAX)
    }

    fn open(&mut self, kind: SyntaxKind, token: u32, node: u32) {
        self.serial = self.serial.saturating_add(1);
        self.markers.push(Marker {
            kind,
            token,
            node,
            serial: self.serial,
        });
    }
}
