//! The `Wire` struct: a `wire` entry of a scope.

use super::binding::Binding;
use super::wire_to::WireTo;
use crate::diag::Span;
use crate::path::CellPath;
use crate::types::TypeTag;

/// One `wire` entry — IR_GRAMMAR §3's `wire` (§15.4.6, 033-86 / 033-87 /
/// 033-88 / 033-89).
///
/// ```text
/// wire ::= 'wire' PATH 'from' PATH 'to' (PATH | 'null')
///          ('type' type_tag)? ('attrs' binding_set)? ('gate' PATH)?
/// ```
///
/// `from` and `to` are mandatory; `to` may be the explicit `null`, which is why
/// it is a [`WireTo`] rather than an `Option`.
///
/// `attrs` is an optional clause holding a possibly-empty set, so — as on a
/// cell's `inputs` — an empty vector IS the clause's absence, and `attrs []`
/// canonicalizes to omission.
///
/// No `PartialEq` (R17); the span runs from the `wire` word through the last
/// clause.
#[derive(Debug, Clone)]
pub struct Wire {
    /// The wire's path.
    pub path: CellPath,
    /// The `wire` word through the entry's last token (R17).
    pub span: Span,
    /// `from PATH` — the source.
    pub from: CellPath,
    /// `to (PATH | 'null')` — the destination.
    pub to: WireTo,
    /// `type type_tag` — the wire's declared type.
    pub tag: Option<TypeTag>,
    /// `attrs binding_set` — its attributes; empty when the clause is absent.
    pub attrs: Vec<Binding>,
    /// `gate PATH` — the gate that guards it.
    pub gate: Option<CellPath>,
}
