//! The `Stream` struct: a `stream` entry of a scope.

use super::lookback_entry::LookbackEntry;
use super::stream_policy::StreamPolicy;
use crate::diag::Span;
use crate::path::CellPath;
use crate::types::TypeTag;

/// One `stream` entry — IR_GRAMMAR §3's `stream` (§15.4.6, 033-106 … 033-117).
///
/// ```text
/// stream ::= 'stream' PATH ':' type_tag 'policy' ('ring' | 'gate') 'capacity' INT
///            ('source_deps' path_set)? ('observes' path_set)?
///            ('reset_on_reload')? ('reset_on_reopen')?
///            ('history' INT)? ('lookback' lookback_map)?
/// ```
///
/// The line serializes the stream primitive (§15.4.1's Stream cells) with its
/// ten data-model fields. `path` is the stream's id and `tag` its element type;
/// `policy` and `capacity` are mandatory; the rest are optional clauses in the
/// production's order, each at most once.
///
/// `observes` names the six synthesized observation cells — `pending_count`,
/// `pressure`, `is_full`, `dropped_total`, `rejected_total`,
/// `last_overflow_at`. The count is semantics, not grammar: any path set
/// parses here and the loader checks what it names (F4).
///
/// `history` is `output_history_size` and is present only on a recurrent
/// stream; `lookback` is the input lookback map (033-117).
///
/// `capacity`, `history` and each lookback `k` are STRUCTURAL numbers, so they
/// convert to `u64` at parse time (R57).
///
/// The two bare flags mark the matching `@`-annotations. `reset_on_reopen` here
/// is the THIRD grammar position of that keyword — a bare flag on `stream`,
/// alongside the bare flag on `cell` and the `reopen_set` payload on `scope`
/// (033-215).
///
/// The optional set clauses follow the same rule as everywhere else: an empty
/// vector is the clause's absence, and `source_deps []` canonicalizes to
/// omission.
///
/// No `PartialEq` (R17); the span runs from the `stream` word through the last
/// clause.
#[derive(Debug, Clone)]
pub struct Stream {
    /// The stream's id.
    pub path: CellPath,
    /// The `stream` word through the entry's last token (R17).
    pub span: Span,
    /// The element type, after the `:`.
    pub tag: TypeTag,
    /// `policy (ring | gate)` — the overflow policy.
    pub policy: StreamPolicy,
    /// `capacity INT` — the buffer capacity.
    pub capacity: u64,
    /// `source_deps path_set` — the source dependencies; empty when absent.
    pub source_deps: Vec<CellPath>,
    /// `observes path_set` — the synthesized observation cells; empty when
    /// absent.
    pub observes: Vec<CellPath>,
    /// `reset_on_reload` — the bare flag.
    pub reset_on_reload: bool,
    /// `reset_on_reopen` — the bare flag (033-215's third position).
    pub reset_on_reopen: bool,
    /// `history INT` — `output_history_size`, on a recurrent stream.
    pub history: Option<u64>,
    /// `lookback lookback_map` — the input lookback map; empty when absent.
    pub lookback: Vec<LookbackEntry>,
}
