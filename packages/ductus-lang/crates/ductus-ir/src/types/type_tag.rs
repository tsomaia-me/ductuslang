//! The `TypeTag` enum: the unified type-tag nonterminal.

use crate::types::Prim;

/// One type tag — the unified `type_tag` nonterminal used throughout both the
/// module and the behavior grammars (IR_GRAMMAR §2, §15.4.6).
///
/// The nine alternatives mirror the production one for one, in its order:
///
/// ```text
/// type_tag ::= PRIM
///            | '%' NAME
///            | 'pool_index' '<' '%' NAME '>'
///            | 'yielded' '<' type_tag '>'
///            | 'handle' '<' type_tag '>'
///            | 'portal' '<' type_tag '>'
///            | '(' type_tag (',' type_tag)* ')'
///            | '[' type_tag ';' INT ']'
///            | 'closure' '<' '(' (type_tag (',' type_tag)*)? ')' '->' type_tag '>'
/// ```
///
/// Recursion is by `Box` and `Vec`, so a tag is an owned tree with no lifetime
/// and no arena — the text of a `.duir` file is small, and a type tag is read
/// once per load.
///
/// No `PartialEq` (R17). Two tags are the same when they emit the same
/// canonical text, and `text::emitter::emit` is that definition — a derived
/// equality would invite `==` to become the type-identity rule, which is the
/// loader's decision to make under the module's type table, not a structural
/// one.
#[derive(Debug, Clone)]
pub enum TypeTag {
    /// A primitive: `i32`, `str`, … (§2's `PRIM`).
    Prim(Prim),
    /// `%NAME` — a named record/enum/tuple layout from the module's `types`
    /// section.
    ///
    /// The payload is the NAME alone; the `%` is the sigil, not part of the id.
    TypeId(String),
    /// `pool_index<%NAME>` — a pool-indexed aggregate (§14.3.3).
    ///
    /// An aggregate-VALUED cell is typed this way and never inline `%NAME`. The
    /// payload is the referenced type's NAME, the inner `%` being a sigil like
    /// the one above.
    PoolIndex(String),
    /// `yielded<T>` — the ABI-only membership-descriptor type (§15.4.1,
    /// 033-79).
    ///
    /// It types synthesized membership-descriptor cells and `yielded T`
    /// parameters, and is never a user-facing value type. That restriction is
    /// semantic, so the grammar admits the tag wherever a tag is admitted and
    /// the loader is what rejects a misuse (F4).
    Yielded(Box<TypeTag>),
    /// `handle<T>` — a handle reference (§15.4.1's inline
    /// `(slot_path, generation)` representation).
    Handle(Box<TypeTag>),
    /// `portal<T>` — a portal reference, the other half of the reference pair.
    Portal(Box<TypeTag>),
    /// `(T, U, …)` — a structural tuple type.
    ///
    /// Distinct from the `tuple(…)` LAYOUT of a named type: this one is a tag
    /// usable in any tag position, that one is a `type_def`'s body. The
    /// production admits no empty tuple — a parsed `Tuple` always has at least
    /// one element.
    Tuple(Vec<TypeTag>),
    /// `[T; N]` — a fixed-size array, element type then length.
    ///
    /// The length is a STRUCTURAL number, converted to `u64` at parse time
    /// (R57): it describes the shape of storage rather than a value in it, so
    /// it is not a literal the loader re-reads under a type.
    Array(Box<TypeTag>, u64),
    /// `closure<(T, …) -> R>` — a closure environment type.
    ///
    /// The parameter list may be empty (`closure<() -> str>`); the return type
    /// may not be omitted.
    Closure {
        /// The parameter tags, in order; empty for a nullary closure.
        params: Vec<TypeTag>,
        /// The return tag.
        ret: Box<TypeTag>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // The recursion is owned, so a clone is a deep copy: mutating the original
    // afterwards cannot reach the copy. `TypeTag` has no equality by design
    // (R17), so the comparison is on `Debug` text.
    #[test]
    fn a_nested_tag_clones_deeply() {
        let tag = TypeTag::Handle(Box::new(TypeTag::Portal(Box::new(TypeTag::Array(
            Box::new(TypeTag::Prim(Prim::I32)),
            4,
        )))));
        let copy = tag.clone();

        assert_eq!(format!("{tag:?}"), format!("{copy:?}"));
        assert!(matches!(copy, TypeTag::Handle(_)));
    }

    #[test]
    fn a_closure_tag_carries_its_parameters_and_return() {
        let tag = TypeTag::Closure {
            params: vec![
                TypeTag::Prim(Prim::I32),
                TypeTag::TypeId("Request".to_string()),
            ],
            ret: Box::new(TypeTag::Prim(Prim::Bool)),
        };

        let TypeTag::Closure { params, ret } = tag else {
            panic!("the tag is a closure");
        };

        assert_eq!(params.len(), 2);
        assert!(matches!(*ret, TypeTag::Prim(Prim::Bool)));
    }
}
