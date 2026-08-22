//! [`is_reserved_word()`] — the language-wide reservation test, in one place.

use crate::text::keywords::RESERVED;

/// Whether `text` is one of the five spellings reserved language-wide
/// (002-39, 033-259): `null`, `true`, `false`, `nan`, `inf`.
///
/// A reserved spelling is never a `NAME`. No declaration of any kind may bear
/// one — a module name, a type id, a field name, a variant name, a binding name
/// and a path's name segment all reject it — so no cell, wire, stream or member
/// name can ever collide with a literal spelling.
///
/// It sits at `text/` level for the reason [`is_name_shaped`] does: it is the
/// text form's law, checked by more than one of its halves and owned by none of
/// them. The parser rejects at the name positions; the path concern asserts the
/// same claim as a producer invariant, and the emitter reads it to decide
/// whether a key may render bare — a key spelled `null` must stay quoted or the
/// next read would take it for the keyword (033-243).
///
/// [`is_name_shaped`]: crate::text::is_name_shaped
#[must_use]
pub(crate) fn is_reserved_word(text: &str) -> bool {
    RESERVED.contains(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_five_reserved_spellings() {
        for text in ["null", "true", "false", "nan", "inf"] {
            assert!(is_reserved_word(text), "{text}");
        }
    }

    // Reservation is EXACT-MATCH, and deliberately so: `nullable` and `infer`
    // are ordinary names, `-inf` is a FLOAT lexeme rather than a word, and the
    // soft keywords (`gate`, `init`, `uses`) stay legal names under R46.
    #[test]
    fn rejects_everything_else() {
        for text in [
            "", "nullable", "Null", "NULL", "infer", "-inf", "nan_", "truely", "gate", "init",
            "uses", "module",
        ] {
            assert!(!is_reserved_word(text), "{text}");
        }
    }
}
