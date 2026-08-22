//! The `BindingTarget` enum: what a binding binds to.

use super::value_literal::ValueLiteral;
use crate::path::CellPath;

/// The right-hand side of a `binding` — IR_GRAMMAR §3's `(PATH | value)`
/// (§15.4.6).
///
/// ```text
/// binding ::= NAME ':' (PATH | value) ('@' provenance)?
/// ```
///
/// The two alternatives are the two placement binding kinds (§13.8.4,
/// §15.4.1): a `PATH` is REACTIVE — it references source cells, and the
/// binding tracks them — while a `value` is STATIC, a compile-time literal
/// with nothing to track.
///
/// The provenance marker is not here: it is optional metadata ON the binding
/// and applies to either alternative (033-252), so it rides
/// [`Binding`](crate::graph::Binding) instead of splitting this enum in two.
///
/// No `PartialEq` (R17): a composite AST node, compared by emitted text.
#[derive(Debug, Clone)]
pub enum BindingTarget {
    /// A cell path — a reactive binding.
    Path(CellPath),
    /// A compile-time literal — a static binding.
    Value(ValueLiteral),
}
