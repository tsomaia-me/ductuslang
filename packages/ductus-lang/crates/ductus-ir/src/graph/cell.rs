//! The `Cell` struct: a `cell` entry of a scope.

use super::cell_kind::CellKind;
use super::fallback_entry::FallbackEntry;
use super::lookback_entry::LookbackEntry;
use super::member::Member;
use super::value_literal::ValueLiteral;
use crate::diag::Span;
use crate::path::CellPath;
use crate::types::TypeTag;

/// One `cell` entry — IR_GRAMMAR §3's `cell` (§15.4.6, 033-169 / 033-215 /
/// 033-81 / 033-266 / 033-267).
///
/// ```text
/// cell ::= cell_kind PATH ':' type_tag
///          ('uses' BID)? ('inputs' path_set)? ('depth' INT)?
///          ('lookback' lookback_map)?
///          ('reset_on_reopen')? ('init' value)?
///          ('fallbacks' fallback_map)? ('gate' PATH)?
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
/// `deferred`, `depth` on `recurrent`/`deferred`, `lookback`/`fallbacks` on
/// `recurrent`, `init` on `input`/`recurrent`, `combiner`/`else` on `fold` — is
/// the LOADER's (F4, R21). The parser admits any combination in the fixed
/// order, so `input App.x : i32 combiner B@1` parses and is rejected at load.
/// The grammar is what this type mirrors; legality is somewhere else on
/// purpose.
///
/// That division is what settles `init` against `fallbacks`. On a `recurrent`
/// the two are ONE clause in two spellings — `init v` says every depth falls
/// back to `v`, `fallbacks` says them one by one — and carrying both is a LOAD
/// error (033-268). Both fields hold a value here regardless: a module that
/// means two things is the loader's to refuse, not this type's to prevent.
///
/// The canonical SPELLING of a well-formed pair is the emitter's, and it is
/// picked BY CONTENT (033-268): a `fallbacks` map that seeds every depth with
/// one value is written `init v`, since that is what it means. So these two
/// fields are the one place where a cell's rendering depends on its VALUES and
/// not on its shape alone.
///
/// # Absent clause versus empty set
///
/// `inputs`, `lookback`, `fallbacks` and `members` are optional clauses holding
/// a possibly-empty set, so "clause absent" and "clause present but empty"
/// would be two spellings of one state. They are ONE state here — an empty
/// `Vec` — and the canonical rendering of that state is the clause OMITTED.
/// `inputs []` therefore parses and canonicalizes away, which is the
/// emitter-as-canonicalizer rule (R54) doing what it does to any other
/// non-canonical spelling.
///
/// # Numbers
///
/// `depth` and every `lookback` / `fallbacks` key are STRUCTURAL and convert at
/// parse time (R57); `init`, `else` and every `fallbacks` VALUE are TYPED and
/// stay text until the loader reads them under this cell's `tag`. `uses` and
/// `combiner` carry the `u32` a `BID` decodes to — the behavior handle itself,
/// hex being a spelling of it.
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
    /// `depth INT` — SELF-history depth for a recurrent, latency in commits for
    /// a deferred (033-245).
    pub depth: Option<u64>,
    /// `lookback lookback_map` — a recurrent's per-INPUT history depths, each
    /// pair an input cell mapped to the deepest `.past(k, …)` its body reads of
    /// that cell; empty when the clause is absent (033-266).
    pub lookback: Vec<LookbackEntry>,
    /// `reset_on_reopen` — the bare flag form of the keyword (033-215).
    pub reset_on_reopen: bool,
    /// `init value` — the initial value; on a recurrent, the UNIFORM spelling
    /// of `fallbacks` (033-268).
    pub init: Option<ValueLiteral>,
    /// `fallbacks fallback_map` — a recurrent's per-depth history seeds; empty
    /// when the clause is absent (033-267).
    pub fallbacks: Vec<FallbackEntry>,
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
