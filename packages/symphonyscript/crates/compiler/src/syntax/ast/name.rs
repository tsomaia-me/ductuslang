use rowan::TextRange;

use crate::syntax::SyntaxToken;
use crate::syntax::nfc::nfc;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Name(SyntaxToken);

impl Name {
    pub(crate) fn new(token: SyntaxToken) -> Self {
        Self(token)
    }

    pub fn text(&self) -> String {
        nfc(self.0.text()).into_owned()
    }

    pub fn token(&self) -> &SyntaxToken {
        &self.0
    }

    pub fn range(&self) -> TextRange {
        self.0.text_range()
    }
}
