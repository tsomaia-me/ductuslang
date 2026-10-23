//! The `LookbackEntry` struct: one `(input cell : k)` pair of a `lookback_map`.

use crate::path::CellPath;

/// One input cell and the deepest `.past(k, …)` read of it — IR_GRAMMAR §3's
/// `lookback_map` element (§15.4.6, 033-117).
///
/// ```text
/// lookback_map ::= '[' (PATH ':' INT (',' PATH ':' INT)*)? ']'
/// ```
///
/// The map drives per-input history allocation for a recurrent stream: each
/// entry names an input cell the stream's body reads via `.past(k, …)` and the
/// maximum `k` it reads. It is empty for a non-recurrent stream and for a
/// recurrent stream whose body calls no `.past` on inputs (033-118).
///
/// `k` is a STRUCTURAL number (R57): it counts the READING cell's own advances
/// back — not elapsed commits (023-71, 016-79) — rather than holding a value of
/// the cell's type, so it is a `u64` here rather than text for the loader.
///
/// **The colon rule here is NOT reopen_set's.** Both sides of a pair's colon-INT
/// are integers, so "an INT after a colon is an ordinal" cannot decide it. The
/// law is positional instead: the LAST `':' INT` of an entry is the lookback
/// value, and any earlier one belongs to the key path — so `App.print:0 : 3` is
/// the cell `App.print:0` at lookback 3 (R43, §3's scope note).
///
/// No span (R17): the entry rides its [`Stream`](crate::graph::Stream).
#[derive(Debug, Clone)]
pub struct LookbackEntry {
    /// The input cell read through `.past`.
    pub cell: CellPath,
    /// The deepest lookback of it, in the reading cell's own advances.
    pub k: u64,
}
