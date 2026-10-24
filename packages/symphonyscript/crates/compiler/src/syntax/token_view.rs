use std::fmt;
use std::marker::PhantomData;
use std::ops::Range;

use super::navigation;
use super::node_view::NodeView;
use super::parsed::Parsed;
use super::syntax_kind::SyntaxKind;
use super::syntax_token::SyntaxToken;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TokenView<'p> {
    raw: SyntaxToken,
    parsed: PhantomData<&'p Parsed>,
}

impl<'p> TokenView<'p> {
    pub(crate) fn new(raw: SyntaxToken) -> Self {
        Self {
            raw,
            parsed: PhantomData,
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.raw.kind()
    }

    pub fn text(&self) -> &str {
        self.raw.text()
    }

    pub fn range(&self) -> Range<u32> {
        let range = self.raw.text_range();
        u32::from(range.start())..u32::from(range.end())
    }

    pub fn parent(&self) -> Option<NodeView<'p>> {
        self.raw.parent().map(NodeView::new)
    }

    pub fn next_token(&self) -> Option<TokenView<'p>> {
        navigation::next_token(&self.raw).map(TokenView::new)
    }

    pub fn prev_token(&self) -> Option<TokenView<'p>> {
        navigation::prev_token(&self.raw).map(TokenView::new)
    }
}

impl fmt::Debug for TokenView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@{:?} {:?}", self.kind(), self.range(), self.text())
    }
}

impl fmt::Display for TokenView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text())
    }
}
