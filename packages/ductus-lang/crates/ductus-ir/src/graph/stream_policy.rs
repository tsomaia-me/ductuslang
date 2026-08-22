//! The `StreamPolicy` enum: the closed set of stream overflow policies.

/// A stream's overflow policy (IR_GRAMMAR §3's `stream`, §15.4.6).
///
/// ```text
/// stream ::= 'stream' PATH ':' type_tag 'policy' ('ring' | 'gate') 'capacity' INT …
/// ```
///
/// The alternatives are spelled inline in the production rather than under a
/// nonterminal of their own; they are a closed two-word vocabulary all the
/// same, so they get the same shape every other closed vocabulary here has.
///
/// **`gate` is also a keyword elsewhere**, and that is not a collision: it
/// heads a `gate` entry and names a `gate PATH` clause, while here it names a
/// policy. Position decides, which is exactly what the soft-keyword law is for
/// (R46) — the lexer knows no keywords, so `gate` is an ordinary word until a
/// position classifies it.
///
/// A vocabulary scalar, so `PartialEq` is not the equality R17 withholds — the
/// `Prim` reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamPolicy {
    /// `ring` — the oldest pending element is dropped when the buffer is full.
    Ring,
    /// `gate` — the writer is refused when the buffer is full.
    Gate,
}

impl StreamPolicy {
    /// The one spelling of this policy in `.duir` text (§3's `stream`).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            StreamPolicy::Ring => "ring",
            StreamPolicy::Gate => "gate",
        }
    }

    /// The policy `word` names, or `None` when it names none.
    ///
    /// The soft-keyword classifier for the word after `policy` (R46).
    #[must_use]
    pub fn from_word(word: &str) -> Option<StreamPolicy> {
        let policy = match word {
            "ring" => StreamPolicy::Ring,
            "gate" => StreamPolicy::Gate,
            _ => return None,
        };

        Some(policy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [StreamPolicy; 2] = [StreamPolicy::Ring, StreamPolicy::Gate];

    #[test]
    fn every_policy_spells_its_grammar_word() {
        assert_eq!(StreamPolicy::Ring.as_str(), "ring");
        assert_eq!(StreamPolicy::Gate.as_str(), "gate");
    }

    #[test]
    fn from_word_inverts_as_str_for_every_policy() {
        for policy in ALL {
            assert_eq!(StreamPolicy::from_word(policy.as_str()), Some(policy));
        }
    }

    #[test]
    fn from_word_rejects_what_is_not_one_of_the_two() {
        for word in ["buffer", "drop", "Ring", "ring ", "", "gated", "banana"] {
            assert_eq!(StreamPolicy::from_word(word), None, "{word}");
        }
    }
}
