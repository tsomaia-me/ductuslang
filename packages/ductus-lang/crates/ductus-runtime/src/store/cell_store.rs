//! The `CellStore` struct: the storage seam — two committed buffers plus the
//! staged-write buffer, and the history region beside them.

use super::history_plan::HistoryPlan;
use crate::loader::CellIndex;
use crate::value::Value;

/// Every cell's value, held in the three buffers the write→commit→read loop
/// needs: two same-length COMMITTED buffers with a live-buffer index between
/// them, and one STAGED buffer the host writes into.
///
/// # Why double-buffered
///
/// §14.3.1 grants a backend full latitude over cell representation — "a backend
/// may represent cells however it likes as long as reads and writes are
/// value-preserving" — so this shape is a choice, and it is chosen to honor the
/// fixed-extent cost guarantee LITERALLY: a fixed-extent cell's slot is
/// pre-allocated at construction, the commit writes into reserved capacity
/// (double-buffered), and the publish is a constant-time index flip with no
/// allocation, realloc, or resize (§13.12.4; DECISION_LOG 025-61). The previous
/// committed values stay readable in the live buffer for the whole of a commit,
/// right up to the flip — which is what makes a [`Snapshot`](super::Snapshot)
/// coherent while the engine is mid-commit, and what lets step 1 and step 3 read
/// "previously-committed" without keeping a third copy.
///
/// # The three buffers
///
/// - **live** — the published values. Reads see this, and nothing else, until a
///   flip.
/// - **pending** — the off buffer. `begin_pending` refills it from live at the
///   top of every commit, the commit writes this commit's values into it, and
///   the flip makes it live.
/// - **staged** — the host's writes, accumulating between commits. Never
///   compared by anything but step 1, and never published directly: step 1
///   copies a dirty input's staged value into pending.
///
/// The buffers are all `graph.cells.len()` long and stay that length for the
/// engine's whole life; a cell's [`CellIndex`] is its slot in each of them.
///
/// # The history region is BESIDE them, not inside them
///
/// A `recurrent depth N` needs `N` past values on top of its current one, and
/// each of its `lookback` entries needs `k` more for an input — a ragged shape
/// the three flat buffers cannot hold without losing "one slot per cell per
/// buffer". So history lives in its own region (R78's fork-7 call): a flat
/// value area, a flat fallback area laid out identically, and one descriptor
/// per ring, addressed through a per-cell offset table. Three properties fall
/// out of that separation and are the reason for it:
///
/// - the M0 hot path is untouched — `begin_pending`'s `copy_from_slice` and the
///   index flip neither see nor copy history;
/// - history does NOT ride the publish (C-11): a ring is advanced once, in
///   step 4, and there is no second copy to keep in sync;
/// - an advance is O(1) whatever the depth — a head bump and one slot write,
///   never a shift of `N` values.
///
/// Crate-visible, not public: a host reaches the committed state through
/// [`Snapshot`](super::Snapshot) alone — history included, which is not
/// readable by path at all (R78's fork 8: a program observes it through a
/// derived that reads `.past`). Every method below is the engine's side of the
/// write→commit→read loop.
#[derive(Debug, Clone)]
pub(crate) struct CellStore {
    buffers: [Vec<Value>; 2],
    live_index: usize,
    staged: Vec<Value>,
    history: Vec<Value>,
    fallbacks: Vec<Value>,
    rings: Vec<Ring>,
    ring_index: Vec<usize>,
}

/// One history ring's descriptor — private plumbing of the region above.
///
/// `head` is the ring-relative slot holding the MOST RECENT value, so a read of
/// `.past(k, …)` lands at `(head + k - 1) % depth` and an advance walks `head`
/// backwards by one. `filled` counts the real advances present, saturating at
/// `depth`: a `k` beyond it has no value yet and reads its fallback instead
/// (§13.2.4.1's "a history read before the ring has advanced that far returns
/// the read's fallback").
#[derive(Debug, Clone, Copy)]
struct Ring {
    offset: usize,
    depth: usize,
    head: usize,
    filled: usize,
}

impl CellStore {
    /// Allocates the store from per-cell initial values and per-cell history
    /// plans (§13.14.1 step 2).
    ///
    /// All three flat buffers start as copies of `initial`, so the store is
    /// internally consistent before construction runs: a cell's live, pending
    /// and staged slots agree, and a commit performed against an untouched
    /// store is clean.
    ///
    /// `plans` is one entry per cell, in the same rank order — the store reads
    /// no roles and asks no questions about kinds; a cell with no history hands
    /// in an empty plan. Each plan's rings are laid out in ITS order (the self
    /// ring first, then one per `lookback` entry in list order), each ring's
    /// fallbacks copied into the fallback area and echoed into the value area
    /// so no slot holds an untyped placeholder. Every ring starts EMPTY —
    /// `filled` is zero — so every read answers from the fallbacks until the
    /// first advance.
    ///
    /// # Panics
    ///
    /// If `plans` is not the same length as `initial`: the two describe one
    /// graph, and a mismatch means the caller built them from different ones.
    pub(crate) fn new(initial: Vec<Value>, plans: &[HistoryPlan]) -> CellStore {
        assert_eq!(
            plans.len(),
            initial.len(),
            "one history plan per cell, in rank order"
        );

        let buffers = [initial.clone(), initial.clone()];
        let mut history: Vec<Value> = Vec::new();
        let mut rings: Vec<Ring> = Vec::with_capacity(plans.iter().map(HistoryPlan::rings).sum());
        let mut ring_index: Vec<usize> = Vec::with_capacity(plans.len() + 1);

        for plan in plans {
            assert!(
                plan.own.is_empty() == (plan.rings() == 0),
                "a plan with input rings has a self ring: the self ring is what the \
                 input rings are addressed relative to"
            );

            ring_index.push(rings.len());

            let seeds = std::iter::once(&plan.own)
                .filter(|own| !own.is_empty())
                .chain(plan.inputs.iter());

            for seed in seeds {
                rings.push(Ring {
                    offset: history.len(),
                    depth: seed.len(),
                    head: 0,
                    filled: 0,
                });
                history.extend_from_slice(seed);
            }
        }

        ring_index.push(rings.len());

        CellStore {
            buffers,
            live_index: 0,
            staged: initial,
            fallbacks: history.clone(),
            history,
            rings,
            ring_index,
        }
    }

    /// The published values — what a snapshot reads, and what "previously
    /// committed" means for the whole of a commit until step 5's flip.
    pub(crate) fn live(&self) -> &[Value] {
        &self.buffers[self.live_index]
    }

    /// `cell`'s staged value.
    pub(crate) fn staged(&self, cell: CellIndex) -> Value {
        self.staged[cell.0]
    }

    /// Stages a host write for `cell` (§13.10.1: staging only — no evaluation,
    /// no dirtiness judged here).
    ///
    /// Last write wins by construction: the slot is overwritten, so nothing
    /// remembers the intermediate values a net-revert pattern wrote.
    pub(crate) fn stage(&mut self, cell: CellIndex, value: Value) {
        self.staged[cell.0] = value;
    }

    /// Refills the pending buffer from live — the top of §13.10.2 step 1.
    ///
    /// A commit writes only the cells it dirties or evaluates, so every other
    /// slot must already hold the previous committed value for the flip to
    /// publish a complete snapshot. This copy is what puts it there, and it is
    /// also what makes an untouched cell's pending slot equal its committed
    /// value — the identity step 3's read law leans on.
    pub(crate) fn begin_pending(&mut self) {
        let [first, second] = &mut self.buffers;

        if self.live_index == 0 {
            second.copy_from_slice(first);
        } else {
            first.copy_from_slice(second);
        }
    }

    /// `cell`'s pending value — this commit's value if something wrote it, the
    /// previous committed value otherwise.
    pub(crate) fn pending(&self, cell: CellIndex) -> Value {
        self.buffers[self.live_index ^ 1][cell.0]
    }

    /// Writes `cell`'s value for this commit into the pending buffer.
    pub(crate) fn set_pending(&mut self, cell: CellIndex, value: Value) {
        self.buffers[self.live_index ^ 1][cell.0] = value;
    }

    /// Publishes the pending buffer — §13.10.2 step 5, as the constant-time
    /// index flip 025-61 requires. No copy, no allocation, no per-cell work.
    pub(crate) fn flip(&mut self) {
        self.live_index ^= 1;
    }

    /// Writes `value` into ALL THREE buffers at once — construction's write
    /// (§13.14.1 step 3), never a commit's.
    ///
    /// Construction is not a commit: there is no previous snapshot to preserve
    /// and nothing to publish, so a startup-evaluated derived's value belongs in
    /// both committed buffers (they must start identical — the first commit's
    /// flip publishes the other one) and in the staged slot (so no slot is left
    /// holding the placeholder the store was allocated with).
    pub(crate) fn seed(&mut self, cell: CellIndex, value: Value) {
        self.buffers[0][cell.0] = value;
        self.buffers[1][cell.0] = value;
        self.staged[cell.0] = value;
    }

    /// `cell`'s own value `k` ADVANCES ago — what `.past(k, …)` reads
    /// (§13.2.4.3), or the `k` fallback when the ring has advanced fewer than
    /// `k` times.
    ///
    /// "Advances", never commits (R72): a commit in which the recurrent did not
    /// evaluate consumed no slot, so nothing here counts elapsed time.
    ///
    /// # Panics
    ///
    /// If `cell` has no history, or `k` is zero or past the ring's depth — the
    /// loader admits neither (`invalid_history_depth`), so reaching either is a
    /// broken engine, not a bad program.
    pub(crate) fn past(&self, cell: CellIndex, k: usize) -> Value {
        self.read(self.own_ring(cell), k)
    }

    /// The `slot`-th `lookback` input's value `k` of `cell`'s advances ago.
    ///
    /// `slot` is the entry's position in the `lookback` list, which is also its
    /// position in the behavior's argument list (R71's ABI order, §4.9).
    ///
    /// # Panics
    ///
    /// As [`past`](CellStore::past), plus a `slot` past the cell's `lookback`
    /// list.
    pub(crate) fn input_past(&self, cell: CellIndex, slot: usize, k: usize) -> Value {
        self.read(self.input_ring(cell, slot), k)
    }

    /// Advances `cell`'s own ring by one, writing `value` as the newest past —
    /// §13.10.2 step 4, once per evaluation and never otherwise (R72 + R77).
    ///
    /// O(1): the head walks back one slot and one value is written. Nothing
    /// shifts, and an advance whose value equals the one the cell held still
    /// consumes a slot — value-change gating governs propagation, never the
    /// ring (§13.2.4.1's "evaluation implies advancement").
    pub(crate) fn advance_history(&mut self, cell: CellIndex, value: Value) {
        self.push(self.own_ring(cell), value);
    }

    /// Advances the `slot`-th `lookback` input's ring by one — the same law,
    /// stepped by the READING recurrent's advances (C-10: fork 2's option (iii)
    /// was not taken, so input history follows the same cadence as self
    /// history).
    pub(crate) fn advance_input_history(&mut self, cell: CellIndex, slot: usize, value: Value) {
        self.push(self.input_ring(cell, slot), value);
    }

    /// Seeds every fallback of the `slot`-th `lookback` input's ring with
    /// `value` — construction's answer to what an input's pre-history is
    /// (§13.14.1 step 3).
    ///
    /// The text form seeds SELF-history only (`fallbacks`/`init`, 033-267), so
    /// an input's history before the first advance has no spelling in the IR.
    /// The engine answers with the input cell's own construction value, which
    /// is the only value of that cell's type the runtime holds at that moment,
    /// and this is where it lands.
    pub(crate) fn seed_input_history(&mut self, cell: CellIndex, slot: usize, value: Value) {
        let ring = self.rings[self.input_ring(cell, slot)];

        for fallback in &mut self.fallbacks[ring.offset..ring.offset + ring.depth] {
            *fallback = value;
        }
    }

    /// Empties every ring `cell` owns — its self ring and each of its
    /// `lookback` input rings — so every history read answers from the
    /// fallbacks again (§13.2.4's `@reset_on_reopen`, C-13).
    ///
    /// Nothing is overwritten and nothing is copied: a ring is empty when
    /// `filled` is zero, because [`read`](CellStore::read) consults the value
    /// area only for a `k` the ring has actually advanced to. The head returns
    /// to the top for the same reason it starts there — an empty ring has no
    /// newest slot to point at.
    ///
    /// Called at step 1a on an EFFECTIVE reopen, before step 3's reads, which
    /// is what makes "the first post-gap evaluation reads fallbacks" true. A
    /// cell with no history is a no-op: its ring range is empty.
    pub(crate) fn clear_history(&mut self, cell: CellIndex) {
        for ring in &mut self.rings[self.ring_index[cell.0]..self.ring_index[cell.0 + 1]] {
            ring.head = 0;
            ring.filled = 0;
        }
    }

    // The self ring's position: the FIRST of the cell's rings, by the layout
    // `new` lays down.
    fn own_ring(&self, cell: CellIndex) -> usize {
        let first = self.ring_index[cell.0];

        assert!(
            first < self.ring_index[cell.0 + 1],
            "cell {} has no history ring",
            cell.0
        );

        first
    }

    // A lookback ring's position: after the self ring, in `lookback` list
    // order.
    fn input_ring(&self, cell: CellIndex, slot: usize) -> usize {
        let ring = self.own_ring(cell) + 1 + slot;

        assert!(
            ring < self.ring_index[cell.0 + 1],
            "cell {} has no lookback ring {slot}",
            cell.0
        );

        ring
    }

    // The read law, one place: a filled slot answers from the ring, an unfilled
    // one from the fallback for that exact `k`. Pre-seeding the ring with the
    // fallbacks instead would be wrong — the head moves, so after one advance
    // the slot `k = 2` addresses would hold `k = 1`'s seed.
    fn read(&self, ring: usize, k: usize) -> Value {
        let ring = self.rings[ring];

        assert!(
            k >= 1 && k <= ring.depth,
            "history depth {k} is outside 1..={}",
            ring.depth
        );

        if k <= ring.filled {
            self.history[ring.offset + (ring.head + k - 1) % ring.depth]
        } else {
            self.fallbacks[ring.offset + k - 1]
        }
    }

    // The advance law, one place: the head walks BACK one slot — so what was
    // `past(k)` becomes `past(k + 1)` and the oldest slot is the one overwritten
    // — and `filled` saturates at the depth.
    fn push(&mut self, ring: usize, value: Value) {
        let mut descriptor = self.rings[ring];

        descriptor.head = (descriptor.head + descriptor.depth - 1) % descriptor.depth;
        descriptor.filled = (descriptor.filled + 1).min(descriptor.depth);

        self.history[descriptor.offset + descriptor.head] = value;
        self.rings[ring] = descriptor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> CellStore {
        let plans = [
            HistoryPlan::none(),
            HistoryPlan::none(),
            HistoryPlan::none(),
        ];

        CellStore::new(
            vec![Value::I64(1), Value::I64(2), Value::Bool(false)],
            &plans,
        )
    }

    // Two cells, the second a `depth 2` recurrent with one `lookback [k = 3]`
    // entry over the first — the smallest store with a ragged history region.
    fn historied() -> CellStore {
        let plans = [
            HistoryPlan::none(),
            HistoryPlan {
                own: vec![Value::I64(1), Value::I64(0)],
                inputs: vec![vec![Value::I64(-1); 3]],
            },
        ];

        CellStore::new(vec![Value::I64(5), Value::I64(0)], &plans)
    }

    fn live(store: &CellStore) -> Vec<String> {
        store.live().iter().map(Value::render).collect()
    }

    // All three buffers start as the initial values, so a store nobody has
    // touched is already self-consistent.
    #[test]
    fn allocation_fills_every_buffer_with_the_initial_values() {
        let mut store = store();

        assert_eq!(live(&store), ["1", "2", "false"]);
        assert!(store.staged(CellIndex(1)).bits_eq(&Value::I64(2)));

        store.flip();

        assert_eq!(live(&store), ["1", "2", "false"]);
    }

    // Staging touches the staged buffer and nothing else: the published values
    // are unchanged until a commit moves them, which is §13.10.1's whole point.
    #[test]
    fn staging_leaves_the_published_values_alone() {
        let mut store = store();

        store.stage(CellIndex(0), Value::I64(7));

        assert!(store.staged(CellIndex(0)).bits_eq(&Value::I64(7)));
        assert_eq!(live(&store), ["1", "2", "false"]);
    }

    // Last write wins — the slot is overwritten, so the intermediate value is
    // gone by commit time.
    #[test]
    fn the_last_stage_wins() {
        let mut store = store();

        store.stage(CellIndex(0), Value::I64(7));
        store.stage(CellIndex(0), Value::I64(9));

        assert!(store.staged(CellIndex(0)).bits_eq(&Value::I64(9)));
    }

    // The pending buffer starts as a copy of live, so an untouched slot's
    // pending value IS its committed value.
    #[test]
    fn begin_pending_copies_the_live_values() {
        let mut store = store();

        store.begin_pending();

        assert!(store.pending(CellIndex(0)).bits_eq(&Value::I64(1)));
        assert!(store.pending(CellIndex(2)).bits_eq(&Value::Bool(false)));
    }

    // Writing pending is invisible until the flip: that gap is what keeps a
    // snapshot coherent while a commit is running.
    #[test]
    fn pending_writes_stay_invisible_until_the_flip() {
        let mut store = store();

        store.begin_pending();
        store.set_pending(CellIndex(0), Value::I64(7));

        assert_eq!(live(&store), ["1", "2", "false"]);
        assert!(store.pending(CellIndex(0)).bits_eq(&Value::I64(7)));

        store.flip();

        assert_eq!(live(&store), ["7", "2", "false"]);
    }

    // Two commits in a row: the second's `begin_pending` refills the buffer the
    // first one published FROM, so the cells the second commit does not touch
    // still publish their current values rather than the stale ones.
    #[test]
    fn a_second_pass_republishes_the_untouched_cells() {
        let mut store = store();

        store.begin_pending();
        store.set_pending(CellIndex(0), Value::I64(7));
        store.flip();

        store.begin_pending();
        store.set_pending(CellIndex(1), Value::I64(8));
        store.flip();

        assert_eq!(live(&store), ["7", "8", "false"]);
    }

    // Seeding is construction's write: every buffer at once, so the two
    // committed buffers start identical and no placeholder survives step 3.
    #[test]
    fn seeding_writes_every_buffer() {
        let mut store = store();

        store.seed(CellIndex(2), Value::Bool(true));

        assert_eq!(live(&store), ["1", "2", "true"]);
        assert!(store.staged(CellIndex(2)).bits_eq(&Value::Bool(true)));

        store.flip();

        assert_eq!(live(&store), ["1", "2", "true"]);
    }

    // Every ring starts EMPTY, so every read answers from its own `k`
    // fallback — the first-evaluation law (§13.2.4.1), self-history and input
    // history alike.
    #[test]
    fn an_unadvanced_ring_reads_its_per_depth_fallbacks() {
        let store = historied();

        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(1)));
        assert!(store.past(CellIndex(1), 2).bits_eq(&Value::I64(0)));
        assert!(
            store
                .input_past(CellIndex(1), 0, 3)
                .bits_eq(&Value::I64(-1))
        );
    }

    // One advance fills exactly one slot: `past(1)` is the value pushed and
    // `past(2)` is still `2`'s own fallback — never `1`'s, which is what a
    // pre-seeded ring would have answered once the head moved.
    #[test]
    fn one_advance_fills_one_slot_and_leaves_the_rest_on_fallbacks() {
        let mut store = historied();

        store.advance_history(CellIndex(1), Value::I64(7));

        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(7)));
        assert!(store.past(CellIndex(1), 2).bits_eq(&Value::I64(0)));
    }

    // The ring shifts by exactly one per advance, and the oldest value falls
    // off the end rather than lingering.
    #[test]
    fn each_advance_shifts_the_ring_by_one() {
        let mut store = historied();

        store.advance_history(CellIndex(1), Value::I64(7));
        store.advance_history(CellIndex(1), Value::I64(8));

        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(8)));
        assert!(store.past(CellIndex(1), 2).bits_eq(&Value::I64(7)));

        store.advance_history(CellIndex(1), Value::I64(9));

        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(9)));
        assert!(store.past(CellIndex(1), 2).bits_eq(&Value::I64(8)));
    }

    // R77: a repeated value still consumes a slot, so `past(2)` reads the
    // repeat rather than the value from before the two advances.
    #[test]
    fn an_equal_value_still_consumes_a_slot() {
        let mut store = historied();

        store.advance_history(CellIndex(1), Value::I64(4));
        store.advance_history(CellIndex(1), Value::I64(4));

        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(4)));
        assert!(store.past(CellIndex(1), 2).bits_eq(&Value::I64(4)));
    }

    // R65 in the ring: a NaN rides through history by its BITS, so the value
    // that comes back out is the pattern that went in.
    #[test]
    fn a_nan_rides_the_ring_by_its_bits() {
        let plans = [HistoryPlan {
            own: vec![Value::F64(0.0)],
            inputs: Vec::new(),
        }];
        let mut store = CellStore::new(vec![Value::F64(0.0)], &plans);

        store.advance_history(CellIndex(0), Value::F64(f64::NAN));

        assert!(store.past(CellIndex(0), 1).bits_eq(&Value::F64(f64::NAN)));
        assert!(!store.past(CellIndex(0), 1).bits_eq(&Value::F64(0.0)));
    }

    // The input rings are addressed by their `lookback` position and advance
    // independently of the self ring.
    #[test]
    fn input_rings_advance_beside_the_self_ring() {
        let mut store = historied();

        store.advance_history(CellIndex(1), Value::I64(7));
        store.advance_input_history(CellIndex(1), 0, Value::I64(5));
        store.advance_input_history(CellIndex(1), 0, Value::I64(6));

        assert!(store.input_past(CellIndex(1), 0, 1).bits_eq(&Value::I64(6)));
        assert!(store.input_past(CellIndex(1), 0, 2).bits_eq(&Value::I64(5)));
        assert!(
            store
                .input_past(CellIndex(1), 0, 3)
                .bits_eq(&Value::I64(-1))
        );
        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(7)));
    }

    // Construction's answer to an input's pre-history: seeding replaces every
    // unfilled depth's fallback at once.
    #[test]
    fn seeding_an_input_ring_replaces_every_fallback() {
        let mut store = historied();

        store.seed_input_history(CellIndex(1), 0, Value::I64(42));

        assert!(
            store
                .input_past(CellIndex(1), 0, 1)
                .bits_eq(&Value::I64(42))
        );
        assert!(
            store
                .input_past(CellIndex(1), 0, 3)
                .bits_eq(&Value::I64(42))
        );

        store.advance_input_history(CellIndex(1), 0, Value::I64(5));

        assert!(store.input_past(CellIndex(1), 0, 1).bits_eq(&Value::I64(5)));
        assert!(
            store
                .input_past(CellIndex(1), 0, 2)
                .bits_eq(&Value::I64(42))
        );
    }

    // `@reset_on_reopen` (C-13): clearing drops every advance the cell made,
    // self ring and input rings alike, so the next read answers from the
    // fallbacks exactly as it did before the first advance.
    #[test]
    fn clearing_a_cells_history_returns_every_ring_to_its_fallbacks() {
        let mut store = historied();

        store.advance_history(CellIndex(1), Value::I64(7));
        store.advance_history(CellIndex(1), Value::I64(8));
        store.advance_input_history(CellIndex(1), 0, Value::I64(5));

        store.clear_history(CellIndex(1));

        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(1)));
        assert!(store.past(CellIndex(1), 2).bits_eq(&Value::I64(0)));
        assert!(
            store
                .input_past(CellIndex(1), 0, 1)
                .bits_eq(&Value::I64(-1))
        );
    }

    // And the ring is genuinely EMPTY afterwards, not merely re-pointed: the
    // first advance after a clear fills slot 1 and leaves slot 2 on its own
    // fallback, which is the state a never-advanced ring is in.
    #[test]
    fn a_cleared_ring_refills_from_the_beginning() {
        let mut store = historied();

        store.advance_history(CellIndex(1), Value::I64(7));
        store.advance_history(CellIndex(1), Value::I64(8));
        store.clear_history(CellIndex(1));
        store.advance_history(CellIndex(1), Value::I64(9));

        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(9)));
        assert!(store.past(CellIndex(1), 2).bits_eq(&Value::I64(0)));
    }

    // A cell with no rings has nothing to clear, and asking is not an error:
    // the engine clears by cell, and only some cells have history.
    #[test]
    fn clearing_a_historyless_cell_does_nothing() {
        let mut store = historied();

        store.clear_history(CellIndex(0));

        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(1)));
    }

    // History does NOT ride the publish (C-11): the flip moves the value
    // buffers and leaves every ring exactly as it was.
    #[test]
    fn the_flip_leaves_history_alone() {
        let mut store = historied();

        store.advance_history(CellIndex(1), Value::I64(7));
        store.begin_pending();
        store.set_pending(CellIndex(1), Value::I64(7));
        store.flip();

        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(7)));
        assert!(store.past(CellIndex(1), 2).bits_eq(&Value::I64(0)));
    }

    // The region is per-cell: one cell's advances never reach another's ring.
    #[test]
    fn rings_are_addressed_per_cell() {
        let plans = [
            HistoryPlan {
                own: vec![Value::I64(0)],
                inputs: Vec::new(),
            },
            HistoryPlan {
                own: vec![Value::I64(0)],
                inputs: Vec::new(),
            },
        ];
        let mut store = CellStore::new(vec![Value::I64(0), Value::I64(0)], &plans);

        store.advance_history(CellIndex(0), Value::I64(1));

        assert!(store.past(CellIndex(0), 1).bits_eq(&Value::I64(1)));
        assert!(store.past(CellIndex(1), 1).bits_eq(&Value::I64(0)));
    }

    // A depth-1 ring is the degenerate case the head arithmetic must still
    // handle: every advance overwrites the one slot.
    #[test]
    fn a_depth_one_ring_holds_only_the_latest_advance() {
        let plans = [HistoryPlan {
            own: vec![Value::I64(0)],
            inputs: Vec::new(),
        }];
        let mut store = CellStore::new(vec![Value::I64(0)], &plans);

        store.advance_history(CellIndex(0), Value::I64(1));
        store.advance_history(CellIndex(0), Value::I64(2));

        assert!(store.past(CellIndex(0), 1).bits_eq(&Value::I64(2)));
    }

    // A depth past the ring is an engine bug, not a program error — the loader
    // rejects it as `invalid_history_depth` long before a read happens.
    #[test]
    #[should_panic(expected = "history depth 3 is outside 1..=2")]
    fn reading_past_the_rings_depth_panics() {
        historied().past(CellIndex(1), 3);
    }

    // And so is a history read of a cell that has none.
    #[test]
    #[should_panic(expected = "cell 0 has no history ring")]
    fn reading_history_of_a_historyless_cell_panics() {
        historied().past(CellIndex(0), 1);
    }
}
