//! The `Segment` enum: one step of a cell path.

/// One step of a cell path.
///
/// M0 has named segments only. Further segment forms — the corpus's keyed
/// `.<key>.` segments and `:N` ordinal suffixes (§15.4.1.1) — land at their
/// milestones; this enum grows then (R5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Segment {
    /// A declared identifier — the `gain` in `app.mixer.gain`.
    Name(String),
}
