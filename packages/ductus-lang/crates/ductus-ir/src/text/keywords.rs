//! The `.duir` keyword vocabulary — one spelling per keyword, in one place
//! (R58).
//!
//! Every word-shaped token lexes as `Ident` (R46), so a keyword is not a token
//! class: it is TEXT compared at the position that expects it. Both the parser
//! and the emitter import from here, so each spelling is written down exactly
//! once — the parser cannot come to accept a word the emitter does not write,
//! and a typo fails every round-trip test rather than half of them.
//!
//! A const vocabulary is one concern and one file (R58): S1's one-public-entity
//! law governs types and file-level functions, not consts.
//!
//! The sixteen primitive words (`i32`, `str`, …) are deliberately NOT here.
//! They are [`Prim`](crate::types::Prim)'s own closed vocabulary, spelled by
//! `Prim::as_str` and classified by `Prim::from_word`; splitting them across
//! two tables is what would let the two disagree.
//!
//! The same rule keeps four more CLOSED SETS out of this file, each owning its
//! spellings the way `Prim` does: the five cell kinds
//! ([`CellKind`](crate::graph::CellKind)), the three membership drivers
//! ([`MemberDriver`](crate::graph::MemberDriver)), the three provenance markers
//! ([`Provenance`](crate::graph::Provenance)) and the two stream policies
//! ([`StreamPolicy`](crate::graph::StreamPolicy)). A closed set is a TYPE here,
//! not a list of words, and its `as_str`/`from_word` pair is the single place
//! its spellings live. What remains below is the open vocabulary: section
//! keywords, entry heads, and clause names, which belong to no type.
//!
//! One spelling appears in both places, and it is not a collision: `gate` heads
//! a `gate` entry and names a `gate PATH` clause (here), and also names a
//! stream policy (`StreamPolicy`). Position decides — the soft-keyword law
//! (R46) — so the disjointness tests below check this table against ITSELF and
//! against `Prim`, which are the two vocabularies one position ever classifies
//! against at once.
//!
//! The spellings are IR_GRAMMAR §2 and §3 (§15.4.6).

/// `module` — the module header.
pub(crate) const MODULE: &str = "module";

/// `types` — the type-table section.
pub(crate) const TYPES: &str = "types";

/// `graph` — the graph section.
pub(crate) const GRAPH: &str = "graph";

/// `behaviors` — the behavior-table section.
pub(crate) const BEHAVIORS: &str = "behaviors";

/// `record` — a named type's field layout.
pub(crate) const RECORD: &str = "record";

/// `enum` — a named type's variant layout.
pub(crate) const ENUM: &str = "enum";

/// `tuple` — a named type's positional layout.
pub(crate) const TUPLE: &str = "tuple";

/// `size` — a `type_def`'s stated size in bytes.
pub(crate) const SIZE: &str = "size";

/// `align` — a `type_def`'s stated alignment in bytes.
pub(crate) const ALIGN: &str = "align";

/// `pool_index` — the pool-indexed-aggregate type tag.
pub(crate) const POOL_INDEX: &str = "pool_index";

/// `yielded` — the membership-descriptor type tag.
pub(crate) const YIELDED: &str = "yielded";

/// `handle` — the handle-reference type tag.
pub(crate) const HANDLE: &str = "handle";

/// `portal` — the portal-reference type tag.
pub(crate) const PORTAL: &str = "portal";

/// `closure` — the closure-environment type tag.
pub(crate) const CLOSURE: &str = "closure";

/// `scope` — a graph-section scope.
pub(crate) const SCOPE: &str = "scope";

/// `exposes` — a scope's exposed topology.
pub(crate) const EXPOSES: &str = "exposes";

/// `effects` — a scope's effect set.
pub(crate) const EFFECTS: &str = "effects";

/// `reset_on_reopen` — the keyword's three grammar positions (033-215): a
/// scope's `reopen_set` payload, a cell's bare flag, a stream's bare flag.
pub(crate) const RESET_ON_REOPEN: &str = "reset_on_reopen";

/// `reset_on_reload` — a stream's bare reload flag.
pub(crate) const RESET_ON_RELOAD: &str = "reset_on_reload";

/// `uses` — a cell's behavior handle.
pub(crate) const USES: &str = "uses";

/// `inputs` — a cell's or a gate's input cells.
pub(crate) const INPUTS: &str = "inputs";

/// `depth` — a recurrent cell's SELF-history depth, a deferred cell's latency.
pub(crate) const DEPTH: &str = "depth";

/// `init` — a cell's initial value, and on a recurrent the uniform spelling of
/// `fallbacks` (033-268).
pub(crate) const INIT: &str = "init";

/// `fallbacks` — a recurrent cell's per-depth history seeds (033-267): the
/// general form `init` is the uniform spelling of.
pub(crate) const FALLBACKS: &str = "fallbacks";

/// `gate` — the gate entry head, and the `gate PATH` clause on a cell, a wire
/// and an effect.
pub(crate) const GATE: &str = "gate";

/// `combiner` — a fold cell's `by:` combiner handle.
pub(crate) const COMBINER: &str = "combiner";

/// `else` — a fold cell's empty-membership value.
pub(crate) const ELSE: &str = "else";

/// `members` — a fold or membership-descriptor cell's member set.
pub(crate) const MEMBERS: &str = "members";

/// `pred` — a gate's predicate behavior handle.
pub(crate) const PRED: &str = "pred";

/// `guards` — what a gate guards.
pub(crate) const GUARDS: &str = "guards";

/// `gate_parent` — the enclosing gate, when gates nest.
pub(crate) const GATE_PARENT: &str = "gate_parent";

/// `wire` — the wire entry head.
pub(crate) const WIRE: &str = "wire";

/// `from` — a wire's source.
pub(crate) const FROM: &str = "from";

/// `to` — a wire's destination.
pub(crate) const TO: &str = "to";

/// `null` — a wire destination spelled as none.
pub(crate) const NULL: &str = "null";

/// `type` — a wire's declared type.
pub(crate) const TYPE: &str = "type";

/// `attrs` — a wire's attribute bindings.
pub(crate) const ATTRS: &str = "attrs";

/// `effect` — the effect entry head.
pub(crate) const EFFECT: &str = "effect";

/// `reconciler` — an effect's reconciler name.
pub(crate) const RECONCILER: &str = "reconciler";

/// `params` — an effect's argument bindings.
pub(crate) const PARAMS: &str = "params";

/// `desired` — an effect's desired-state cells.
pub(crate) const DESIRED: &str = "desired";

/// `observed` — an effect's observed-state cells.
pub(crate) const OBSERVED: &str = "observed";

/// `stream` — the stream entry head.
pub(crate) const STREAM: &str = "stream";

/// `policy` — a stream's overflow policy.
pub(crate) const POLICY: &str = "policy";

/// `capacity` — a stream's buffer capacity.
pub(crate) const CAPACITY: &str = "capacity";

/// `source_deps` — a stream's source dependencies.
pub(crate) const SOURCE_DEPS: &str = "source_deps";

/// `observes` — a stream's synthesized observation cells.
pub(crate) const OBSERVES: &str = "observes";

/// `history` — a recurrent stream's output history size.
pub(crate) const HISTORY: &str = "history";

/// `lookback` — a recurrent stream's or a recurrent CELL's input lookback map
/// (033-266): one clause name, one production, two positions.
pub(crate) const LOOKBACK: &str = "lookback";

/// `true` — a boolean value literal.
pub(crate) const TRUE: &str = "true";

/// `false` — the other boolean value literal.
pub(crate) const FALSE: &str = "false";

/// `nan` — the not-a-number float spelling (type-gated, 033-248).
pub(crate) const NAN: &str = "nan";

/// `inf` — the infinity float spelling (type-gated, 033-248). Its negative
/// counterpart `-inf` is not word-shaped and arrives as a `Float` lexeme.
pub(crate) const INF: &str = "inf";

/// The five RESERVED spellings (002-39, 033-259) — the language-wide
/// reservation, not a `.duir` rule of its own.
///
/// These five differ in kind from everything above. Every other spelling in
/// this file is SOFT (R46): `gate` is a keyword where a gate is expected and an
/// ordinary name everywhere else, so a cell may legally be called `gate`. These
/// five are HARD: no declaration of any kind may bear one as its name, so they
/// are never a `NAME` and the parser rejects one at every name position under
/// [`DiagClass::ReservedWord`](crate::diag::DiagClass::ReservedWord).
///
/// That is what lets the two positions admitting both a `PATH` and a literal —
/// a wire's destination, a binding's right-hand side — read the word as the
/// literal with NO lookahead: a path cannot begin with one of these, so there
/// is nothing to disambiguate (033-258).
///
/// The array is built from the consts above rather than respelling them, so the
/// five spellings still live in exactly one place each.
pub(crate) const RESERVED: [&str; 5] = [NULL, TRUE, FALSE, NAN, INF];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Prim;
    use std::collections::HashSet;

    // The vocabulary as a list, for the two properties that a per-const
    // assertion could never state. Written out rather than generated: a keyword
    // added without being listed here is invisible to both checks.
    const ALL: [&str; 53] = [
        MODULE,
        TYPES,
        GRAPH,
        BEHAVIORS,
        RECORD,
        ENUM,
        TUPLE,
        SIZE,
        ALIGN,
        POOL_INDEX,
        YIELDED,
        HANDLE,
        PORTAL,
        CLOSURE,
        SCOPE,
        EXPOSES,
        EFFECTS,
        RESET_ON_REOPEN,
        RESET_ON_RELOAD,
        USES,
        INPUTS,
        DEPTH,
        INIT,
        FALLBACKS,
        GATE,
        COMBINER,
        ELSE,
        MEMBERS,
        PRED,
        GUARDS,
        GATE_PARENT,
        WIRE,
        FROM,
        TO,
        NULL,
        TYPE,
        ATTRS,
        EFFECT,
        RECONCILER,
        PARAMS,
        DESIRED,
        OBSERVED,
        STREAM,
        POLICY,
        CAPACITY,
        SOURCE_DEPS,
        OBSERVES,
        HISTORY,
        LOOKBACK,
        TRUE,
        FALSE,
        NAN,
        INF,
    ];

    // The reservation is a SUBSET of the vocabulary, never a sixth spelling
    // invented beside it: a reserved word the parser rejects but no position
    // ever writes would be a word nothing can produce.
    #[test]
    fn every_reserved_spelling_is_one_of_the_vocabularys_own() {
        for word in RESERVED {
            assert!(ALL.contains(&word), "{word}");
        }

        let distinct: HashSet<&str> = RESERVED.into_iter().collect();

        assert_eq!(distinct.len(), RESERVED.len());
    }

    // Two consts sharing a spelling would silently make one position accept the
    // other's word — the copy-paste failure this table exists to prevent.
    #[test]
    fn no_two_keywords_share_a_spelling() {
        let distinct: HashSet<&str> = ALL.into_iter().collect();

        assert_eq!(distinct.len(), ALL.len());
    }

    // A type position classifies a word against BOTH tables — the wrappers here
    // and `Prim` — so a word in both would have two readings and the dispatch
    // order would silently decide which one wins.
    #[test]
    fn no_keyword_is_also_a_primitive_word() {
        for keyword in ALL {
            assert_eq!(Prim::from_word(keyword), None, "{keyword}");
        }
    }

    // Every spelling is NAME-shaped, which is what makes it reachable at all:
    // the lexer emits one `Ident` per word (R46), so a keyword spelled with a
    // hyphen or a space could never be matched by any position. R39 respelled
    // the two member drivers for exactly this reason.
    #[test]
    fn every_keyword_is_one_word_in_the_name_charset() {
        for keyword in ALL {
            let mut characters = keyword.chars();
            let first = characters.next().expect("a keyword is not empty");

            assert!(first.is_ascii_alphabetic() || first == '_', "{keyword}");
            assert!(
                characters.all(|character| character.is_ascii_alphanumeric() || character == '_'),
                "{keyword}"
            );
        }
    }
}
