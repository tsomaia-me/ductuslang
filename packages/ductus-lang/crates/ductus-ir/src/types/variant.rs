//! The `Variant` struct: one case of an `enum` layout.

use crate::types::TypeTag;

/// One `'#' NAME ('(' type_tag (',' type_tag)* ')')?` case of an `enum` layout
/// (IR_GRAMMAR §3, §15.4.6).
///
/// The optional payload list is represented by its EMPTINESS: `#None` parses to
/// an empty `payload` and emits without parentheses, `#Some(i32)` parses to a
/// one-tag payload and emits with them. There is no third state — the grammar
/// admits no `#None()`, so an empty parenthesis pair never round-trips into
/// existence.
///
/// Variants are held in declaration order by their layout, and that order is
/// meaning (discriminant assignment is the loader's, over this order).
#[derive(Debug, Clone)]
pub struct Variant {
    /// The variant's NAME, without the `#` sigil.
    pub name: String,
    /// The payload tags in order; EMPTY means the variant carries no payload
    /// and is written without parentheses.
    pub payload: Vec<TypeTag>,
}
