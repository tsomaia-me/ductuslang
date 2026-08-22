//! The `Provenance` enum: the closed set of binding-provenance markers.

/// How a binding's cell argument was materialized (IR_GRAMMAR §3's
/// `provenance`, §15.4.6).
///
/// ```text
/// provenance ::= 'bound' | 'constant_wrap' | 'bridge'
/// ```
///
/// The `@`-suffixed marker on an effect's `params` binding or a wire's `attrs`
/// binding records how the argument reached the binding under the uniform
/// cell-argument rule (§13.17.3). It is optional METADATA on a binding, never a
/// distinct binding case (033-252) — which is why it lives behind an `Option`
/// on [`Binding`](crate::graph::Binding) rather than splitting the type.
///
/// A vocabulary scalar, so `PartialEq` is not the equality R17 withholds — the
/// `Prim` reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// `bound` — the argument is a cell the caller bound directly.
    Bound,
    /// `constant_wrap` — a static value wrapped as a degenerate constant cell.
    ConstantWrap,
    /// `bridge` — the argument arrives through a bridge cell.
    Bridge,
}

impl Provenance {
    /// The one spelling of this marker in `.duir` text (§3's `provenance`).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Provenance::Bound => "bound",
            Provenance::ConstantWrap => "constant_wrap",
            Provenance::Bridge => "bridge",
        }
    }

    /// The marker `word` names, or `None` when it names none.
    ///
    /// The soft-keyword classifier for the word after a binding's `@` (R46).
    #[must_use]
    pub fn from_word(word: &str) -> Option<Provenance> {
        let provenance = match word {
            "bound" => Provenance::Bound,
            "constant_wrap" => Provenance::ConstantWrap,
            "bridge" => Provenance::Bridge,
            _ => return None,
        };

        Some(provenance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const ALL: [Provenance; 3] = [
        Provenance::Bound,
        Provenance::ConstantWrap,
        Provenance::Bridge,
    ];

    #[test]
    fn every_marker_spells_its_grammar_word() {
        assert_eq!(Provenance::Bound.as_str(), "bound");
        assert_eq!(Provenance::ConstantWrap.as_str(), "constant_wrap");
        assert_eq!(Provenance::Bridge.as_str(), "bridge");
    }

    #[test]
    fn from_word_inverts_as_str_for_every_marker() {
        let mut words = HashSet::new();

        for provenance in ALL {
            assert_eq!(Provenance::from_word(provenance.as_str()), Some(provenance));
            assert!(
                words.insert(provenance.as_str()),
                "{} twice",
                provenance.as_str()
            );
        }

        assert_eq!(words.len(), 3);
    }

    #[test]
    fn from_word_rejects_what_is_not_one_of_the_three() {
        for word in [
            "constant-wrap",
            "constant",
            "wrap",
            "binding",
            "Bound",
            "bound ",
            "",
            "banana",
        ] {
            assert_eq!(Provenance::from_word(word), None, "{word}");
        }
    }
}
