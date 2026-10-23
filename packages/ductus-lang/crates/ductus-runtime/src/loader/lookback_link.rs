//! The `LookbackLink` struct: one linked `lookback` entry of a recurrent.

use super::cell_index::CellIndex;

/// One `lookback` pair after linking — an input cell as a dense index, and how
/// many of ITS past values the recurrent's behavior reads (R71, §4.9).
///
/// The text form's [`LookbackEntry`](ductus_ir::graph::LookbackEntry) is a path
/// and a `u64`; this is the same pair with the path resolved and the depth in
/// the unit the store allocates rings in. The loader has already checked that
/// the cell is among the recurrent's own `inputs` (`lookback_not_an_input`) and
/// that `k` is positive (`invalid_history_depth`), so nothing downstream
/// re-asks either question.
///
/// **Order is ABI.** The links ride in `lookback` list order, and that order is
/// the order their `<input>_past1 … <input>_pastK` arguments arrive in
/// (§4.9's third clause). It is also the order the store allocates the rings
/// in, which is what lets a ring be addressed by its position in this list.
#[derive(Debug, Clone, Copy)]
pub struct LookbackLink {
    /// The input cell whose past values are read.
    pub(crate) input: CellIndex,
    /// The deepest `.past(k, …)` of it the behavior reads — the ring's depth.
    pub(crate) k: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    // The pair is plain data; the loader's own tests are where the checks that
    // produce it are pinned.
    #[test]
    fn a_link_carries_its_input_and_depth() {
        let link = LookbackLink {
            input: CellIndex(2),
            k: 3,
        };
        let copy = link;

        assert_eq!(copy.input, CellIndex(2));
        assert_eq!(copy.k, 3);
    }
}
