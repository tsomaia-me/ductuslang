//! The `Effect` struct: an `effect` entry of a scope.

use super::binding::Binding;
use crate::diag::Span;
use crate::path::CellPath;

/// One `effect` entry — IR_GRAMMAR §3's `effect` (§15.4.6, 033-119 / 033-120 /
/// 033-121 / 033-122 / 033-123).
///
/// ```text
/// effect ::= 'effect' PATH 'reconciler' STRING 'params' binding_set
///            ('desired' path_set)? ('observed' path_set)? ('gate' PATH)?
/// ```
///
/// `reconciler` names the reconciler as a STRING — its content, escapes already
/// processed, not the spelling it arrived in. `params` is mandatory, so an
/// empty vector there means the text spelled `[]`; `desired` and `observed` are
/// optional, so an empty vector there is the clause's absence.
///
/// `desired` names the per-declaration cells of the effect's `desired:` block —
/// each declaration is its own graph cell (§15.4.1's `desired_cell_ids`), and
/// the whole-record view the reconciler consumes is assembled by the runtime
/// from their current values rather than being a cell itself.
///
/// No `PartialEq` (R17); the span runs from the `effect` word through the last
/// clause.
#[derive(Debug, Clone)]
pub struct Effect {
    /// The effect instance's path.
    pub path: CellPath,
    /// The `effect` word through the entry's last token (R17).
    pub span: Span,
    /// `reconciler STRING` — the reconciler's name, unescaped.
    pub reconciler: String,
    /// `params binding_set` — the effect's arguments.
    pub params: Vec<Binding>,
    /// `desired path_set` — the desired-state cells; empty when absent.
    pub desired: Vec<CellPath>,
    /// `observed path_set` — the observed-state cells; empty when absent.
    pub observed: Vec<CellPath>,
    /// `gate PATH` — the gate that suspends and resumes it.
    pub gate: Option<CellPath>,
}
