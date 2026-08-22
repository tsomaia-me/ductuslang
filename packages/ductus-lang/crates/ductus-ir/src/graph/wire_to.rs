//! The `WireTo` enum: a wire's destination.

use crate::path::CellPath;

/// Where a wire ends — IR_GRAMMAR §3's `(PATH | 'null')` (§15.4.6).
///
/// ```text
/// wire ::= 'wire' PATH 'from' PATH 'to' (PATH | 'null') …
/// ```
///
/// An enum rather than an `Option<CellPath>` because `null` is EXPLICIT in the
/// grammar: it is an alternative the text spells out, not the absence of a
/// clause, and the `to` clause itself is mandatory. An `Option` would render
/// the two indistinguishable at the type level and invite an emitter to omit
/// what the grammar requires.
///
/// No `PartialEq` (R17): a composite AST node, compared by emitted text.
#[derive(Debug, Clone)]
pub enum WireTo {
    /// A destination cell.
    Path(CellPath),
    /// `null` — the wire is spelled with no destination.
    Null,
}
