//! The `TypesSection` struct: a module's `types` table.

use crate::diag::Span;

/// The `types` section: `'types' '{' type_def* '}'` (IR_GRAMMAR §3, §15.4.6).
///
/// Empty at increment 4 — the section parses and emits as `types {}`, and its
/// `type_def` rows arrive at increment 5, where this struct gains the field
/// that holds them.
///
/// The span is here from the start (R17) so a load-time diagnostic about the
/// type table can point at the section even before the table has content.
#[derive(Debug, Clone)]
pub struct TypesSection {
    /// The `types` keyword through the section's closing `}`.
    pub span: Span,
}
