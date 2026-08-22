//! The `Cell` struct: a `cell` entry of a scope.

use super::cell_kind::CellKind;
use super::member::Member;
use super::value_literal::ValueLiteral;
use crate::diag::Span;
use crate::path::CellPath;
use crate::types::TypeTag;

/// One `cell` entry — IR_GRAMMAR §3's `cell` (§15.4.6, 033-169 / 033-215 /
/// 033-81).
///
/// ```text
/// cell ::= cell_kind PATH ':' type_tag
///          ('uses' BID)? ('inputs' path_set)? ('depth' INT)?
///          ('reset_on_reopen')? ('init' value)? ('gate' PATH)?
///          ('combiner' BID)? ('else' value)? ('members' member_set)?
/// ```
///
/// A cell is KIND-LED: the kind word leads the line, parallel to `gate`,
/// `wire`, `effect` and `stream`. Everything after the type tag is an optional
/// clause, each admitted AT MOST ONCE and only in the order the production
/// lists.
///
/// # What the parser does not check
///
/// Which clauses a KIND admits — `uses`/`inputs` on `derived`/`recurrent`/
/// `deferred`, `depth` on `recurrent`/`deferred`, `init` on `input`/
/// `recurrent`, `combiner`/`else` on `fold` — is the LOADER's (F4, R21). The
/// parser admits any combination in the fixed order, so `input App.x : i32
/// combiner B@1` parses and is rejected at load. The grammar is what this type
/// mirrors; legality is somewhere else on purpose.
///
/// # Absent clause versus empty set
///
/// `inputs` and `members` are optional clauses holding a possibly-empty set, so
/// "clause absent" and "clause present but empty" would be two spellings of one
/// state. They are ONE state here — an empty `Vec` — and the canonical
/// rendering of that state is the clause OMITTED. `inputs []` therefore parses
/// and canonicalizes away, which is the emitter-as-canonicalizer rule (R54)
/// doing what it does to any other non-canonical spelling.
///
/// # Numbers
///
/// `depth` is STRUCTURAL and converts at parse time (R57); `init` and `else`
/// are TYPED and stay text until the loader reads them under this cell's
/// `tag`. `uses` and `combiner` carry the `u32` a `BID` decodes to — the
/// behavior handle itself, hex being a spelling of it.
///
/// No `PartialEq` (R17); the span is the entry's own, from its kind word
/// through its last clause.
#[derive(Debug, Clone)]
pub struct Cell {
    /// The kind word that leads the line.
    pub kind: CellKind,
    /// The cell's path.
    pub path: CellPath,
    /// The kind word through the entry's last token (R17).
    pub span: Span,
    /// The cell's type, after the `:`.
    pub tag: TypeTag,
    /// `uses BID` — the behavior handle that computes it.
    pub uses: Option<u32>,
    /// `inputs path_set` — its input cells; empty when the clause is absent.
    pub inputs: Vec<CellPath>,
    /// `depth INT` — history depth for a recurrent, latency in commits for a
    /// deferred (033-245).
    pub depth: Option<u64>,
    /// `reset_on_reopen` — the bare flag form of the keyword (033-215).
    pub reset_on_reopen: bool,
    /// `init value` — the initial value.
    pub init: Option<ValueLiteral>,
    /// `gate PATH` — the gate that guards it.
    pub gate: Option<CellPath>,
    /// `combiner BID` — a fold cell's `by:` combiner handle.
    pub combiner: Option<u32>,
    /// `else value` — a fold cell's empty-membership result.
    pub else_value: Option<ValueLiteral>,
    /// `members member_set` — the member edges in member order; empty when the
    /// clause is absent.
    pub members: Vec<Member>,
}
