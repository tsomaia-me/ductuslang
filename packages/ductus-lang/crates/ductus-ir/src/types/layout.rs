//! The `Layout` enum: a named type's body.

use crate::types::{Field, TypeTag, Variant};

/// A named type's body — the `layout` production (IR_GRAMMAR §3, §15.4.6):
///
/// ```text
/// layout ::= 'record' '{' field_list '}'
///          | 'enum'   '{' variant (',' variant)* '}'
///          | 'tuple'  '(' type_tag (',' type_tag)* ')'
/// ```
///
/// The three arities are not the same. A `record`'s `field_list` is OPTIONAL,
/// so `record {}` is a legal layout; an `enum` requires at least one variant
/// and a `tuple` at least one element, so a parsed `Enum` and a parsed `Tuple`
/// are never empty.
///
/// A layout is structure only. Size and alignment are not derived from it —
/// they are stated beside it on the `type_def` line, because the runtime's ABI,
/// not this grammar, is what fixes them.
#[derive(Debug, Clone)]
pub enum Layout {
    /// `record { name: tag, … }` — named fields in declaration order, possibly
    /// none.
    Record(Vec<Field>),
    /// `enum { #A(tag), #B }` — cases in declaration order, at least one.
    Enum(Vec<Variant>),
    /// `tuple(tag, …)` — positional element types, at least one.
    Tuple(Vec<TypeTag>),
}
