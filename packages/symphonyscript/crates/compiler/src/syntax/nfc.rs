use std::borrow::Cow;

use unicode_normalization::UnicodeNormalization;

pub(crate) fn nfc(text: &str) -> Cow<'_, str> {
    if text.is_ascii() {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(text.nfc().collect())
    }
}
