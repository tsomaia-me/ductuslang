use std::fmt;
use std::ops::Range;
use std::sync::OnceLock;

use super::ast::{AstNode, File};
use super::node_view::NodeView;
use super::syntax_kind::SyntaxKind;
use super::token_columns::TokenColumns;
use super::tree_columns::TreeColumns;
use crate::diagnostics::Diagnostic;

pub(crate) const NO_NODE: u32 = u32::MAX;

#[derive(Clone)]
pub struct Parsed {
    text: String,
    tokens: TokenColumns,
    tree: TreeColumns,
    pub diagnostics: Vec<Diagnostic>,
    pub(crate) detached: Vec<Range<u32>>,
    parents: OnceLock<Vec<u32>>,
    token_parents: OnceLock<Vec<u32>>,
}

impl Parsed {
    pub(crate) fn new(
        text: String,
        tokens: TokenColumns,
        tree: TreeColumns,
        diagnostics: Vec<Diagnostic>,
        detached: Vec<Range<u32>>,
    ) -> Self {
        Self {
            text,
            tokens,
            tree,
            diagnostics,
            detached,
            parents: OnceLock::new(),
            token_parents: OnceLock::new(),
        }
    }

    pub fn root(&self) -> NodeView<'_> {
        let root = self.tree.len().saturating_sub(1);
        NodeView::new(self, u32::try_from(root).unwrap_or(0))
    }

    pub fn file(&self) -> Option<File<'_>> {
        File::cast(self.root())
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn tokens(&self) -> &TokenColumns {
        &self.tokens
    }

    pub(crate) fn tree(&self) -> &TreeColumns {
        &self.tree
    }

    pub(crate) fn token_start(&self, token: u32) -> u32 {
        self.tokens
            .start(token as usize)
            .unwrap_or_else(|| u32::try_from(self.text.len()).unwrap_or(u32::MAX))
    }

    pub(crate) fn token_count(&self) -> u32 {
        u32::try_from(self.tokens.len()).unwrap_or(u32::MAX)
    }

    pub(crate) fn parent_of(&self, node: u32) -> u32 {
        self.parents
            .get_or_init(|| self.parent_column())
            .get(node as usize)
            .copied()
            .unwrap_or(NO_NODE)
    }

    pub(crate) fn parent_of_token(&self, token: u32) -> u32 {
        self.token_parents
            .get_or_init(|| self.token_parent_column())
            .get(token as usize)
            .copied()
            .unwrap_or(NO_NODE)
    }

    fn parent_column(&self) -> Vec<u32> {
        let mut parents = vec![NO_NODE; self.tree.len()];
        for node in 0..self.tree.len() as u32 {
            let mut child = self.tree.last_child(node);
            while let Some(found) = child {
                if let Some(slot) = parents.get_mut(found as usize) {
                    *slot = node;
                }
                child = self.tree.previous_sibling(node, found);
            }
        }
        parents
    }

    fn token_parent_column(&self) -> Vec<u32> {
        let mut parents = vec![NO_NODE; self.tokens.len()];
        let mut assign = |tokens: Range<u32>, node: u32| {
            for token in tokens {
                if let Some(slot) = parents.get_mut(token as usize) {
                    *slot = node;
                }
            }
        };
        for node in 0..self.tree.len() as u32 {
            let (first, mut end) = self.tree.tokens(node);
            let mut child = self.tree.last_child(node);
            while let Some(found) = child {
                let (child_first, child_end) = self.tree.tokens(found);
                assign(child_end..end, node);
                end = child_first;
                child = self.tree.previous_sibling(node, found);
            }
            assign(first..end, node);
        }
        parents
    }
}

impl PartialEq for Parsed {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.tokens == other.tokens
            && self.tree == other.tree
            && self.diagnostics == other.diagnostics
            && self.detached == other.detached
    }
}

impl Eq for Parsed {}

impl fmt::Debug for Parsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = self.tree.kinds.last().copied().unwrap_or(SyntaxKind::File);
        f.debug_struct("Parsed")
            .field("kind", &kind)
            .field("text_len", &self.text.len())
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}
