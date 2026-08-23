//! The `CellIndex` newtype: a loaded cell's dense rank.

/// A loaded cell's dense index into the `cells` vector of a
/// [`LoadedGraph`](super::loaded_graph::LoadedGraph).
///
/// The index is ALSO the cell's rank in declaration order — scope-major, then
/// body order — because the loader assigns indices by walking the graph
/// section's scopes in section order and each scope's entries in body order
/// (FND-22/R64: graph-section entry order is the normative carrier of
/// §13.10.3's composed tiebreak). One number is therefore both "where the cell
/// lives" and the same-level evaluation tiebreak the static order sorts by;
/// reordering entries in the text is a semantic change, and this type is where
/// that law lands in the runtime.
///
/// A newtype rather than a bare `usize` so an index into `cells` can never be
/// confused with any other count the engine carries.
///
/// The payload is CRATE-VISIBLE: an index is only meaningful against the graph
/// the loader assigned it from, so nothing outside the crate should be able to
/// invent one or read one out as a bare number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellIndex(pub(crate) usize);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // `Copy` plus `Hash`/`Eq` is the whole contract: an index is passed around
    // by value and keys the engine's maps.
    #[test]
    fn an_index_copies_and_keys_a_map() {
        let index = CellIndex(3);
        let copy = index;

        assert_eq!(index, copy);
        assert_ne!(index, CellIndex(4));

        let mut ranks = HashMap::new();
        ranks.insert(index, "gain");

        assert_eq!(ranks.get(&CellIndex(3)), Some(&"gain"));
        assert_eq!(ranks.get(&CellIndex(4)), None);
    }
}
