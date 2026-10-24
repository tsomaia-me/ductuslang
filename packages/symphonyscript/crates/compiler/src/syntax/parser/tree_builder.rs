use rowan::{GreenNode, GreenToken, Language, NodeOrToken};

use super::checkpoint::Checkpoint;
use crate::syntax::symphony_language::SymphonyLanguage;
use crate::syntax::syntax_kind::SyntaxKind;

pub(super) struct TreeBuilder {
    parents: Vec<(SyntaxKind, usize)>,
    children: Vec<NodeOrToken<GreenNode, GreenToken>>,
}

impl TreeBuilder {
    pub(super) fn new() -> Self {
        Self {
            parents: Vec::new(),
            children: Vec::new(),
        }
    }

    pub(super) fn token(&mut self, kind: SyntaxKind, text: &str) {
        let token = GreenToken::new(SymphonyLanguage::kind_to_raw(kind), text);
        self.children.push(NodeOrToken::Token(token));
    }

    pub(super) fn start_node(&mut self, kind: SyntaxKind) {
        self.parents.push((kind, self.children.len()));
    }

    pub(super) fn checkpoint(&self) -> Checkpoint {
        Checkpoint(self.children.len())
    }

    pub(super) fn start_node_at(&mut self, checkpoint: Checkpoint, kind: SyntaxKind) {
        let floor = self.parents.last().map_or(0, |(_, first)| *first);
        let first = checkpoint.0.clamp(floor, self.children.len());
        self.parents.push((kind, first));
    }

    pub(super) fn finish_node(&mut self) {
        let Some((kind, first)) = self.parents.pop() else {
            return;
        };
        let first = first.min(self.children.len());
        let node = GreenNode::new(
            SymphonyLanguage::kind_to_raw(kind),
            self.children.drain(first..),
        );
        self.children.push(NodeOrToken::Node(node));
    }

    pub(super) fn finish(mut self) -> GreenNode {
        while !self.parents.is_empty() {
            self.finish_node();
        }
        match self.children.pop() {
            Some(NodeOrToken::Node(node)) if self.children.is_empty() => node,
            other => {
                let mut children: Vec<_> = self.children.drain(..).collect();
                children.extend(other);
                GreenNode::new(SymphonyLanguage::kind_to_raw(SyntaxKind::File), children)
            }
        }
    }
}
