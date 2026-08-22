//! The `ValueLiteral` enum: a compile-time literal in a graph entry.

/// The one float spelling that is not word-shaped (033-248), and so arrives as
/// a `Float` token rather than an `Ident`.
const NEGATIVE_INFINITY: &str = "-inf";

/// A compile-time literal — IR_GRAMMAR §3's `value` (§15.4.6).
///
/// ```text
/// value ::= INT | FLOAT | 'true' | 'false' | STRING
///         | 'nan' | 'inf' | '-inf'
/// ```
///
/// It stands where a static binding stands: a cell's `init`, a fold cell's
/// `else`, and the right-hand side of a `binding` that is not a `PATH`.
///
/// # Why the numbers are text
///
/// A typed value is NOT converted here (R57). The grammar admits `u128`/`i128`
/// literals no `i64` holds, and the position's type is not known at parse
/// time — the loader owns it, under the cell's declared `type_tag`. So `Int`
/// and `Float` carry the digits and the loader reads them. STRUCTURAL numbers
/// go the other way: a `depth`, a `capacity`, a `history`, an ordinal and a
/// lookback `k` are all `u64` at parse time, because their type follows from
/// the position alone.
///
/// The three float words are TYPE-GATED (033-248): the grammar admits `nan`,
/// `inf` and `-inf` anywhere a value stands, and the load-time type check is
/// what rejects them outside a float-typed position. Not a parser matter.
///
/// # Canonical spelling (033-257)
///
/// Since a typed value is not converted, canonical uniqueness cannot ride on
/// the conversion the way it does for structural numbers — `init 7` and
/// `init 007` would otherwise be two stable spellings of one value. So the
/// canonicalization is TEXTUAL, and it happens HERE, in
/// [`int`](ValueLiteral::int) and [`float`](ValueLiteral::float):
///
/// - an `Int` strips leading zeros down to at least one digit, and `-0` becomes
///   `0` — `007` → `7`, `-007` → `-7`, `-0` → `0`;
/// - a `Float` KEEPS its sign (`-0.0` is a value of its own, not `0.0`), strips
///   integer-part leading zeros down to at least one digit, and strips fraction
///   trailing zeros down to at least one digit — `1.50` → `1.5`, `0.10` → `0.1`,
///   `00.0` → `0.0`, `1.000` → `1.0`.
///
/// Every one of those rewrites names the same real number, so the result is
/// **value-safe under every type the position may later take**: no width, no
/// signedness and no rounding can read the canonical spelling differently from
/// the one it replaced. That is the whole reason the normalization is allowed
/// to run before the type is known.
///
/// # Producer contract
///
/// The variants are public — plain data, no constructor, legality is the
/// loader's (F4) — so a hand-built `ValueLiteral::Int("007".to_string())` is
/// writable and is NOT canonical. Producers go through
/// [`int`](ValueLiteral::int) and [`float`](ValueLiteral::float); the parser
/// does, and the emitter writes the stored text as it stands.
///
/// No `PartialEq` (R17): two literals are the same when they emit the same
/// canonical text.
#[derive(Debug, Clone)]
pub enum ValueLiteral {
    /// An `INT` lexeme's digits, sign included, canonically spelled.
    Int(String),
    /// A `FLOAT` lexeme's digits, sign included, canonically spelled.
    Float(String),
    /// `true`.
    True,
    /// `false`.
    False,
    /// A `STRING`'s content, ESCAPES ALREADY PROCESSED — the value, not the
    /// spelling. Canonical re-escaping happens on the way out (033-249).
    Str(String),
    /// `nan`.
    Nan,
    /// `inf`.
    Inf,
    /// `-inf`.
    NegInf,
}

impl ValueLiteral {
    /// The value an `INT` lexeme names, canonically spelled (033-257).
    ///
    /// Canonicalization is in place — both rules REMOVE bytes — so the text
    /// moves in from the token and out to the AST without a second allocation
    /// (R18). Text that is not `INT`-shaped is left exactly as given: there is
    /// nothing to canonicalize in a spelling the grammar does not admit, and
    /// inventing one would hide the producer's bug.
    #[must_use]
    pub fn int(mut text: String) -> ValueLiteral {
        canonicalize_int(&mut text);

        ValueLiteral::Int(text)
    }

    /// The value a `FLOAT` lexeme names, canonically spelled (033-257).
    ///
    /// The lexeme covers one non-numeric spelling — `-inf`, the one float word
    /// that is not word-shaped (033-248) — which lands on its own variant here
    /// rather than as digits. Bare `nan` and `inf` ARE word-shaped, so they
    /// arrive as words and never reach this function.
    #[must_use]
    pub fn float(mut text: String) -> ValueLiteral {
        if text == NEGATIVE_INFINITY {
            return ValueLiteral::NegInf;
        }

        canonicalize_float(&mut text);

        ValueLiteral::Float(text)
    }
}

// `INT` canonicalization (033-257): leading zeros go, at least one digit stays,
// and `-0` loses its sign because an integer has no negative zero. Both rules
// remove bytes, so the string is drained rather than rebuilt.
fn canonicalize_int(text: &mut String) {
    let negative = text.starts_with('-');
    let digits_at = usize::from(negative);

    if !is_digit_run(&text[digits_at..]) {
        return;
    }

    let significant = leading_zeros(&text[digits_at..]);
    text.drain(digits_at..digits_at + significant);

    if negative && text == "-0" {
        text.remove(0);
    }
}

// `FLOAT` canonicalization (033-257): integer-part leading zeros and
// fraction-part trailing zeros go, at least one digit staying on each side, and
// the SIGN is preserved — `-0.0` is not `0.0`.
//
// The fraction is drained BEFORE the integer part, since draining the integer
// part would shift every index the fraction's was measured with.
fn canonicalize_float(text: &mut String) {
    let negative = text.starts_with('-');
    let body_at = usize::from(negative);

    let Some(dot) = text[body_at..].find('.').map(|index| body_at + index) else {
        return;
    };

    let integer = &text[body_at..dot];
    let fraction = &text[dot + 1..];

    if !is_digit_run(integer) || !is_digit_run(fraction) {
        return;
    }

    let keep = fraction.len() - trailing_zeros(fraction);
    let significant = leading_zeros(integer);
    let end = text.len();

    text.drain(dot + 1 + keep..end);
    text.drain(body_at..body_at + significant);
}

// A nonempty run of ASCII digits — what both lexemes are made of. Anything else
// reaching a canonicalizer is a hand-built spelling, left untouched.
fn is_digit_run(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

// How many leading zeros a digit run may lose: all of them, except that one
// digit always stays, so `000` keeps its last `0`.
fn leading_zeros(digits: &str) -> usize {
    digits
        .find(|character: char| character != '0')
        .unwrap_or(digits.len() - 1)
}

// How many trailing zeros a digit run may lose, under the same one-digit floor.
fn trailing_zeros(digits: &str) -> usize {
    match digits.rfind(|character: char| character != '0') {
        Some(last) => digits.len() - last - 1,
        None => digits.len() - 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `ValueLiteral` has no equality (R17), so a test reads the payload out.
    fn int(text: &str) -> String {
        match ValueLiteral::int(text.to_string()) {
            ValueLiteral::Int(canonical) => canonical,
            other => panic!("an INT lexeme is an `Int`, got {other:?}"),
        }
    }

    fn float(text: &str) -> String {
        match ValueLiteral::float(text.to_string()) {
            ValueLiteral::Float(canonical) => canonical,
            other => panic!("a FLOAT lexeme is a `Float`, got {other:?}"),
        }
    }

    #[test]
    fn an_int_strips_leading_zeros_to_one_digit() {
        assert_eq!(int("007"), "7");
        assert_eq!(int("0"), "0");
        assert_eq!(int("00"), "0");
        assert_eq!(int("000"), "0");
        assert_eq!(int("0000000001"), "1");
        assert_eq!(int("10"), "10");
        assert_eq!(int("100"), "100");
        assert_eq!(int("0100"), "100");
    }

    // An integer has no negative zero, so `-0` is `0`; every other negative
    // keeps its sign.
    #[test]
    fn an_int_canonicalizes_negative_zero_and_keeps_every_other_sign() {
        assert_eq!(int("-0"), "0");
        assert_eq!(int("-00"), "0");
        assert_eq!(int("-000"), "0");
        assert_eq!(int("-007"), "-7");
        assert_eq!(int("-5"), "-5");
        assert_eq!(int("-0005"), "-5");
    }

    // A spelling wider than any Rust integer is still just digits: the
    // canonicalization is textual, so it neither converts nor loses anything.
    #[test]
    fn an_int_canonicalizes_a_spelling_no_machine_integer_holds() {
        assert_eq!(
            int("0170141183460469231731687303715884105727"),
            "170141183460469231731687303715884105727"
        );
        assert_eq!(int("-0000000000000000000000000000000000000001"), "-1");
    }

    #[test]
    fn a_float_strips_integer_leading_and_fraction_trailing_zeros() {
        assert_eq!(float("1.50"), "1.5");
        assert_eq!(float("0.10"), "0.1");
        assert_eq!(float("00.0"), "0.0");
        assert_eq!(float("1.000"), "1.0");
        assert_eq!(float("007.500"), "7.5");
        assert_eq!(float("0.0"), "0.0");
        assert_eq!(float("000.000"), "0.0");
        assert_eq!(float("10.01"), "10.01");
        assert_eq!(float("0.00001"), "0.00001");
    }

    // The sign is PRESERVED on a float, always: IEEE-754 negative zero is a
    // value of its own, and nothing here may fold it into positive zero.
    #[test]
    fn a_float_preserves_its_sign_including_negative_zero() {
        assert_eq!(float("-0.0"), "-0.0");
        assert_eq!(float("-00.000"), "-0.0");
        assert_eq!(float("-1.50"), "-1.5");
        assert_eq!(float("-007.500"), "-7.5");
    }

    // Canonicalization is IDEMPOTENT: the canonical spelling of a canonical
    // spelling is itself, which is what "exactly one spelling per value" means
    // operationally.
    #[test]
    fn canonicalization_is_idempotent() {
        for text in [
            "7",
            "0",
            "-7",
            "100",
            "170141183460469231731687303715884105727",
        ] {
            assert_eq!(int(&int(text)), int(text), "{text}");
        }

        for text in ["1.5", "0.1", "0.0", "-0.0", "1.0", "10.01"] {
            assert_eq!(float(&float(text)), float(text), "{text}");
        }
    }

    // `-inf` reaches the float constructor as a lexeme, since the leading `-`
    // makes it the one float spelling that is not word-shaped. It is its own
    // variant, not digits.
    #[test]
    fn negative_infinity_lands_on_its_own_variant() {
        assert!(matches!(
            ValueLiteral::float(NEGATIVE_INFINITY.to_string()),
            ValueLiteral::NegInf
        ));
    }

    // A spelling the grammar does not admit is left alone rather than guessed
    // at: `+7` cannot come from the lexer, and inventing a canonical form for
    // it would hide the producer bug that made it.
    #[test]
    fn a_spelling_that_is_not_the_lexeme_passes_through_untouched() {
        assert_eq!(int("+7"), "+7");
        assert_eq!(int(""), "");
        assert_eq!(int("-"), "-");
        assert_eq!(int("7x"), "7x");
        assert_eq!(float("1"), "1");
        assert_eq!(float("1."), "1.");
        assert_eq!(float(".5"), ".5");
        assert_eq!(float("1.5e3"), "1.5e3");
    }

    // The payload-free variants carry no text and need no canonicalization —
    // their spellings ARE canonical as written (033-257).
    #[test]
    fn the_word_shaped_literals_are_payload_free() {
        assert!(matches!(ValueLiteral::True, ValueLiteral::True));
        assert!(matches!(ValueLiteral::Nan, ValueLiteral::Nan));
        assert!(matches!(
            ValueLiteral::Str("raw\ncontent".to_string()).clone(),
            ValueLiteral::Str(_)
        ));
    }
}
