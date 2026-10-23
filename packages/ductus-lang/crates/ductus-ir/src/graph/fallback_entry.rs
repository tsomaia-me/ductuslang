//! The `FallbackEntry` struct: one `(depth : value)` pair of a `fallback_map`.

use super::value_literal::ValueLiteral;

/// One history depth and the value a `.past(k, …)` read yields before that
/// depth exists — IR_GRAMMAR §3's `fallback_map` element (§15.4.6, 033-267).
///
/// ```text
/// fallback_map ::= '[' (INT ':' value (',' INT ':' value)*)? ']'
/// ```
///
/// The map seeds a recurrent cell's own history: key `k` is what the body reads
/// at `.past(k, …)` while fewer than `k` advances exist. `depth 2 fallbacks
/// [1: 1, 2: 0]` is the corpus's `fib` — one seed per slot, each read until its
/// slot is filled.
///
/// **`init` is the same clause in its uniform spelling.** On a `recurrent`,
/// `init v` says every depth falls back to `v` and `fallbacks` says them one by
/// one; carrying BOTH is a LOAD error, and the canonical rendering picks `init`
/// when every depth shares one value (033-268). Neither half of that is decided
/// here — this type is the general form's shape and nothing more.
///
/// # Numbers
///
/// `k` is STRUCTURAL and converts at parse time (R57): it counts the cell's own
/// ADVANCES back — not elapsed commits (023-71, 016-77) — rather than holding a
/// value of the cell's type. The `value` is TYPED and stays text until the
/// loader reads it under the cell's `tag`, exactly as `init` and `else` do.
///
/// No colon ambiguity arises in this production — the key is an `INT` and not a
/// `PATH`, so an entry has exactly one colon and `lookback_map`'s positional
/// rule (R43) is not needed here.
///
/// No span (R17): the entry rides its [`Cell`](crate::graph::Cell).
#[derive(Debug, Clone)]
pub struct FallbackEntry {
    /// The history depth this seed answers for.
    pub k: u64,
    /// What `.past(k, …)` yields before `k` advances exist.
    pub value: ValueLiteral,
}
