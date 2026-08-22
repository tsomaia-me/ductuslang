//! The `CellKind` enum: the closed set of cell-kind tags.

/// The kind that leads a `cell` line (IR_GRAMMAR §3's `cell_kind`, §15.4.6).
///
/// ```text
/// cell_kind ::= 'input' | 'derived' | 'recurrent' | 'deferred' | 'fold'
/// ```
///
/// The set is CLOSED at exactly these five (033-80): `fold` is a cell KIND
/// rather than a sixth graph primitive, and no `yielded` tag exists — a
/// standalone `yielded` group lowers to a membership-descriptor `derived` cell
/// carrying a `members` set, and a folded group is absorbed into its fold
/// cell's `members` set.
///
/// Which CLAUSES a kind admits is not enforced here and not enforced by the
/// parser: `depth` on an `input` cell parses (F4, R21). The loader owns
/// kind-versus-clause legality, where the graph's semantics live.
///
/// A vocabulary scalar, so `PartialEq` is not the equality R17 withholds — the
/// same reading `Prim` carries: there is no structure to compare and no
/// identity question to beg, and [`from_word`](CellKind::from_word)'s dispatch
/// and tests are what need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellKind {
    /// `input` — a stored attr or signal, written from outside the graph.
    Input,
    /// `derived` — a pure function of its `inputs`, recomputed on demand.
    Derived,
    /// `recurrent` — a derived cell that reads its own history, `depth` deep.
    Recurrent,
    /// `deferred` — a derived cell published `depth` commits after it is
    /// evaluated (033-245).
    Deferred,
    /// `fold` — the lowering of the language fold form: a `combiner`, an
    /// `else` value, and a `members` set (033-81).
    Fold,
}

impl CellKind {
    /// The one spelling of this kind in `.duir` text (§3's `cell_kind`).
    ///
    /// The emitter writes this and nothing else, which is what makes the
    /// spelling canonical rather than conventional.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            CellKind::Input => "input",
            CellKind::Derived => "derived",
            CellKind::Recurrent => "recurrent",
            CellKind::Deferred => "deferred",
            CellKind::Fold => "fold",
        }
    }

    /// The kind `word` names, or `None` when it names none.
    ///
    /// The soft-keyword classifier for the entry-head position (R46): the lexer
    /// knows no keywords, so `derived` arrives as an ordinary `Ident` exactly as
    /// a cell named `derived` does. A word becomes a kind only where an entry
    /// head is what the grammar expects.
    #[must_use]
    pub fn from_word(word: &str) -> Option<CellKind> {
        let kind = match word {
            "input" => CellKind::Input,
            "derived" => CellKind::Derived,
            "recurrent" => CellKind::Recurrent,
            "deferred" => CellKind::Deferred,
            "fold" => CellKind::Fold,
            _ => return None,
        };

        Some(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // The closed set, written out. A sixth variant fails to compile here, and
    // one added WITH a spelling fails the count assertion below until listed.
    const ALL: [CellKind; 5] = [
        CellKind::Input,
        CellKind::Derived,
        CellKind::Recurrent,
        CellKind::Deferred,
        CellKind::Fold,
    ];

    #[test]
    fn every_kind_spells_its_grammar_word() {
        assert_eq!(CellKind::Input.as_str(), "input");
        assert_eq!(CellKind::Derived.as_str(), "derived");
        assert_eq!(CellKind::Recurrent.as_str(), "recurrent");
        assert_eq!(CellKind::Deferred.as_str(), "deferred");
        assert_eq!(CellKind::Fold.as_str(), "fold");
    }

    #[test]
    fn from_word_inverts_as_str_for_every_kind() {
        let mut words = HashSet::new();

        for kind in ALL {
            assert_eq!(CellKind::from_word(kind.as_str()), Some(kind));
            assert!(words.insert(kind.as_str()), "{} twice", kind.as_str());
        }

        assert_eq!(words.len(), 5);
    }

    // The set is closed, so a plausible neighbour is not a kind: `yielded` is
    // a type tag, `gate` an entry of its own, `attr` the surface language's
    // word for what lowers to `input`.
    #[test]
    fn from_word_rejects_what_is_not_one_of_the_five() {
        for word in [
            "yielded", "gate", "wire", "effect", "stream", "attr", "Input", "inputs", "input ", "",
            "folded", "banana",
        ] {
            assert_eq!(CellKind::from_word(word), None, "{word}");
        }
    }
}
