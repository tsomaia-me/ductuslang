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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Prim;
    use std::collections::HashSet;

    // The vocabulary as a list, for the two properties that a per-const
    // assertion could never state. Written out rather than generated: a keyword
    // added without being listed here is invisible to both checks.
    const ALL: [&str; 14] = [
        MODULE, TYPES, GRAPH, BEHAVIORS, RECORD, ENUM, TUPLE, SIZE, ALIGN, POOL_INDEX, YIELDED,
        HANDLE, PORTAL, CLOSURE,
    ];

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
}
