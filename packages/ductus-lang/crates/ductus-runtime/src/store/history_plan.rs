//! The `HistoryPlan` struct: one cell's share of the history region, as the
//! store is asked to allocate it.

use crate::value::Value;

/// What history ONE cell needs — the allocation request a
/// [`CellStore`](super::CellStore) is built from (R78's fork-7 storage shape).
///
/// The store's history region is ragged: a `recurrent depth N` needs `N` past
/// slots, each `lookback` entry needs `k` more for its input, and every other
/// cell needs none. A plan per cell — in rank order, one for EVERY cell,
/// empty for the ones with no history — is what turns that raggedness into a
/// flat area plus an offset table, without the store having to know what a
/// recurrent is.
///
/// # The vectors are the fallbacks, and their length is the depth
///
/// A ring's depth is `fallbacks.len()`, so a plan cannot describe a ring with
/// no seed for one of its slots — the loader's `missing_fallback_key` law
/// (M1.md §5.1) expressed as a type. `own` is the cell's self-history
/// (index `k - 1` is what `.past(k, …)` yields before `k` advances exist), and
/// `inputs` holds one such vector per `lookback` entry, IN LIST ORDER, which is
/// the order the behavior ABI passes them in and therefore the order the rings
/// are addressed by (R71, §4.9).
///
/// An input's pre-history is not spellable in the `.duir` text form — the
/// `fallbacks` clause seeds self-history only (033-267) — so the engine seeds
/// these vectors with the input cell's own construction value
/// (§13.14.1 step 3), which is the only value of the input's type the runtime
/// has before the first advance.
#[derive(Debug, Clone)]
pub(crate) struct HistoryPlan {
    /// The self-history ring's per-`k` fallbacks; empty for a cell with no
    /// self-history.
    pub(crate) own: Vec<Value>,
    /// One per `lookback` entry in list order: that input's per-`k` fallbacks.
    pub(crate) inputs: Vec<Vec<Value>>,
}

impl HistoryPlan {
    /// The plan of a cell with no history at all — every `input` and every
    /// `derived`.
    pub(crate) fn none() -> HistoryPlan {
        HistoryPlan {
            own: Vec::new(),
            inputs: Vec::new(),
        }
    }

    /// How many rings this plan asks for: the self ring, if any, plus one per
    /// `lookback` entry.
    pub(crate) fn rings(&self) -> usize {
        usize::from(!self.own.is_empty()) + self.inputs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_historyless_cell_asks_for_no_rings() {
        let plan = HistoryPlan::none();

        assert_eq!(plan.rings(), 0);
        assert!(plan.own.is_empty());
        assert!(plan.inputs.is_empty());
    }

    // A `depth 2` recurrent with two `lookback` entries asks for three rings —
    // its own plus one per entry — and the seeds ride along with each.
    #[test]
    fn a_recurrent_plan_counts_its_own_ring_and_one_per_lookback() {
        let plan = HistoryPlan {
            own: vec![Value::I64(1), Value::I64(0)],
            inputs: vec![vec![Value::I32(0)], vec![Value::Bool(false)]],
        };

        assert_eq!(plan.rings(), 3);
        assert_eq!(plan.own.len(), 2);
    }
}
