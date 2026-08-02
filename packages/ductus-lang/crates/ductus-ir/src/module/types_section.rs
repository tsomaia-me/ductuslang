//! The `TypesSection` struct: a module's `types` table.

use crate::diag::Span;
use crate::types::TypeDef;

/// The `types` section: `'types' '{' type_def* '}'` (IR_GRAMMAR §3, §15.4.6).
///
/// The rows are held in FILE ORDER and nothing sorts them: the emitter writes
/// them back in that order, so order is what makes the round trip byte-stable.
/// It carries no other meaning — a type may reference one declared below it,
/// resolution being the loader's (F4).
///
/// The `*` in the production is literal: `types {}` is a legal, empty table,
/// and it emits as `types {}` on one line rather than as an empty block.
///
/// The span is the section's own (R17), so a load-time diagnostic about the
/// type table can point at the table even when the table is empty.
#[derive(Debug, Clone)]
pub struct TypesSection {
    /// The `types` keyword through the section's closing `}`.
    pub span: Span,
    /// The table's rows, in file order.
    pub type_defs: Vec<TypeDef>,
}
