//! The `GateIndex` newtype: a loaded gate's dense rank among the gates.

/// A loaded gate's dense index into the `gates` vector of a
/// [`LoadedGraph`](super::loaded_graph::LoadedGraph).
///
/// It is the gate's order among GATES, assigned by the same Pass-A walk that
/// assigns [`CellIndex`](super::cell_index::CellIndex) — scopes in section
/// order, entries in body order. Gates and cells are two dense sequences over
/// one interleaved walk, so this index orders the gates among themselves and
/// says nothing about where a gate sits relative to a cell. That second
/// question is what [`LoadedGate`](super::loaded_gate::LoadedGate)'s
/// `entry_rank` answers, and C-45 is why it has to be asked at all: a `gate`
/// entry takes
/// graph-section entry rank exactly like every other entry, so step 1's root
/// walk interleaves gate transitions with cell roots by that rank (R76).
///
/// A newtype rather than a bare `usize` so a gate's index can never be passed
/// where a cell's is wanted — the two are dense over different sequences and
/// would silently alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GateIndex(pub(crate) usize);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::CellIndex;

    #[test]
    fn an_index_copies_and_compares() {
        let index = GateIndex(2);
        let copy = index;

        assert_eq!(index, copy);
        assert_ne!(index, GateIndex(3));
    }

    // The point of the newtype: the two dense sequences do not unify, so no
    // `GateIndex` can stand where a `CellIndex` is wanted. That the two types
    // wrap the same payload is exactly why the distinction has to be in the
    // type system rather than in a convention.
    #[test]
    fn a_gate_index_is_not_a_cell_index() {
        let gate = GateIndex(1);
        let cell = CellIndex(1);

        assert_eq!(format!("{gate:?}"), "GateIndex(1)");
        assert_eq!(format!("{cell:?}"), "CellIndex(1)");
    }
}
