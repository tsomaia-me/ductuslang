//! The `Scope` struct: one `scope` of the graph section.

use super::entry::Entry;
use super::reopen_pair::ReopenPair;
use crate::diag::Span;
use crate::path::CellPath;

/// One `scope` — IR_GRAMMAR §3's `scope` (§15.4.6, 033-168 / 033-75).
///
/// ```text
/// graph_section ::= 'graph' '{' scope* '}'
/// scope         ::= 'scope' PATH 'exposes' path_set 'effects' path_set
///                   ('reset_on_reopen' reopen_set)? '{' entry* '}'
/// ```
///
/// A scope is a BRACED-BODY entry (033-250): it may span physical lines between
/// its braces, its own entries are terminated by NEWLINE or by its closing `}`,
/// and the scope itself is terminated by the NEWLINE after that `}`.
///
/// Scopes are FLAT under the graph section — `scope*`, never nested — and the
/// nesting a program has is carried by the scope's PATH instead: `scope
/// App.panel` is inside `scope App` by name, not by position (§15.4.1.1). So
/// this type holds entries and never other scopes.
///
/// `exposes` and `effects` are MANDATORY clauses, so an empty vector there
/// means the text spelled `[]`. `reset_on_reopen` is optional and carries a
/// set, so an empty vector there means the clause was ABSENT — the same rule
/// the optional set clauses follow everywhere else. This is the first of that
/// keyword's three grammar positions: the `reopen_set` payload here, the bare
/// flag on a cell, and the bare flag on a stream (033-215).
///
/// No `PartialEq` (R17); the span runs from the `scope` word through the
/// closing `}`.
#[derive(Debug, Clone)]
pub struct Scope {
    /// The scope's path.
    pub path: CellPath,
    /// The `scope` word through the closing `}` (R17).
    pub span: Span,
    /// `exposes path_set` — the topology this scope exposes.
    pub exposes: Vec<CellPath>,
    /// `effects path_set` — the effects it holds (not topology, §13.3.8).
    pub effects: Vec<CellPath>,
    /// `reset_on_reopen reopen_set` — the consumer/stream pairs whose cursors
    /// reset on the gate reopen edge; empty when the clause is absent.
    pub reset_on_reopen: Vec<ReopenPair>,
    /// The body's entries, in file order.
    pub entries: Vec<Entry>,
}
