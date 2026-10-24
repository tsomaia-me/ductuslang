use std::ops::Range;

use crate::syntax::TokenView;
use crate::syntax::nfc::nfc;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Name<'p>(TokenView<'p>);

impl<'p> Name<'p> {
    pub(crate) fn new(token: TokenView<'p>) -> Self {
        Self(token)
    }

    pub fn text(&self) -> String {
        nfc(self.0.text()).into_owned()
    }

    pub fn token(&self) -> &TokenView<'p> {
        &self.0
    }

    pub fn range(&self) -> Range<u32> {
        self.0.range()
    }
}
