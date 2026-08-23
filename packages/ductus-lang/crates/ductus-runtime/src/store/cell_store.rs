//! The `CellStore` struct: the M0 storage seam — two committed buffers plus
//! the staged-write buffer.

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
/// Crate-visible, not public: a host reaches the committed state through
/// [`Snapshot`](super::Snapshot) alone, and every method below is the engine's
/// side of the write→commit→read loop.
#[derive(Debug, Clone)]
pub(crate) struct CellStore {
    buffers: [Vec<Value>; 2],
    live_index: usize,
    staged: Vec<Value>,
}

impl CellStore {
    /// Allocates the store from per-cell initial values (§13.14.1 step 2).
    ///
    /// All three buffers start as copies of `initial`, so the store is
    /// internally consistent before construction runs: a cell's live, pending
    /// and staged slots agree, and a commit performed against an untouched
    /// store is clean.
    pub(crate) fn new(initial: Vec<Value>) -> CellStore {
        let buffers = [initial.clone(), initial.clone()];

        CellStore {
            buffers,
            live_index: 0,
            staged: initial,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> CellStore {
        CellStore::new(vec![Value::I64(1), Value::I64(2), Value::Bool(false)])
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
}
