//! The `ReopenPair` struct: one `(consumer : stream)` pair of a `reopen_set`.

use crate::path::CellPath;

/// One consumer-and-stream pair — IR_GRAMMAR §3's `reopen_set` element
/// (§15.4.6, 033-75).
///
/// ```text
/// reopen_set ::= '[' (PATH ':' PATH (',' PATH ':' PATH)*)? ']'
/// ```
///
/// The elements are `(consumer_id : stream_id)` pairs naming the consuming
/// operator or derived instance and the consumed stream whose cursor resets to
/// head on the scope's gate reopen edge.
///
/// The pair has no nonterminal of its own in the grammar — the production
/// spells `PATH ':' PATH` out — but it is one thing with two named halves, and
/// naming them here is what keeps `consumer` and `stream` from being told apart
/// by tuple position at every use site.
///
/// **The colon is doubly used**, and the grammar rules the ambiguity: each
/// `PATH` may itself end with a `:N` ordinal, so after a colon an INT is the
/// left path's ordinal and a NAME begins the right path (§3's boundary note).
///
/// No span (R17): the pair rides its [`Scope`](crate::graph::Scope), which
/// carries one.
#[derive(Debug, Clone)]
pub struct ReopenPair {
    /// The consuming operator or derived instance.
    pub consumer: CellPath,
    /// The consumed stream, whose cursor resets to head on reopen.
    pub stream: CellPath,
}
