use super::layout::Layout;

/// The summary of a history read's fallback whose shape differs from its cell's.
pub(crate) const FALLBACK_SHAPE: &str =
    "a history read's fallback has the cell's type, and this one's shape differs";

/// Whether a value held for a cell, a fallback or an init, has the cell's shape
/// (SPEC §6.6).
pub(crate) fn held_agrees(cell: Option<&Layout>, held: Option<&Layout>) -> bool {
    match (cell, held) {
        (None, None) => true,
        (Some(cell), Some(held)) if cell.variable => {
            held.inner() == cell.inner() || (held.outer() == Some(0) && held.inner().is_empty())
        }
        (Some(cell), Some(held)) => cell == held && !held.variable,
        (None, Some(_)) | (Some(_), None) => false,
    }
}
