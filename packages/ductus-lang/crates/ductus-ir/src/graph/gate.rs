//! The `Gate` struct: a `gate` entry of a scope.

use crate::diag::Span;
use crate::path::CellPath;

/// One `gate` entry — IR_GRAMMAR §3's `gate` (§15.4.6, 033-168 / 033-65).
///
/// ```text
/// gate ::= 'gate' PATH 'pred' BID 'inputs' path_set 'guards' path_set
///          ('gate_parent' PATH)?
/// ```
///
/// A gate names a predicate behavior, the input cells that predicate reads, and
/// the constructs it guards. `gate_parent` is the only optional clause: it
/// names the enclosing gate when gates nest.
///
/// `inputs` and `guards` are MANDATORY clauses, so an empty vector here means
/// the text spelled `[]` — unlike the optional set clauses elsewhere, where
/// empty means the clause was absent. The distinction is the grammar's, not a
/// convention: what is required is always written.
///
/// `pred` carries the `u32` a `BID` decodes to.
///
/// No `PartialEq` (R17); the span runs from the `gate` word through the last
/// clause.
#[derive(Debug, Clone)]
pub struct Gate {
    /// The gate's path.
    pub path: CellPath,
    /// The `gate` word through the entry's last token (R17).
    pub span: Span,
    /// `pred BID` — the predicate behavior handle.
    pub pred: u32,
    /// `inputs path_set` — the cells the predicate reads.
    pub inputs: Vec<CellPath>,
    /// `guards path_set` — what this gate guards.
    pub guards: Vec<CellPath>,
    /// `gate_parent PATH` — the enclosing gate, when gates nest.
    pub gate_parent: Option<CellPath>,
}
