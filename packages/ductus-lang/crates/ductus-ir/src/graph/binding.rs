//! The `Binding` struct: one named argument of a `params` or `attrs` set.

use super::binding_target::BindingTarget;
use super::provenance::Provenance;

/// One named binding — IR_GRAMMAR §3's `binding` (§15.4.6, 033-252).
///
/// ```text
/// binding_set ::= '[' (binding (',' binding)*)? ']'
/// binding     ::= NAME ':' (PATH | value) ('@' provenance)?
/// ```
///
/// A binding set is an effect's `params` and a wire's `attrs`. The set holds
/// its bindings in FILE ORDER and nothing sorts them, so the round trip is
/// byte-stable; whether a name may repeat is the loader's question (F4).
///
/// The `@`-suffixed provenance is optional METADATA recording how the cell
/// argument was materialized under the uniform cell-argument rule (§13.17.3) —
/// never a distinct binding case (033-252), which is why it is an `Option`
/// here rather than a third target alternative.
///
/// No span (R17): a binding rides the named entry that holds it — the effect or
/// the wire — and that entry carries the span a diagnostic points with.
#[derive(Debug, Clone)]
pub struct Binding {
    /// The parameter or attribute NAME.
    pub name: String,
    /// What it binds to: a cell path, or a literal.
    pub target: BindingTarget,
    /// How the argument was materialized, when the text records it.
    pub provenance: Option<Provenance>,
}
