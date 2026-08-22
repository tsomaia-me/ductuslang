//! [`is_name_shaped()`] — the `NAME` charset test, in one place.

/// Whether `text` is NAME-shaped (IR_GRAMMAR §2): ASCII letters, digits and
/// `_`, never digit-initial, never empty.
///
/// The same shape the lexer's `ident` recognizer matches, written here as a
/// PREDICATE rather than a recognizer because its callers hold a finished
/// string rather than a cursor into a source. Both of them are contract
/// checks, not parsing: the path concern asserts a segment name with it, and
/// the emitter asserts every name position it writes.
///
/// It sits at `text/` level for the reason `keywords` does — it is the text
/// form's law, shared by more than one of its halves and owned by none of
/// them.
#[must_use]
pub(crate) fn is_name_shaped(text: &str) -> bool {
    let mut characters = text.chars();

    let Some(first) = characters.next() else {
        return false;
    };

    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_name_charset() {
        for text in [
            "a",
            "A",
            "_",
            "_0",
            "app",
            "App",
            "pool_index",
            "bb0",
            "x1_2",
        ] {
            assert!(is_name_shaped(text), "{text}");
        }
    }

    // Digit-initial, empty, and every character outside the charset — which is
    // what makes a key quote (R41) and what trips the emitter's producer
    // contract.
    #[test]
    fn rejects_everything_else() {
        for text in [
            "", "0", "42", "0a", "a b", "a.b", "a-b", "a:0", "a'b", "café", "日", "a\n",
        ] {
            assert!(!is_name_shaped(text), "{text}");
        }
    }
}
