//! The `Engine` struct: the write→commit→read loop, and §13.10.2's six steps.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use ductus_ir::diag::Diagnostic;
use ductus_ir::path::CellPath;

use super::transaction::Transaction;
use super::write_check::write_check;
use crate::loader::{CellIndex, CellRole, GateIndex, LoadedCell, LoadedGraph, LookbackLink};
use crate::provider::Behavior;
use crate::store::{CellStore, HistoryPlan, Snapshot};
use crate::trace_sink::{TraceEvent, TraceSink};
use crate::value::{Value, ValueType};

/// The running graph — the runtime interface of §13.14 for the M0 slice:
/// [`start`](Engine::start), [`write_signal`](Engine::write_signal),
/// [`transaction`](Engine::transaction), [`commit`](Engine::commit),
/// [`acquire_snapshot`](Engine::acquire_snapshot),
/// [`teardown`](Engine::teardown).
///
/// The engine owns the loaded graph and the store, and BORROWS its sink for its
/// whole life — the trace is the runtime's observable contract (R24), so the
/// thing hearing it outlives the run that produces it.
///
/// # What each field is
///
/// - `graph` — the loaded graph: cells in rank order, the path index, and the
///   static evaluation order. The engine never re-derives any of the three; the
///   loader owns all legality (R21).
/// - `store` — the three buffers of §13.10.1/§13.10.2: live, pending, staged.
/// - `dirty` — one bit per cell, engine BOOKKEEPING rather than storage. It is
///   born at step 1, read and extended at step 3, read again at step 4½, and
///   cleared at step 6, so it never outlives a commit; nothing reads it between
///   commits.
/// - `advanced` — one bit per cell, set at step 4 for each `recurrent` whose
///   advance CHANGED its bits. It is read twice: once at step 4½ of the SAME
///   commit, where it is a deferred's provenance over the values that commit is
///   about to publish (023-73), and once at the NEXT commit's step 1b, where it
///   is consumed as a silent dirty root (§4.12's advance-root law). It is the
///   third piece of state that outlives a commit, and it outlives exactly one.
/// - `runs` — the deferred runs in flight: each one a value already computed
///   and the commit it is due to land at (R74a). This is one of the three
///   pieces of engine state that DO outlive a commit.
/// - `gates` — one `GateState` per gate, in gate rank order. The last piece
///   of surviving state, and the reason a gate needs no cell: its boolean lives
///   HERE, not in the store (C-50), so nothing about it is readable by path.
/// - `commit_index` — how many commits have completed, which is the number a
///   run's `due` is measured against.
/// - `sink` — the trace seam.
///
/// M0 is strictly sequential: there is one driving context (§13.14.2), it is
/// the caller's thread, and `&mut self` is what says so.
pub struct Engine<'s> {
    graph: LoadedGraph,
    store: CellStore,
    dirty: Vec<bool>,
    advanced: Vec<bool>,
    runs: Vec<DeferredRun>,
    gates: Vec<GateState>,
    commit_index: u64,
    sink: &'s mut dyn TraceSink,
}

/// One gate's activation, in the two forms §13.9.7 distinguishes.
///
/// `own` is what this gate's own predicate last answered. `effective` is that
/// AND every ancestor's own value along the `parent` chain — *"An instance is
/// effectively active iff its own gate AND every ancestor gate on the path from
/// the root are open"*.
///
/// Both are kept because the difference is observable in exactly one direction:
/// an inner gate's own predicate keeps evaluating and its own state keeps
/// updating under a closed ancestor (C-39/C-40), and only the EFFECTIVE value
/// traces, freezes and snaps. That is what makes an ancestor's reopen restore
/// each descendant to *its own* state without replaying anything.
#[derive(Debug, Clone, Copy)]
struct GateState {
    own: bool,
    effective: bool,
}

/// One deferred run in flight — a value the engine has already computed, and
/// the commit at which it becomes visible (R74a, under M0.md R1's sequential
/// rendering).
///
/// "In flight" is a figure of speech here and the struct is the proof: the
/// behavior ran SYNCHRONOUSLY at dispatch, over the values that commit was
/// about to publish, so nothing is executing and nothing can be waited on. What
/// is deferred is the PUBLICATION, which is why §13.10.2's publication barrier
/// is vacuous in a sequential core — the value due at a commit is on hand
/// before that commit opens.
#[derive(Debug, Clone, Copy)]
struct DeferredRun {
    /// The deferred cell the run belongs to.
    cell: CellIndex,
    /// The value its behavior produced at dispatch.
    value: Value,
    /// The commit index at which the value lands — `dispatch + depth`.
    due: u64,
}

/// The runs in flight, together with the clock their `due` is measured against.
///
/// The two travel as one because neither step that touches them can ask its
/// question with only half: step 1 asks "which of these lands NOW", and step 4½
/// asks "what does a dispatch here become due at" — both are `due` against this
/// commit's index, and a run list without the clock answers neither.
struct Runs<'a> {
    /// The hold list — [`Engine::runs`], borrowed for the commit.
    held: &'a mut Vec<DeferredRun>,
    /// The commit doing the asking.
    commit_index: u64,
}

impl<'s> Engine<'s> {
    /// Starts the runtime on a loaded graph — §13.14.1 startup steps 2–4
    /// (step 1, loading the IR, is [`load`](crate::loader::load())'s).
    ///
    /// **Step 2 — allocate cell storage.** One slot per cell in each buffer,
    /// sized once and never resized (§13.12.4's fixed-extent cost shape), plus
    /// the history region every `recurrent` needs beside them: its own ring at
    /// its declared depth, and one more per `lookback` entry. Inputs start at
    /// their converted `init`; a `derived` or `deferred` slot starts at a typed
    /// placeholder that step 3 overwrites before anything can read it — every
    /// computed cell is in `static_order`, deferreds included, and every reader
    /// follows what it reads there, so no placeholder is ever an argument.
    ///
    /// A `recurrent` starts at its DEPTH-1 FALLBACK instead, because it is the
    /// one computed cell that can be read before it has run: where the ordering
    /// pass had to break a read-cycle it released a reader ahead of the
    /// recurrent, and §13.11.3's previous-committed reading before any advance
    /// is exactly what §4.9's fallbacks answer. The released reader therefore
    /// gets a value the source spells, not a typed zero (FND-40).
    ///
    /// **Step 3 — initialize the cells.** Every computed cell evaluates ONCE,
    /// in `static_order`, reading the live buffer, and its result is seeded
    /// into both committed buffers and the staged slot. This is CONSTRUCTION,
    /// not a commit: the runtime is "constructing" through step 3 and only
    /// "live" after step 4, there is no previous snapshot to preserve and
    /// nothing to publish — so it is UNTRACED. The trace records commits (R24),
    /// and construction is not one. A trap during construction aborts exactly
    /// as a trap during a commit does, because both go through the one invoke
    /// path below.
    ///
    /// A recurrent's construction evaluation reads FALLBACKS for every history
    /// depth — no advance has happened — and then ADVANCES, because an
    /// evaluation is what shifts a ring (R77) and because the value the cell
    /// now holds must be what `.past(1, …)` reads at the first commit.
    ///
    /// A deferred's construction evaluation is a DERIVED's (C-23, 016-298):
    /// synchronous, in place, its result seeded like any other. It does not
    /// dispatch — the latency governs commits, and construction is not one — so
    /// the value a deferred serves until its first landing is the one computed
    /// here.
    ///
    /// **Step 3½ — the initial gate state.** Every predicate evaluates once,
    /// after construction, against the just-constructed values — §13.2.6 /
    /// 016-144: *"An instance whose `when` evaluates to false at the end of
    /// startup begins inactive, with its other cells holding their
    /// just-computed initial values."* So gates gate propagation and never
    /// existence even at startup: the subtree is CONSTRUCTED unconditionally
    /// and then freezes, holding values it really computed (RW-7).
    ///
    /// This is also why a closed-at-startup gate emits no `gate` line: the
    /// first commit's pre-pass recomputes the same predicate over the same
    /// values and finds no TRANSITION. A trace records changes.
    ///
    /// **Step 4 — the first commit.** The ordinary [`commit`](Engine::commit),
    /// with nothing dirty: construction has already put every cell's value in
    /// both buffers, so step 1 finds no input whose staged value differs, step 3
    /// evaluates nothing, and step 5 publishes the initial snapshot. Every trace
    /// therefore OPENS with the empty block
    /// `commit_begin` / `publish` / `commit_end` — that block IS §13.14.1 step 4
    /// made visible, and its absence would mean the runtime went live without
    /// publishing.
    pub fn start(graph: LoadedGraph, sink: &'s mut dyn TraceSink) -> Engine<'s> {
        let initial: Vec<Value> = graph
            .cells
            .iter()
            .map(|cell| match &cell.role {
                CellRole::Input { init } => *init,
                CellRole::Recurrent { fallbacks, .. } => fallbacks[0],
                CellRole::Derived { .. } | CellRole::Deferred { .. } => {
                    placeholder(cell.value_type)
                }
            })
            .collect();

        let plans = history_plans(&graph, &initial);
        let dirty = vec![false; graph.cells.len()];
        let advanced = vec![false; graph.cells.len()];
        let mut engine = Engine {
            graph,
            store: CellStore::new(initial, &plans),
            dirty,
            advanced,
            runs: Vec::new(),
            gates: Vec::new(),
            commit_index: 0,
            sink,
        };

        engine.construct();
        engine.gates = initial_gate_states(&engine.graph, &engine.store);
        engine.commit();

        engine
    }

    /// §13.14.1 step 3 — the construction pass, in `static_order` so a cell
    /// reading another computed cell reads a value already computed.
    ///
    /// The order is the SAME one step 3 of a commit walks (R80 item 4): every
    /// computed cell is a node of it and every read of one is an edge, so a
    /// reader of a recurrent constructs after that recurrent exactly as it
    /// evaluates after it. The deferreds differ only in what the two passes DO
    /// with them — evaluated here (C-23), walked past at commit time.
    ///
    /// The one place a reader does NOT follow what it reads is the deadlock
    /// RELEASE a read-cycle forces (§13.11.3 applied to the schedule): the
    /// released node reads the recurrent's depth-1 fallback, seeded by
    /// [`start`](Engine::start).
    ///
    /// The released node is not confined to the cycle. The ordering pass picks
    /// the rank-least waiting node whose every unmet predecessor is a recurrent
    /// read, so a plain reader downstream of an in-cycle recurrent — no loop of
    /// its own — is eligible, and if it is declared earlier than the true cycle
    /// member it is the one handed the fallback. One cycle can force several
    /// releases in turn. WHICH nodes are released is decided by entry rank and
    /// is FND-40's open half.
    ///
    /// A recurrent's turn has three parts, in this order: its `lookback` rings
    /// are seeded with their inputs' CONSTRUCTED values (the store's
    /// `seed_input_history` explains why that is the answer), it evaluates
    /// against fallbacks, and then it advances — self ring and input rings
    /// alike. The order matters: the seeds must be in place before the
    /// evaluation reads them, and the advance must follow the evaluation that
    /// earned it.
    fn construct(&mut self) {
        let Engine { graph, store, .. } = self;

        for &cell_index in &graph.static_order {
            let lookback = lookback_of(&graph.cells[cell_index.0]);

            for (slot, link) in lookback.iter().enumerate() {
                let constructed = store.live()[link.input.0];

                store.seed_input_history(cell_index, slot, constructed);
            }

            let value = invoke(graph, store, cell_index, ArgumentSource::Live);

            store.seed(cell_index, value);

            if matches!(graph.cells[cell_index.0].role, CellRole::Recurrent { .. }) {
                advance(graph, store, cell_index, value, ArgumentSource::Live);
            }
        }
    }

    /// Stages `value` for the input cell at `path` — §13.14.2 over §13.10.1's
    /// lazy writes.
    ///
    /// Nothing evaluates here. The write lands in the staged buffer and stays
    /// there until the next [`commit`](Engine::commit), which is where
    /// dirtiness is judged — so staged writes accumulate, the last write to a
    /// cell wins, and a sequence that ends back at the previously-committed
    /// value dirties nothing (§13.10.1's net-revert law).
    ///
    /// # Errors
    ///
    /// `unknown_input_path` for a path no cell carries, `write_to_non_input`
    /// for a derived cell, `type_mismatched_write` for a value outside the
    /// cell's declared type. A rejected write stages nothing.
    pub fn write_signal(&mut self, path: &CellPath, value: Value) -> Result<(), Diagnostic> {
        let cell = write_check(&self.graph, path, &value)?;

        self.store.stage(cell, value);

        Ok(())
    }

    /// Runs `body` against a [`Transaction`] and applies its writes atomically
    /// at close — §13.14.5's invocation form of §13.10.4, D3's host sugar over
    /// the staged buffer.
    ///
    /// Three closes, and no fourth is representable:
    ///
    /// - `body` returns `Ok` and did not abort → every accumulated pair is
    ///   applied to the staged buffer, then `Ok`.
    /// - `body` returns `Err` → NOTHING is applied; the error propagates.
    /// - `body` called [`abort`](Transaction::abort) → nothing is applied, and
    ///   the transaction closes `Ok` (§13.10.4: "the closure returns normally").
    ///
    /// "Atomic" here is all-or-nothing across those paths, not concurrency
    /// control: M0 is single-threaded, so there is no observer to see a partial
    /// application. A PANIC in the closure propagates freely — §13.10.4 is
    /// explicit that trap-track semantics apply, there is no rollback, and the
    /// staged state at the moment of abort is irrelevant because the process is
    /// terminating.
    ///
    /// # Errors
    ///
    /// Whatever `body` returns as `Err` — including a write rejected by
    /// [`Transaction::write_signal`] that the closure chose to propagate.
    pub fn transaction<F>(&mut self, body: F) -> Result<(), Diagnostic>
    where
        F: FnOnce(&mut Transaction<'_>) -> Result<(), Diagnostic>,
    {
        let mut transaction = Transaction::new(&self.graph);

        body(&mut transaction)?;

        for (cell, value) in transaction.into_writes() {
            self.store.stage(cell, value);
        }

        Ok(())
    }

    /// Settles the staged writes and publishes a new snapshot — §13.14.4, the
    /// full operation of §13.10.2.
    ///
    /// The body is the audit surface (R23): §13.10.2's six steps appear as six
    /// functions in step order, including the thin step 2, so the spec section
    /// and this function read as one list. Step 4 is no longer empty — it
    /// drains what step 3 held, which is why the two are one expression apart
    /// here. One phase sits BETWEEN two numbered steps: deferred dispatch,
    /// after step 4's advance and before step 5's flip (R74a), which is the
    /// position that keeps `publish` the last value-bearing act of a commit.
    /// Nothing else happens; the trace's `commit_begin`/`commit_end` bracket
    /// the six and the half.
    ///
    /// Step 1 has TWO halves and they are two calls (R73). Step 1a runs every
    /// gate predicate against PRIOR-COMMIT values and settles the effective
    /// transitions; step 1b then fixes the dirty roots in one rank-ordered walk
    /// with the transitions interleaved (R76). Two consequences of 1a's
    /// position ride between them, and they are the two things a transition
    /// does before anything reads: a reopened `reset_on_reopen` recurrent's
    /// history clears (C-13), and a frozen deferred's in-flight run is
    /// cancelled (§13.9.7) — so it never lands at all, rather than landing into
    /// a subtree that cannot see it.
    ///
    /// A commit with nothing dirty still runs all of them and still publishes:
    /// §13.14.4's idempotence — a fresh snapshot whose values are identical.
    ///
    /// The commit index advances last. It is the clock a deferred's `due` is
    /// measured against and nothing else reads it, so it moves once the commit
    /// it names is over.
    pub fn commit(&mut self) {
        let Engine {
            graph,
            store,
            dirty,
            advanced,
            runs,
            gates,
            commit_index,
            sink,
        } = self;

        sink.event(&TraceEvent::CommitBegin);

        let transitions = gate_pass(graph, store, gates);

        reset_reopened_histories(graph, store, &transitions);
        cancel_frozen_runs(graph, gates, runs);

        let mut in_flight = Runs {
            held: runs,
            commit_index: *commit_index,
        };

        dirty_set(
            graph,
            store,
            dirty,
            advanced,
            &mut in_flight,
            &transitions,
            *sink,
        );

        let order = evaluation_order(graph);
        let held = evaluate(graph, order, store, dirty, gates, &transitions, *sink);

        advance_recurrents(graph, store, &held, advanced);
        dispatch_deferreds(
            graph,
            store,
            dirty,
            advanced,
            &mut in_flight,
            gates,
            &transitions,
        );
        publish(store, *sink);
        clear_dirty(dirty);

        *commit_index += 1;

        sink.event(&TraceEvent::CommitEnd);
    }

    /// Whether a dispatched deferred run has not yet landed — R74b's readiness
    /// surface, and the whole of it.
    ///
    /// 027-128 makes the staged-work question "the same surface as queued
    /// writes", and queued writes are QUERIED, never returned from `commit`. So
    /// [`commit`](Engine::commit)'s signature is untouched and this is the one
    /// verb M1 adds: an event-driven host that would otherwise go idle asks
    /// here whether another commit still has something to publish.
    ///
    /// It answers about PUBLICATION, not about work: in the sequential
    /// rendering every held run's behavior has already executed (R1), so a
    /// `true` means "a value is waiting for its commit", never "a computation
    /// is running".
    #[must_use]
    pub fn has_staged_work(&self) -> bool {
        !self.runs.is_empty()
    }

    /// A consistent view of the latest committed state — §13.14.6, rendered as
    /// R22's borrow view.
    ///
    /// The returned [`Snapshot`] borrows the engine, so the compiler — not the
    /// runtime — is what forbids committing while a read view is alive.
    pub fn acquire_snapshot(&self) -> Snapshot<'_> {
        Snapshot::new(&self.graph.index, self.store.live())
    }

    /// Shuts the graph down — §13.14.10's `teardown`, running §13.14.1's
    /// shutdown sequence.
    ///
    /// In the implemented slice that sequence collapses to almost nothing:
    /// there are no effects to deliver reconciler `teardown` to, no in-flight
    /// commit to drain (commits are synchronous and this call needs `self` by
    /// value, so none can be running), no wires or pools, and no string pool.
    /// What remains is "release cell storage" — which is what consuming `self`
    /// does. The verb exists because it is mandatory core (§13.14.11) and
    /// because M2+ grows into it; writing it as an empty body now is what keeps
    /// the host's shutdown call site unchanged when it stops being empty.
    ///
    /// **Undelivered deferred runs are DROPPED, and that is the whole story**
    /// (RW-8). A held run is a value in a `Vec` — its behavior already ran, at
    /// dispatch — so there is nothing to cancel and nothing to await; the runs
    /// die with the engine and their values are never published. The corpus
    /// never says what whole-runtime teardown owes an undelivered run (C-30,
    /// non-blocking), and this body's emptiness is that silence honestly
    /// rendered rather than a decision hidden in it.
    ///
    /// Consuming `self` also ends the sink borrow, which is how a host gets its
    /// trace sink back to inspect.
    pub fn teardown(self) {}
}

/// §13.10.2 **step 1a — the gate predicate pre-pass** (R73, fork 3 as ruled).
///
/// Evaluates EVERY gate's predicate, in gate rank order, against the LIVE
/// buffer — the values as they stand at commit start — then recomposes each
/// gate's effective activation and reports which ones CHANGED.
///
/// **Prior-commit values, taken literally.** 023-57's unqualified sentence —
/// *"the gate predicate itself keeps reading prior-commit values"* — is the
/// whole of the read law here, and R73 rules it literally: a staged write
/// publishes at commit N, so the predicate sees it at commit N+1, and the flip
/// and its same-commit snap happen at N+1. That one-commit lag is not a
/// scheduling artifact to be tuned away; it is what keeps §13.10.2's own
/// invariant — *"No new dirty bits are added during the rest of the commit
/// cycle"* — true without reinterpretation, because the transition set is
/// complete before step 1b begins.
///
/// **Every predicate, every commit** (C-50). No dirtiness test gates the
/// pre-pass. A predicate is a pure function over live values, so evaluating an
/// unchanged one is unobservable — and only the TRANSITION it computes is used.
/// Because a gate's boolean occupies no graph cell, there is also no slot whose
/// dirtiness could have gated it.
///
/// **Own state updates under a closed ancestor** (C-39/C-40). Each gate's own
/// value is written whatever its ancestors say; what the return value reports —
/// and therefore what traces, freezes and snaps — is the EFFECTIVE change. That
/// asymmetry is what makes an ancestor's reopen restore each descendant to its
/// own state with no replay: the descendant's own bool was never frozen, only
/// its effect.
///
/// The result is indexed by gate rank: `Some(true)` for an effective open this
/// commit, `Some(false)` for an effective close, `None` for no change.
fn gate_pass(graph: &LoadedGraph, store: &CellStore, gates: &mut [GateState]) -> Vec<Option<bool>> {
    let own: Vec<bool> = (0..graph.gates.len())
        .map(|rank| predicate(graph, store, GateIndex(rank)))
        .collect();
    let mut transitions = Vec::with_capacity(gates.len());

    for (rank, state) in gates.iter_mut().enumerate() {
        let effective = effective_of(graph, &own, GateIndex(rank));

        transitions.push((effective != state.effective).then_some(effective));

        state.own = own[rank];
        state.effective = effective;
    }

    transitions
}

/// The gate states a graph starts in — §13.14.1's construction, after every
/// cell has computed its initial value (RW-7).
///
/// It is [`gate_pass`] with nothing to compare against: the same predicates
/// over the same buffer, recorded rather than differenced. A gate that
/// evaluates false here begins CLOSED, and the subtree it guards begins frozen
/// holding the values construction just computed.
fn initial_gate_states(graph: &LoadedGraph, store: &CellStore) -> Vec<GateState> {
    let own: Vec<bool> = (0..graph.gates.len())
        .map(|rank| predicate(graph, store, GateIndex(rank)))
        .collect();

    (0..graph.gates.len())
        .map(|rank| GateState {
            own: own[rank],
            effective: effective_of(graph, &own, GateIndex(rank)),
        })
        .collect()
}

/// One gate's predicate, run over the live buffer.
///
/// The arguments are its `inputs` in list order and nothing else: a predicate
/// has no history and no self-reference, so its ABI is a derived's. The result
/// must be `bool` — §13.9.4: *"The predicate must have type `bool`. A non-bool
/// predicate is a compile error."* The loader cannot check it, for the same
/// reason it cannot check a behavior's arity: a [`Behavior`] is an opaque
/// closure across the provider seam (D5). So a wrong-typed predicate traps here
/// naming its gate, exactly as a wrong-typed cell result does (D8).
fn predicate(graph: &LoadedGraph, store: &CellStore, gate: GateIndex) -> bool {
    let entry = &graph.gates[gate.0];
    let arguments: Vec<Value> = entry
        .inputs
        .iter()
        .map(|&input| store.live()[input.0])
        .collect();

    match call(&entry.predicate, &arguments, &entry.path) {
        Value::Bool(open) => open,
        other => panic!(
            "predicate for gate `{}` produced a `{}`; a predicate is `bool`",
            entry.path.render(),
            ValueType::of(&other).as_str()
        ),
    }
}

/// Effective activation, composed from the own values: §13.9.7's *"its own gate
/// AND every ancestor gate on the path from the root"*.
///
/// It reads `own` and never the stored effective values, so a whole pre-pass
/// can compute every gate's new effective state from one snapshot of the
/// predicates — which is what makes the order gates are visited in
/// unobservable. The loader proved the chain acyclic (`gate_parent_cycle`), so
/// the climb terminates.
fn effective_of(graph: &LoadedGraph, own: &[bool], gate: GateIndex) -> bool {
    let mut at = Some(gate);

    while let Some(current) = at {
        if !own[current.0] {
            return false;
        }

        at = graph.gates[current.0].parent;
    }

    true
}

/// `@reset_on_reopen`, applied at its ruled position — step 1a, BEFORE step 3's
/// reads (C-13).
///
/// A recurrent carrying the flag whose gate effectively REOPENED this commit
/// drops its whole history, self ring and `lookback` rings alike, so the first
/// post-gap evaluation reads FALLBACKS rather than values from the other side
/// of the gap (§13.2.4). The position is what makes that observable in the same
/// commit as the reopen: clearing after step 3 would let one stale evaluation
/// through first.
///
/// The reset fires on the EFFECTIVE reopen edge — the same edge the snap and
/// the trace line use — so an ancestor-driven reopen clears the history of a
/// descendant whose own gate never changed (C-41).
fn reset_reopened_histories(
    graph: &LoadedGraph,
    store: &mut CellStore,
    transitions: &[Option<bool>],
) {
    for (rank, cell) in graph.cells.iter().enumerate() {
        let resets = matches!(
            cell.role,
            CellRole::Recurrent {
                reset_on_reopen: true,
                ..
            }
        );

        if resets && snapped(transitions, cell) {
            store.clear_history(CellIndex(rank));
        }
    }
}

/// Gate-close CANCELS in-flight deferred runs — §13.9.7: *"Deferreds do not
/// dispatch; gate-close cancels any in-flight runs — no publication, no
/// barrier — and the frozen cell serves its last-published value."*
///
/// Cancelling is dropping the hold, and dropping it is what distinguishes
/// cancellation from postponement: a merely-skipped landing would still be
/// waiting when the gate reopened, and the value it carried would arrive from a
/// commit on the far side of the gap. It is written as a filter over every
/// currently-frozen cell rather than as a reaction to the close transition,
/// which makes it idempotent — a run cannot survive by being dispatched into a
/// subtree that closed before it landed.
///
/// The reopen half is [`dispatch_deferreds`](dispatch_deferreds)': a fresh run
/// is dispatched at the reopening commit's end and lands `depth` commits later.
fn cancel_frozen_runs(graph: &LoadedGraph, gates: &[GateState], runs: &mut Vec<DeferredRun>) {
    runs.retain(|run| active(gates, &graph.cells[run.cell.0]));
}

/// Whether `cell` is EFFECTIVELY active — the eligibility test §13.9.7's freeze
/// law rests on.
///
/// A cell's own gate has already ANDed its ancestors (see [`gate_pass`]), so
/// one lookup answers the whole activation question. An ungated cell is always
/// active, which is what makes a graph with no gates behave exactly as M0's
/// did.
fn active(gates: &[GateState], cell: &LoadedCell) -> bool {
    match cell.gate {
        Some(gate) => gates[gate.0].effective,
        None => true,
    }
}

/// Whether `cell`'s gate effectively OPENED this commit — the gate-open snap's
/// trigger (022-59).
///
/// *"The gate-open re-evaluation snap is scheduled within the same commit that
/// flips the predicate false→true — not the next commit."* Under R73's pre-pass
/// the flip is known before step 1b ends, so the snap needs no second pass: it
/// is an extra way for a cell to be stale in step 3.
fn snapped(transitions: &[Option<bool>], cell: &LoadedCell) -> bool {
    match cell.gate {
        Some(gate) => transitions[gate.0] == Some(true),
        None => false,
    }
}

/// §13.10.2 **step 1 — compute the dirty set.**
///
/// Refills the pending buffer from live, then walks every cell in RANK order
/// and dirties the cells a ROOT reaches, copying each dirtied value into
/// pending and emitting its `dirty` line.
///
/// **One walk, all root sources interleaved by rank** (R76, fork 6). FOUR
/// things can reach this commit from outside its evaluation: a host write
/// staged against an `input`, a deferred run landing at this exact commit, a
/// gate transition step 1a settled, and a `recurrent` that ADVANCED at the
/// previous commit. They are folded in by ONE pass over the
/// graph-section ENTRY rank — cells and gates in the order the text declared
/// them (C-45) — so the trace's root lines are a single rank sequence whatever
/// produced them, and a reader cannot tell a landing from a write. That is
/// deliberate: a downstream consumer cannot tell either, and the trace is the
/// consumer's view (R24, R75). A landing therefore reuses
/// `dirty <path> <old> -> <new>`, because it IS a dirty root by the corpus's
/// own words, and no `land` event exists to record which mechanism produced the
/// value. A transition is the one root that earns a line of its own, because it
/// carries no value and no existing event can express it.
///
/// **The advance root is SILENT** (§4.12). A recurrent whose advance changed its
/// bits at commit N is a dirty root here at N+1 — its consumers read the value
/// the advance published, one commit behind it, which is the delay chain — and
/// it renders NOTHING: the change was already observable at N as the advance's
/// own `eval` line, so a line here would report it twice (R24: the trace carries
/// the observable, never the mechanism). It also stages no value, because there
/// is none to stage — step 4 wrote the advanced value to pending at N and step 5
/// published it, so `begin_pending`'s copy has already put it in this commit's
/// pending slot. The bit alone is the whole root.
///
/// The two sequences are merged rather than concatenated, and the merge is a
/// two-pointer walk because both are already sorted: the loader assigns entry
/// ranks in one increasing pass, so cells ascend by [`CellIndex`] and gates
/// ascend by gate rank.
///
/// Three laws are carried here, and each is a line of code:
///
/// - The comparison is [`Value::bits_eq`] — bitwise on the stored
///   representation, not IEEE equality (R65): an unchanged NaN bit pattern is
///   CLEAN, and a `0.0` overwritten with `-0.0` is DIRTY. It governs a LANDING
///   exactly as it governs a write: a run that lands the value the cell already
///   holds produces no `dirty` line and propagates nothing, while still being
///   consumed — landing and propagating are two questions.
/// - Reverted and equal writes produce no dirt: the staged value is compared
///   against the previous COMMIT, never against the intermediate writes, which
///   is §13.10.1's net-revert law with nothing extra needed to implement it.
/// - The walk is rank order (R64), never write order, so the trace is invariant
///   under permutation of same-commit writes to distinct cells.
///
/// **No new dirty bits after this step** — read precisely. This step fixes the
/// dirty ROOTS; §13.10.2's "dirty propagation extends to all derived cells
/// transitively dependent on dirty roots" is not a separate marking pass here,
/// because a derived's dirtiness is its own value-change and is only knowable
/// once it has evaluated. Step 3 therefore sets a derived's bit as an
/// evaluation OUTCOME, and no root joins the set after this function returns.
fn dirty_set(
    graph: &LoadedGraph,
    store: &mut CellStore,
    dirty: &mut [bool],
    advanced: &mut [bool],
    runs: &mut Runs<'_>,
    transitions: &[Option<bool>],
    sink: &mut dyn TraceSink,
) {
    store.begin_pending();

    let mut gate = 0;

    for (rank, cell) in graph.cells.iter().enumerate() {
        while gate < graph.gates.len() && graph.gates[gate].entry_rank < cell.entry_rank {
            gate_transition(graph, transitions, gate, sink);
            gate += 1;
        }

        if advanced[rank] {
            advanced[rank] = false;
            dirty[rank] = true;
        }

        let index = CellIndex(rank);
        let arriving = root_value(store, runs, &cell.role, index);
        let committed = store.live()[rank];
        let changed = arriving.filter(|value| !value.bits_eq(&committed));

        if let Some(value) = changed {
            dirty[rank] = true;

            store.set_pending(index, value);
            sink.event(&TraceEvent::Dirty {
                path: cell.path.clone(),
                old: committed,
                new: value,
            });
        }
    }

    while gate < graph.gates.len() {
        gate_transition(graph, transitions, gate, sink);
        gate += 1;
    }
}

/// One gate's line in step 1b's walk, emitted only when its EFFECTIVE
/// activation changed (R75, C-39/C-40).
///
/// A gate that did not transition contributes nothing — not a suppressed line,
/// simply no event, exactly as a clean input contributes no `dirty` line.
fn gate_transition(
    graph: &LoadedGraph,
    transitions: &[Option<bool>],
    gate: usize,
    sink: &mut dyn TraceSink,
) {
    let Some(open) = transitions[gate] else {
        return;
    };

    sink.event(&TraceEvent::Gate {
        path: graph.gates[gate].path.clone(),
        open,
    });
}

/// The value reaching `cell` from OUTSIDE this commit's evaluation, or `None`
/// when nothing does — step 1's two root sources, asked as one question.
///
/// An `input` always answers, with its staged slot: whether that value is a
/// CHANGE is the caller's comparison, which is what makes a re-written or
/// net-reverted value clean. A `deferred` answers only at the commit one of its
/// runs is due, and answering CONSUMES the run — a landing happens exactly
/// once, whether or not its value turns out to propagate.
///
/// The other two kinds are computed by step 3 and reached by no root.
fn root_value(
    store: &CellStore,
    runs: &mut Runs<'_>,
    role: &CellRole,
    cell: CellIndex,
) -> Option<Value> {
    match role {
        CellRole::Input { .. } => Some(store.staged(cell)),
        CellRole::Deferred { .. } => landed(runs, cell),
        CellRole::Derived { .. } | CellRole::Recurrent { .. } => None,
    }
}

/// Takes `cell`'s run due at this commit out of the hold list.
///
/// At most one can exist: a deferred dispatches at most once per commit and
/// every run of one cell carries the same `depth`, so two runs of a cell never
/// share a due commit. The search is linear over the runs in flight, which is
/// bounded by the deepest `depth` in the graph.
fn landed(runs: &mut Runs<'_>, cell: CellIndex) -> Option<Value> {
    let at = runs
        .held
        .iter()
        .position(|run| run.cell == cell && run.due == runs.commit_index)?;

    Some(runs.held.remove(at).value)
}

/// §13.10.2 **step 2 — compute the evaluation order.**
///
/// The per-commit order IS the load-time static order, so this function returns
/// it and does nothing else.
///
/// It is thin on purpose, and kept as a step on purpose (R23). The spec asks
/// for a topological sort of the per-commit DAG; the loader already sorted the
/// whole graph into the rank-lexicographic minimum topological order (whether
/// that is §13.10.3's composed order is FND-25's open question), and every
/// per-commit DAG is a SUBGRAPH of that graph — so the static order
/// restricted to any dirty subset is a valid topological order of it, and
/// re-sorting per commit would only re-derive an answer already on hand. Step 3
/// walks this order, skips the deferreds and skips what is not dirty, which is
/// the same schedule the spec's per-commit sort would produce.
///
/// The step keeps its own name and anchor so the six-step correspondence stays
/// literal, and so the milestone that adds nodes the static order cannot
/// describe — gate-opened subtrees, `repeat` re-keying — has an obvious place
/// to grow.
fn evaluation_order(graph: &LoadedGraph) -> &[CellIndex] {
    &graph.static_order
}

/// §13.10.2 **step 3 — evaluate in topological order.**
///
/// Walks `order` and evaluates a derived if and only if ANY of its direct
/// dependencies is dirty, which is value-change semantics operationalized: a
/// clean dependency set means nothing this cell reads changed, so its value
/// cannot have.
///
/// **A `deferred` is walked past.** §13.10.2: *"The cell's own body is never a
/// step-2 DAG node."* It holds a place in `order` because construction evaluates
/// it (C-23) and because readers of it must follow it there, and this `continue`
/// is where the commit half of that one order says no — its value arrives as a
/// landing at step 1 and its body dispatches at step 4½.
///
/// **Where arguments are read from.** Every argument comes from the PENDING
/// buffer, and that single rule IS §13.10.2's three-part read law:
///
/// - a signal read resolves to the STAGED value, because step 1 copied every
///   dirtied input's staged value into pending;
/// - a derived read resolves to the THIS-COMMIT value when that derived
///   evaluated earlier in this walk, because its result was written to pending;
/// - and an untouched cell's pending slot still holds its
///   PREVIOUSLY-COMMITTED value, because `begin_pending` copied live into it.
///
/// **What a recurrent adds.** It is stale on exactly the same test — a VALUE
/// input is dirty — because its history reads are not triggers (§4.2). Its
/// arguments come from the history rings as well as pending, in R71's ABI
/// order. And its result is NOT written to pending: it is HELD and returned to
/// step 4. That is the whole of lockstep (§13.2.4.1): while the result sits in
/// the hold list, `store.pending()` still answers with the cell's
/// previous-committed value, so nothing evaluating later in this pass — another
/// recurrent, a derived reading it — can see a just-advanced value.
///
/// **ACTIVATION FIRST, staleness second** (RW-9, §13.9.7's Model B). A cell
/// that is not effectively active is ineligible this commit even with dirty
/// dependencies, and the order of the two tests is what the whole frozen-value
/// law rests on: a frozen cell is one that NOTHING WRITES to pending, and
/// `begin_pending`'s wholesale copy already left its previous committed value
/// there. Freezing is therefore free, and that is an intended property of the
/// store's shape rather than an accident of it — *"Deriveds do not recompute.
/// They hold their last committed value."*
///
/// **The gate-open snap is a second way to be stale.** On the commit a gate
/// effectively opens, its guarded DERIVEDS evaluate — forced stale, reading
/// current values (022-59). Recurrents are pointedly excluded: §13.9.7 says a
/// trigger lost during a gated period is *lost*, that *"the runtime does not
/// queue triggers, and gate-open does not replay them"*, so snapping one would
/// manufacture the advance the corpus says never happens. A frozen recurrent
/// resumes at its pre-gap value and waits for a NEW trigger.
///
/// **The result.** It is type-checked and invoked through the one invoke path
/// below, its `eval` line is emitted (only for evaluations that ACTUALLY
/// happen — a skipped node emits nothing, per R24's no-suppressed-evals rule),
/// and it is written to pending (or held). A DERIVED's dirty bit is then set to
/// whether the value CHANGED, and that is where value-change gating on outputs
/// lives (§13.2.4.4, R65): a cell that re-evaluates to a bit-identical value
/// marks itself clean, so the deriveds downstream of it see no dirty dependency
/// and do not evaluate at all.
///
/// **A RECURRENT sets no dirty bit here** (§4.12). Its result is not visible
/// this commit — it is held aside and published at step 4 — so marking it dirty
/// now would let a reader evaluate at this commit over the PRE-advance value,
/// and whether that happened would depend on which side of the recurrent the
/// reader was declared. The advance is a dirty root at the NEXT commit instead,
/// where the published value is what a reader gets, under either declaration
/// order. Its bit is also not CLEARED here: at the commit its own advance root
/// set it, it stays set through step 3 so its consumers see a changed
/// dependency. Value-change gating still governs it, one commit later and from
/// step 4's comparison — and the ring still shifts whatever the comparison says
/// (R77).
fn evaluate(
    graph: &LoadedGraph,
    order: &[CellIndex],
    store: &mut CellStore,
    dirty: &mut [bool],
    gates: &[GateState],
    transitions: &[Option<bool>],
    sink: &mut dyn TraceSink,
) -> Vec<(CellIndex, Value)> {
    let mut held = Vec::new();

    for &cell_index in order {
        let cell = &graph.cells[cell_index.0];

        if matches!(cell.role, CellRole::Deferred { .. }) {
            continue;
        }

        if !active(gates, cell) {
            continue;
        }

        let snap = matches!(cell.role, CellRole::Derived { .. }) && snapped(transitions, cell);
        let stale = snap || cell.role.inputs().iter().any(|input| dirty[input.0]);

        if stale {
            let value = invoke(graph, store, cell_index, ArgumentSource::Pending);

            sink.event(&TraceEvent::Eval {
                path: cell.path.clone(),
                value,
            });

            match cell.role {
                CellRole::Recurrent { .. } => held.push((cell_index, value)),
                CellRole::Input { .. } | CellRole::Derived { .. } | CellRole::Deferred { .. } => {
                    dirty[cell_index.0] = !value.bits_eq(&store.live()[cell_index.0]);

                    store.set_pending(cell_index, value);
                }
            }
        }
    }

    held
}

/// §13.10.2 **step 4 — advance recurrents.**
///
/// Drains step 3's hold list: each held value shifts its cell's ring by one,
/// shifts every `lookback` input's ring by one over that input's this-commit
/// value, and lands in pending, where step 5's flip publishes it. After this
/// step "recurrent reads return their newly-advanced values" — which is why it
/// sits after evaluation and before publication, and why that position is what
/// makes lockstep true rather than merely stated.
///
/// **Once per evaluation, and never otherwise** (R72 + R77). A commit in which
/// a recurrent did not evaluate is not in the hold list at all, so it consumes
/// no slot; a commit in which it evaluated to the value it already held IS in
/// the list, so it consumes one. `past(k)` therefore counts advances, never
/// commits.
///
/// **The drain order is unobservable, and the two loops are what make it so.**
/// The rings advance first, over the values step 3 evaluated against; only then
/// do the held values land in pending. One loop would not do: a recurrent with
/// a `lookback` on ANOTHER recurrent reads that input's pending slot as it
/// advances, and a single pass would hand it a just-advanced value whenever the
/// other cell happened to drain first — order leaking into a ring, and lockstep
/// broken at exactly the place the law is about. Split this way, every advance
/// records the value its cell SAW, whatever the order (§4.4).
///
/// **The bits-changed advances are RECORDED here** (§4.12). The comparison is
/// against the still-live buffer — the value the cell held when this commit
/// opened — and is [`Value::bits_eq`] like every other change test (R65), so an
/// advance to a bit-identical value is no root at all. Each recorded bit becomes
/// a silent dirty root at the NEXT commit's step 1b, which is what makes a
/// step-3 consumer of a recurrent evaluate one commit behind it — and it is read
/// once more before that, at THIS commit's step 4½, where it is a deferred's
/// provenance over the value about to be published (023-73). The construction
/// advance is deliberately not recorded: [`construct`](Engine::construct) calls
/// [`advance`] directly, because construction publishes its values as the
/// initial snapshot and there is nothing for a consumer to catch up to.
fn advance_recurrents(
    graph: &LoadedGraph,
    store: &mut CellStore,
    held: &[(CellIndex, Value)],
    advanced: &mut [bool],
) {
    for &(cell_index, value) in held {
        advance(graph, store, cell_index, value, ArgumentSource::Pending);
    }

    for &(cell_index, value) in held {
        let changed = !value.bits_eq(&store.live()[cell_index.0]);

        advanced[cell_index.0] = changed;

        store.set_pending(cell_index, value);
    }
}

/// One recurrent's ring advance — step 4's body, and construction's echo of it.
///
/// `source` is where the `lookback` inputs' current values are read from: the
/// live buffer at construction, pending during a commit. Both are "the value
/// this evaluation saw", which is what makes `input_past(1)` mean "the input's
/// value at my previous advance".
fn advance(
    graph: &LoadedGraph,
    store: &mut CellStore,
    cell_index: CellIndex,
    value: Value,
    source: ArgumentSource,
) {
    store.advance_history(cell_index, value);

    for (slot, link) in lookback_of(&graph.cells[cell_index.0]).iter().enumerate() {
        let current = match source {
            ArgumentSource::Live => store.live()[link.input.0],
            ArgumentSource::Pending => store.pending(link.input),
        };

        store.advance_input_history(cell_index, slot, current);
    }
}

/// A cell's `lookback` links — empty for every kind but `recurrent`.
fn lookback_of(cell: &LoadedCell) -> &[LookbackLink] {
    match &cell.role {
        CellRole::Input { .. } | CellRole::Derived { .. } | CellRole::Deferred { .. } => &[],
        CellRole::Recurrent { lookback, .. } => lookback,
    }
}

/// §13.10.2 **step 4½ — dispatch the deferreds.**
///
/// Walks the deferreds in RANK order (C-21) and dispatches each one whose
/// PROVENANCE CHANGED this commit (C-28), which composes with R65's bitwise
/// gating for free, because every bit read here is already bitwise. An
/// unchanged provenance dispatches nothing at all.
///
/// **The position is the ruling** (R74a, fork 4a): after step 4's advance, so
/// the values read here are exactly the ones this commit is about to publish —
/// the recurrent advances included — and before step 5's flip, so `publish`
/// stays the last value-bearing act of a commit block. Both halves of a
/// deferred's life therefore sit at opposite ends of one commit: it lands at
/// step 1 and it dispatches here.
///
/// **A RECURRENT input is asked a different question, and the position is why**
/// (023-73 over §4.12). Every other input answers with its `dirty` bit, which
/// says "this cell's published value moved this commit". A recurrent's dirty
/// bit does not say that: §4.12 sets it at the commit AFTER the advance, as the
/// silent root that wakes its step-3 consumers. Reading it here would ask about
/// the previous commit's advance while the arguments come from THIS commit's,
/// and the two disagree in both directions — an advance at N would dispatch
/// nothing at N and, at N+1, dispatch over the value N+1 advanced to, so v(N)
/// would never be captured at all; a commit that rooted but did not advance
/// would dispatch again over a value that had not moved. So the question asked
/// of a recurrent is its `advanced` bit — "this cell advanced to new bits in
/// THIS commit", the very bits step 4 just wrote to pending and step 5 is about
/// to publish. 023-73's own words are that test: *"the run snapshots exactly
/// the values the commit is about to publish, recurrent advances included"*.
///
/// The consequence is that a deferred sees an advance one commit EARLIER than a
/// derived does, and that is the same law rather than an exception to it. A
/// derived reads in step 3, which runs before the advance, so §4.12 delays it a
/// commit to keep the schedule out of declaration order's hands; a deferred
/// reads at a fixed position after step 4, which no declaration order can move,
/// so it needs no delay and gets none. Each advance reaches a deferred exactly
/// once, in advance order.
///
/// **Dispatch is synchronous** under M0.md R1's rendering: the behavior runs
/// now, over the pending values, and what is held is a finished VALUE plus the
/// commit it is due at. Nothing is scheduled, nothing waits, and §13.10.2's
/// publication barrier is vacuous — a run cannot be "unfinished" at its due
/// commit when it finished `depth` commits earlier.
///
/// **Dispatch produces no trace line.** It publishes no value, so under R24's
/// minimal-semantic rule there is nothing observable to record; the landing is
/// where the value becomes visible and where the `dirty` line appears (R75).
/// A chained deferred needs nothing extra: an upstream landing set a dirty bit
/// at step 1, so the downstream one dispatches here, at the same commit, and
/// latencies compose additively (C-29).
///
/// **Under a gate, both halves of the law land here.** A FROZEN deferred does
/// not dispatch — activation is checked before provenance, exactly as step 3
/// checks it before staleness — and its held run was already cancelled at step
/// 1a. On the commit its gate effectively OPENS it dispatches a FRESH run,
/// landing `depth` commits later; §13.9.7 makes the gate-open snap a dispatch
/// for a deferred rather than the synchronous re-evaluation a derived gets,
/// which is the one place the snap means two different things.
fn dispatch_deferreds(
    graph: &LoadedGraph,
    store: &CellStore,
    dirty: &[bool],
    advanced: &[bool],
    runs: &mut Runs<'_>,
    gates: &[GateState],
    transitions: &[Option<bool>],
) {
    for (rank, cell) in graph.cells.iter().enumerate() {
        let CellRole::Deferred { depth, .. } = cell.role else {
            continue;
        };

        if !active(gates, cell) {
            continue;
        }

        let cell_index = CellIndex(rank);
        let provenance = snapped(transitions, cell)
            || cell
                .role
                .inputs()
                .iter()
                .any(|input| moved(graph, dirty, advanced, *input));

        if provenance {
            let value = invoke(graph, store, cell_index, ArgumentSource::Pending);

            runs.held.push(DeferredRun {
                cell: cell_index,
                value,
                due: runs.commit_index + depth as u64,
            });
        }
    }
}

/// Whether `input`'s value MOVED in the commit step 4½ is standing in — the one
/// question a deferred's provenance asks, put to the bit that answers it for the
/// kind.
///
/// A `recurrent` answers with its `advanced` bit and everything else with its
/// `dirty` bit, and the two are not interchangeable: §4.12 makes a recurrent's
/// dirty bit a NEXT-commit root, so at this position it names the previous
/// commit's advance while the arguments come from this one's. `advanced` is the
/// bit step 4 just set, over the same comparison every other change test uses
/// (R65, bitwise), so both branches mean one thing.
fn moved(graph: &LoadedGraph, dirty: &[bool], advanced: &[bool], input: CellIndex) -> bool {
    match graph.cells[input.0].role {
        CellRole::Recurrent { .. } => advanced[input.0],
        CellRole::Input { .. } | CellRole::Derived { .. } | CellRole::Deferred { .. } => {
            dirty[input.0]
        }
    }
}

/// §13.10.2 **step 5 — publish the snapshot.**
///
/// Flips the live index. That flip is the whole of publication: constant time,
/// no copying, no per-cell work, whatever the graph's size (025-61's cost
/// shape, §13.12.4). Before it, every reader saw the previous commit's values;
/// after it, every reader sees this commit's — there is no interval in which a
/// reader sees a mixture, which is what "atomically makes the just-committed
/// values the visible snapshot" asks for.
fn publish(store: &mut CellStore, sink: &mut dyn TraceSink) {
    store.flip();
    sink.event(&TraceEvent::Publish);
}

/// §13.10.2 **step 6 — clear dirty bits.** Ready for the next commit.
fn clear_dirty(dirty: &mut [bool]) {
    dirty.fill(false);
}

/// Which committed face [`invoke`] reads its arguments from.
enum ArgumentSource {
    /// Construction (§13.14.1 step 3): the live buffer, where each derived's
    /// seeded value has already landed.
    Live,
    /// A commit (§13.10.2 step 3): the pending buffer, which is where the read
    /// law resolves.
    Pending,
}

/// The ONE path through which a behavior is ever called — construction's
/// evaluation and step 3's alike.
///
/// It gathers the cell's arguments from the chosen buffer, calls the behavior,
/// and type-checks the result. Both callers share it so that "a trap during
/// construction aborts exactly as a trap during a commit does" is true by
/// construction rather than by two matching implementations.
///
/// # The argument order IS the ABI (R71, §4.9)
///
/// `inputs` in list order, then the cell's own `past1 … pastDepth` ascending,
/// then per `lookback` entry in list order its `<input>_past1 … <input>_pastK`
/// ascending. A derived's gather is the first clause alone, unchanged from M0,
/// and a DEFERRED's is a derived's: a latency is not a memory, so a deferred
/// has no history clause and no history arguments.
/// Arity is `inputs.len() + depth + sum(lookback k)` and is not checkable at
/// load — a [`Behavior`](crate::provider::Behavior) is an opaque closure — so a
/// stub whose parameter list disagrees traps in [`call`], naming its cell (D8).
///
/// # A wrong-typed result is a panic, not a diagnostic
///
/// The loader owns legality (R21) and it cannot check this one: a behavior's
/// result type is only knowable when the behavior runs. A stub returning the
/// wrong type is a harness-author bug of exactly the kind D8's trap shape is
/// for, so it aborts with the cell path, the declared type, and what arrived.
fn invoke(
    graph: &LoadedGraph,
    store: &CellStore,
    cell_index: CellIndex,
    source: ArgumentSource,
) -> Value {
    let cell = &graph.cells[cell_index.0];

    let behavior = match &cell.role {
        CellRole::Input { .. } => panic!(
            "only computed cells evaluate; `{}` is an input",
            cell.path.render()
        ),
        CellRole::Derived { behavior, .. }
        | CellRole::Recurrent { behavior, .. }
        | CellRole::Deferred { behavior, .. } => behavior,
    };

    let mut arguments: Vec<Value> = cell
        .role
        .inputs()
        .iter()
        .map(|&input| match source {
            ArgumentSource::Live => store.live()[input.0],
            ArgumentSource::Pending => store.pending(input),
        })
        .collect();

    if let CellRole::Recurrent {
        depth, lookback, ..
    } = &cell.role
    {
        for k in 1..=*depth {
            arguments.push(store.past(cell_index, k));
        }

        for (slot, link) in lookback.iter().enumerate() {
            for k in 1..=link.k {
                arguments.push(store.input_past(cell_index, slot, k));
            }
        }
    }

    checked(cell, call(behavior, &arguments, &cell.path))
}

/// The one place a behavior is actually CALLED — a cell's and a gate
/// predicate's alike.
///
/// # The `catch_unwind` is a STAMP, not a boundary
///
/// §13.13.1 is explicit: the runtime does not isolate traps, there is no
/// errored-cell sentinel, no `catch_unwind` boundary, and no continuation past
/// a trap. This does not create one. It catches the unwind and IMMEDIATELY
/// re-panics — nothing runs in between, no value is invented, no state is
/// repaired, and control never returns to the commit. What it buys is the
/// trapping construct's PATH in the message, which is the M0 trap shape the
/// owner ruled (D8: a harness-visible panic carrying the cell path). The
/// tension with §13.13.1's wording is real and is filed as FND-24
/// (`docs/implementation/FINDINGS.md`), awaiting the owner's ruling; the
/// reading applied here is that the prohibition is on ISOLATION — surviving a
/// trap — not on observing one in flight to name it.
fn call(behavior: &Behavior, arguments: &[Value], path: &CellPath) -> Value {
    let outcome = catch_unwind(AssertUnwindSafe(|| behavior(arguments)));

    match outcome {
        Ok(value) => value,
        Err(payload) => trap(path, payload),
    }
}

/// The value a behavior produced, once it is known to carry the cell's declared
/// type.
fn checked(cell: &LoadedCell, value: Value) -> Value {
    assert!(
        cell.value_type.matches(&value),
        "behavior for `{}` produced a `{}`; the cell is declared `{}`",
        cell.path.render(),
        ValueType::of(&value).as_str(),
        cell.value_type.as_str()
    );

    value
}

/// Re-panics a caught behavior trap with the trapping cell's path (D8).
///
/// A `&str` or `String` payload — everything `panic!` produces — is carried
/// through; any other payload is dropped and the path stands alone, because a
/// payload the runtime cannot render is worse than no payload at all.
fn trap(path: &CellPath, payload: Box<dyn Any + Send>) -> ! {
    let message = payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned());

    match message {
        Some(text) => panic!("behavior trap at {}: {text}", path.render()),
        None => panic!("behavior trap at {}", path.render()),
    }
}

/// The store's history allocation request, one plan per cell in rank order
/// (§13.14.1 step 2).
///
/// A recurrent asks for its own ring — depth and per-`k` fallbacks, both
/// already settled by the loader — plus one ring per `lookback` entry, in list
/// order. An input's pre-history has no spelling in the IR (the `fallbacks`
/// clause seeds self-history alone), so the plan opens those rings at the input
/// cell's ALLOCATION value and construction re-seeds them with its constructed
/// one; the two differ only when the input is itself computed.
fn history_plans(graph: &LoadedGraph, initial: &[Value]) -> Vec<HistoryPlan> {
    graph
        .cells
        .iter()
        .map(|cell| match &cell.role {
            CellRole::Input { .. } | CellRole::Derived { .. } | CellRole::Deferred { .. } => {
                HistoryPlan::none()
            }
            CellRole::Recurrent {
                lookback,
                fallbacks,
                ..
            } => HistoryPlan {
                own: fallbacks.clone(),
                inputs: lookback
                    .iter()
                    .map(|link| vec![initial[link.input.0]; link.k])
                    .collect(),
            },
        })
        .collect()
}

/// A computed cell's slot value before construction computes it (§13.14.1
/// step 2).
///
/// Never an argument: every computed cell is in `static_order` and every read of
/// one is an edge of it (R80 item 4), so construction fills a cell's slot before
/// any reader of that cell runs — and step 4's commit publishes only values
/// construction computed.
///
/// It is therefore the DERIVED and DEFERRED seed only. A `recurrent` seeds its
/// slot with its depth-1 FALLBACK instead, because a recurrent IS readable
/// before it has run: §13.11.3's cycle-breaking read hands one to whichever
/// member of a read-cycle the schedule releases, and §13.11.4's "previous-
/// committed value" before any advance is what §4.9's fallbacks are FOR — *"a
/// history read before the ring has advanced that far returns the read's
/// fallback"*. A value the source spells, not a typed zero.
///
/// It exists because the store is allocated to its final size in one shot — the
/// fixed-extent guarantee — which means every slot needs a value of the right
/// type before any behavior has run.
fn placeholder(value_type: ValueType) -> Value {
    match value_type {
        ValueType::I64 => Value::I64(0),
        ValueType::I32 => Value::I32(0),
        ValueType::F64 => Value::F64(0.0),
        ValueType::F32 => Value::F32(0.0),
        ValueType::Bool => Value::Bool(false),
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use ductus_ir::diag::DiagClass;
    use ductus_ir::graph::{Entry, FallbackEntry, LookbackEntry, ValueLiteral};
    use ductus_ir::module::Module;
    use ductus_ir::types::Prim;

    use super::super::test_support::{
        RecordingSink, STARTUP, StubProvider, deferred, derived, entries, gate, gated, input,
        loaded, module, nested, path, recurrent, scope,
    };
    use super::*;
    use crate::provider::Behavior;

    fn i64_of(value: &Value) -> i64 {
        match value {
            Value::I64(payload) => *payload,
            other => panic!("an i64 argument, got {other:?}"),
        }
    }

    fn f64_of(value: &Value) -> f64 {
        match value {
            Value::F64(payload) => *payload,
            other => panic!("an f64 argument, got {other:?}"),
        }
    }

    /// Sums its `i64` arguments — the workhorse stub.
    fn sum() -> Behavior {
        Rc::new(|inputs| Value::I64(inputs.iter().map(i64_of).sum()))
    }

    /// Ignores its arguments and answers the same value every time — the stub
    /// that makes value-change gating visible.
    fn constant(value: Value) -> Behavior {
        Rc::new(move |_| value)
    }

    /// One more than its single `i64` argument — the stub that makes a chain's
    /// depth readable in the trace.
    fn increment() -> Behavior {
        Rc::new(|inputs| Value::I64(i64_of(&inputs[0]) + 1))
    }

    /// The rendered value at `names` — the canonical rendering is what a trace
    /// carries, so tests compare renderings rather than values.
    fn read(snapshot: &Snapshot<'_>, names: &[&str]) -> String {
        snapshot
            .read(&path(names))
            .expect("the cell exists")
            .render()
    }

    fn int(digits: &str) -> ValueLiteral {
        ValueLiteral::int(digits.to_string())
    }

    /// Two `i64` inputs and their sum — the smallest graph with a real
    /// dependency edge.
    fn adder() -> Module {
        module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("2")),
                input(&["App", "y"], Prim::I64, int("3")),
                derived(&["App", "d"], Prim::I64, 1, &[&["App", "x"], &["App", "y"]]),
            ],
        )])
    }

    // §13.14.1 steps 3 and 4, together: construction evaluated the derived
    // once (untraced — it is not a commit), and the first commit published,
    // leaving the empty block every trace opens with.
    #[test]
    fn startup_traces_the_empty_block_and_publishes_the_constructed_values() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let engine = Engine::start(loaded(&witness, &provider), &mut sink);
        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "x"]), "2");
        assert_eq!(read(&snapshot, &["App", "y"]), "3");
        assert_eq!(read(&snapshot, &["App", "d"]), "5");

        engine.teardown();

        assert_eq!(sink.trace(), STARTUP);
    }

    // The whole loop in one commit: the write dirties its input, the derived
    // re-evaluates over the staged value, and the publish makes both visible.
    #[test]
    fn a_write_commits_as_dirty_then_eval_then_publish() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::I64(4))
            .expect("a legal write");
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "x"]), "4");
        assert_eq!(read(&snapshot, &["App", "d"]), "7");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\ndirty App.x 2 -> 4\neval App.d -> 7\npublish\ncommit_end\n"
            )
        );
    }

    // §13.10.1: dirtiness is staged-vs-previously-committed, so writing the
    // value a cell already holds changes nothing — no dirt, no evaluation.
    #[test]
    fn a_write_of_the_committed_value_commits_clean() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::I64(2))
            .expect("a legal write");
        engine.commit();
        engine.teardown();

        assert_eq!(sink.trace(), format!("{STARTUP}{STARTUP}"));
    }

    // The net-revert law, exactly as §13.10.1 spells it: the intermediate 5 is
    // invisible because dirtiness is judged at commit, not per write.
    #[test]
    fn a_reverted_write_commits_clean() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::I64(5))
            .expect("a legal write");
        engine
            .write_signal(&path(&["App", "x"]), Value::I64(2))
            .expect("a legal write");
        engine.commit();
        engine.teardown();

        assert_eq!(sink.trace(), format!("{STARTUP}{STARTUP}"));
    }

    // Accumulated writes to one cell are last-write-wins: only the 6 is ever
    // compared, and the 5 leaves no trace of having existed.
    #[test]
    fn the_last_staged_write_is_the_one_that_commits() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::I64(5))
            .expect("a legal write");
        engine
            .write_signal(&path(&["App", "x"]), Value::I64(6))
            .expect("a legal write");
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "d"]), "9");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\ndirty App.x 2 -> 6\neval App.d -> 9\npublish\ncommit_end\n"
            )
        );
    }

    // Transitive settling: one write, and a→b→c evaluates to the bottom of the
    // chain inside a SINGLE commit, in topological order — each link reads the
    // value the previous link just wrote to pending, never the stale one.
    #[test]
    fn a_chain_settles_to_its_end_in_one_commit() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("0")),
                derived(&["App", "a"], Prim::I64, 1, &[&["App", "x"]]),
                derived(&["App", "b"], Prim::I64, 1, &[&["App", "a"]]),
                derived(&["App", "c"], Prim::I64, 1, &[&["App", "b"]]),
            ],
        )]);
        let provider = StubProvider::new().with(1, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::I64(10))
            .expect("a legal write");
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "c"]), "13");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\n\
                 dirty App.x 0 -> 10\n\
                 eval App.a -> 11\n\
                 eval App.b -> 12\n\
                 eval App.c -> 13\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // Glitch-freedom: the join sees both arms at their THIS-commit values and
    // evaluates exactly once — one `eval App.j` line, never two, and never a
    // half-updated 8.
    #[test]
    fn a_diamond_evaluates_its_join_exactly_once() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("1")),
                derived(&["App", "a"], Prim::I64, 1, &[&["App", "x"]]),
                derived(&["App", "b"], Prim::I64, 1, &[&["App", "x"]]),
                derived(&["App", "j"], Prim::I64, 2, &[&["App", "a"], &["App", "b"]]),
            ],
        )]);
        let provider = StubProvider::new().with(1, increment()).with(2, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::I64(5))
            .expect("a legal write");
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "j"]), "12");

        engine.teardown();

        assert_eq!(sink.trace().matches("eval App.j").count(), 1);
        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\n\
                 dirty App.x 1 -> 5\n\
                 eval App.a -> 6\n\
                 eval App.b -> 6\n\
                 eval App.j -> 12\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // Value-change gating mid-graph (§13.2.4.4, R65): `mid` re-evaluates —
    // its dependency changed — to the value it already held, so it marks
    // itself CLEAN and `down` never evaluates. The absent `eval App.down` line
    // is the whole assertion.
    #[test]
    fn an_equal_derived_result_stops_the_propagation() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("1")),
                derived(&["App", "mid"], Prim::I64, 1, &[&["App", "x"]]),
                derived(&["App", "down"], Prim::I64, 2, &[&["App", "mid"]]),
            ],
        )]);
        let provider = StubProvider::new()
            .with(1, constant(Value::I64(10)))
            .with(2, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::I64(2))
            .expect("a legal write");
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "down"]), "11");

        engine.teardown();

        assert!(!sink.trace().contains("eval App.down"));
        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\n\
                 dirty App.x 1 -> 2\n\
                 eval App.mid -> 10\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // The same-level tiebreak (R64): neither derived depends on the other, so
    // the order is declaration order — `first` was written first in the body,
    // so it evaluates first.
    #[test]
    fn same_level_deriveds_evaluate_in_declaration_order() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("0")),
                derived(&["App", "first"], Prim::I64, 1, &[&["App", "x"]]),
                derived(&["App", "second"], Prim::I64, 1, &[&["App", "x"]]),
            ],
        )]);
        let provider = StubProvider::new().with(1, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::I64(7))
            .expect("a legal write");
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\n\
                 dirty App.x 0 -> 7\n\
                 eval App.first -> 8\n\
                 eval App.second -> 8\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // Two subgraphs that share nothing settle in ONE commit, and the rank order
    // that orders them is scope-major: everything in `App` precedes everything
    // in `App.panel`, whatever the dependency structure says (it says nothing —
    // there is no edge between the islands).
    #[test]
    fn independent_islands_settle_in_one_commit_in_rank_order() {
        let witness = module(vec![
            scope(
                &["App"],
                vec![
                    input(&["App", "a"], Prim::I64, int("0")),
                    derived(&["App", "da"], Prim::I64, 1, &[&["App", "a"]]),
                ],
            ),
            scope(
                &["App", "panel"],
                vec![
                    input(&["App", "panel", "b"], Prim::I64, int("0")),
                    derived(
                        &["App", "panel", "db"],
                        Prim::I64,
                        1,
                        &[&["App", "panel", "b"]],
                    ),
                ],
            ),
        ]);
        let provider = StubProvider::new().with(1, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "panel", "b"]), Value::I64(20))
            .expect("a legal write");
        engine
            .write_signal(&path(&["App", "a"]), Value::I64(10))
            .expect("a legal write");
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\n\
                 dirty App.a 0 -> 10\n\
                 dirty App.panel.b 0 -> 20\n\
                 eval App.da -> 11\n\
                 eval App.panel.db -> 21\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // The dirty lines come out in RANK order, never write order (R64) — `y` was
    // written first and is reported second, because `x` is declared first. This
    // is what makes the trace invariant under permutation of same-commit writes
    // to distinct cells.
    #[test]
    fn dirty_events_follow_rank_order_not_write_order() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "y"]), Value::I64(30))
            .expect("a legal write");
        engine
            .write_signal(&path(&["App", "x"]), Value::I64(20))
            .expect("a legal write");
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\n\
                 dirty App.x 2 -> 20\n\
                 dirty App.y 3 -> 30\n\
                 eval App.d -> 50\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // §13.14.4's idempotence, and the accumulation across commits: a commit
    // with nothing staged publishes a fresh, identical snapshot, and each
    // write's effect is judged against the commit before it — so the second
    // write's `dirty` line opens at the first write's value.
    #[test]
    fn commits_are_idempotent_and_accumulate_across_a_sequence() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine.commit();
        engine
            .write_signal(&path(&["App", "x"]), Value::I64(4))
            .expect("a legal write");
        engine.commit();
        engine
            .write_signal(&path(&["App", "y"]), Value::I64(6))
            .expect("a legal write");
        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "d"]), "10");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}{STARTUP}\
                 commit_begin\ndirty App.x 2 -> 4\neval App.d -> 7\npublish\ncommit_end\n\
                 commit_begin\ndirty App.y 3 -> 6\neval App.d -> 10\npublish\ncommit_end\n\
                 {STARTUP}"
            )
        );
    }

    // The three write refusals (§13.14.2), through the engine's own surface:
    // each carries its class and the path that was asked for, and — the part
    // only the engine can show — a rejected write STAGES NOTHING, so the commit
    // that follows is clean.
    #[test]
    fn a_rejected_write_names_its_class_and_stages_nothing() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        let unknown = engine
            .write_signal(&path(&["App", "nope"]), Value::I64(1))
            .expect_err("no such cell");
        let non_input = engine
            .write_signal(&path(&["App", "d"]), Value::I64(1))
            .expect_err("a derived cell");
        let mistyped = engine
            .write_signal(&path(&["App", "x"]), Value::Bool(true))
            .expect_err("the wrong type");

        assert_eq!(unknown.class, DiagClass::UnknownInputPath);
        assert_eq!(
            unknown.path.as_ref().map(CellPath::render),
            Some("App.nope".to_string())
        );
        assert_eq!(non_input.class, DiagClass::WriteToNonInput);
        assert_eq!(
            non_input.path.as_ref().map(CellPath::render),
            Some("App.d".to_string())
        );
        assert_eq!(mistyped.class, DiagClass::TypeMismatchedWrite);
        assert_eq!(
            mistyped.path.as_ref().map(CellPath::render),
            Some("App.x".to_string())
        );

        engine.commit();
        engine.teardown();

        assert_eq!(sink.trace(), format!("{STARTUP}{STARTUP}"));
    }

    // The read face (§13.14.6): every cell kind reads back by path from one
    // view, and an unknown path is `unknown_input_path`. Exclusivity — no
    // commit while a snapshot is alive — needs no test: `commit` takes
    // `&mut self` and a `Snapshot` holds `&`, so violating it does not compile.
    #[test]
    fn a_snapshot_reads_every_cell_and_refuses_an_unknown_path() {
        let witness = adder();
        let provider = StubProvider::new().with(1, sum());
        let mut sink = RecordingSink::new();
        let engine = Engine::start(loaded(&witness, &provider), &mut sink);
        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "x"]), "2");
        assert_eq!(read(&snapshot, &["App", "y"]), "3");
        assert_eq!(read(&snapshot, &["App", "d"]), "5");

        let failure = snapshot
            .read(&path(&["App", "nope"]))
            .expect_err("no such cell");

        assert_eq!(failure.class, DiagClass::UnknownInputPath);
        assert_eq!(
            failure.path.as_ref().map(CellPath::render),
            Some("App.nope".to_string())
        );

        engine.teardown();
    }

    /// Traps on a negative argument — a behavior that survives construction
    /// (the init is 0) and traps at a later commit.
    fn traps_on_negative() -> Behavior {
        Rc::new(|inputs| {
            assert!(i64_of(&inputs[0]) >= 0, "negative input");

            Value::I64(0)
        })
    }

    /// Traps on every call — including construction's.
    fn always_traps() -> Behavior {
        Rc::new(|_| panic!("always traps"))
    }

    /// A graph whose single derived is `App.boom`, over one `i64` input.
    fn boom_graph() -> Module {
        module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("0")),
                derived(&["App", "boom"], Prim::I64, 1, &[&["App", "x"]]),
            ],
        )])
    }

    // D8's trap shape: the panic that reaches the harness names the CELL, so a
    // trap is diagnosable without a debugger. The payload rides along after the
    // colon.
    #[test]
    #[should_panic(expected = "behavior trap at App.boom")]
    fn a_trapping_behavior_aborts_the_commit_naming_its_cell() {
        let witness = boom_graph();
        let provider = StubProvider::new().with(1, traps_on_negative());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::I64(-1))
            .expect("a legal write");
        engine.commit();
    }

    // A trap during CONSTRUCTION (§13.14.1 step 3) aborts exactly as one during
    // a commit does — same invoke path, same message shape — even though
    // construction itself is untraced.
    #[test]
    #[should_panic(expected = "behavior trap at App.boom: always traps")]
    fn a_trap_during_construction_aborts_the_same_way() {
        let witness = boom_graph();
        let provider = StubProvider::new().with(1, always_traps());
        let mut sink = RecordingSink::new();

        Engine::start(loaded(&witness, &provider), &mut sink).teardown();
    }

    // A behavior returning the wrong type is a harness-author bug the loader
    // cannot catch — it aborts naming the cell, the declared type, and what
    // arrived.
    #[test]
    #[should_panic(expected = "behavior for `App.boom` produced a `bool`")]
    fn a_wrong_typed_result_aborts_naming_the_cell() {
        let witness = boom_graph();
        let provider = StubProvider::new().with(1, constant(Value::Bool(true)));
        let mut sink = RecordingSink::new();

        Engine::start(loaded(&witness, &provider), &mut sink).teardown();
    }

    /// Twice its single `f64` argument.
    fn double_float() -> Behavior {
        Rc::new(|inputs| Value::F64(f64_of(&inputs[0]) * 2.0))
    }

    // R65, the NaN half: the comparison is bitwise, so writing the SAME NaN a
    // cell already holds is clean. IEEE equality would have made this cell
    // perpetually dirty.
    #[test]
    fn a_nan_rewritten_with_the_same_bits_is_clean() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::F64, ValueLiteral::Nan),
                derived(&["App", "d"], Prim::F64, 1, &[&["App", "x"]]),
            ],
        )]);
        let provider = StubProvider::new().with(1, double_float());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::F64(f64::NAN))
            .expect("a legal write");
        engine.commit();
        engine.teardown();

        assert_eq!(sink.trace(), format!("{STARTUP}{STARTUP}"));
    }

    // R65, the signed-zero half: `0.0` and `-0.0` are different bits, so the
    // write is DIRTY — and the trace pins the canonical rendering of both
    // (`-0.0` spells `-0`, R34).
    #[test]
    fn a_signed_zero_write_is_dirty_and_renders_as_minus_zero() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(
                    &["App", "x"],
                    Prim::F64,
                    ValueLiteral::float("0.0".to_string()),
                ),
                derived(&["App", "d"], Prim::F64, 1, &[&["App", "x"]]),
            ],
        )]);
        let provider = StubProvider::new().with(1, double_float());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::F64(-0.0))
            .expect("a legal write");
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\n\
                 dirty App.x 0 -> -0\n\
                 eval App.d -> -0\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // Value-change gating over NaN: `mid` produces a bit-identical NaN twice,
    // so it marks itself clean and `down` does not evaluate — the same gating
    // an equal integer produces, which is exactly what makes bitwise
    // comparison the right law rather than a special case.
    #[test]
    fn a_bit_identical_nan_result_gates_the_downstream_derived() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(
                    &["App", "x"],
                    Prim::F64,
                    ValueLiteral::float("0.0".to_string()),
                ),
                derived(&["App", "mid"], Prim::F64, 1, &[&["App", "x"]]),
                derived(&["App", "down"], Prim::F64, 2, &[&["App", "mid"]]),
            ],
        )]);
        let provider = StubProvider::new()
            .with(1, constant(Value::F64(f64::NAN)))
            .with(2, double_float());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine
            .write_signal(&path(&["App", "x"]), Value::F64(1.0))
            .expect("a legal write");
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\n\
                 dirty App.x 0 -> 1\n\
                 eval App.mid -> NaN\n\
                 publish\ncommit_end\n"
            )
        );
    }

    /// Each `i64` argument weighted by its position — `a0 + 10·a1 + 100·a2 + …`
    /// — so ONE value spells out the whole argument list and the ABI order
    /// (R71, §4.9) is readable straight off the trace.
    fn positional() -> Behavior {
        Rc::new(|inputs| {
            Value::I64(
                inputs
                    .iter()
                    .enumerate()
                    .map(|(at, value)| i64_of(value) * 10i64.pow(at as u32))
                    .sum(),
            )
        })
    }

    /// The corpus's `fib`: `past1 + past2`, ignoring the trigger input.
    fn fibonacci() -> Behavior {
        Rc::new(|inputs| Value::I64(i64_of(&inputs[1]) + i64_of(&inputs[2])))
    }

    /// Stages a legal write — every witness below writes the same way.
    fn write(engine: &mut Engine<'_>, names: &[&str], value: Value) {
        engine
            .write_signal(&path(names), value)
            .expect("a legal write");
    }

    // §6.1's witness 1 — the depth-1 accumulator, IR_GRAMMAR §5's own example.
    // Construction evaluates it once against the fallback (0 + 0), and each
    // commit adds this commit's count to the value one advance back: the total
    // climbs by lockstep, never by two steps, and `past(1)` is always the value
    // the cell currently holds.
    #[test]
    fn a_depth_one_recurrent_accumulates_one_advance_per_commit() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "count"], Prim::I64, int("0")),
                recurrent(
                    &["App", "total"],
                    Prim::I64,
                    2,
                    &[&["App", "count"]],
                    1,
                    int("0"),
                ),
            ],
        )]);
        let provider = StubProvider::new().with(2, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "count"], Value::I64(5));
        engine.commit();
        write(&mut engine, &["App", "count"], Value::I64(3));
        engine.commit();
        write(&mut engine, &["App", "count"], Value::I64(2));
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "total"]), "10");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.count 0 -> 5\neval App.total -> 5\npublish\ncommit_end\n\
                 commit_begin\ndirty App.count 5 -> 3\neval App.total -> 8\npublish\ncommit_end\n\
                 commit_begin\ndirty App.count 3 -> 2\neval App.total -> 10\npublish\ncommit_end\n"
            )
        );
    }

    // What a DERIVED reading a recurrent sees — the advance-root law (§4.12)
    // made visible, and the reason the law exists.
    //
    // RE-DERIVED for R80 item 1. Step 3 sets NO dirty bit for a recurrent, so
    // the derived does not evaluate in the advancing commit at all; the advance
    // is a SILENT dirty root at the NEXT commit's step 1b, and the derived
    // evaluates there over the PUBLISHED value. Walked out:
    //
    //   commit 1 (count 0 -> 5): total = sum(5, past1=0) = 5, held, advanced.
    //                            `shown` sees no dirty dependency — no eval.
    //   commit 2 (count 5 -> 3): the advance root dirties `total` silently;
    //                            total = sum(3, past1=5) = 8, held, advanced;
    //                            `shown` = increment(pending total = 5) = 6.
    //   commit 3 (quiescent):    the advance root from commit 2 dirties `total`
    //                            silently; `total` has no dirty VALUE input so
    //                            it does not re-evaluate; `shown` =
    //                            increment(8) = 9.
    //
    // So the derived trails the recurrent by exactly one advance, and the
    // quiescent commit CLOSES the gap rather than leaving it open — which is
    // the delay chain, and is what the old expectations (shown = 6, a third
    // block identical to STARTUP) got wrong before the law landed.
    #[test]
    fn a_derived_reading_a_recurrent_trails_it_by_one_advance() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "count"], Prim::I64, int("0")),
                recurrent(
                    &["App", "total"],
                    Prim::I64,
                    2,
                    &[&["App", "count"]],
                    1,
                    int("0"),
                ),
                derived(&["App", "shown"], Prim::I64, 3, &[&["App", "total"]]),
            ],
        )]);
        let provider = StubProvider::new().with(2, sum()).with(3, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "count"], Value::I64(5));
        engine.commit();
        write(&mut engine, &["App", "count"], Value::I64(3));
        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "total"]), "8");
        assert_eq!(read(&snapshot, &["App", "shown"]), "9");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.count 0 -> 5\n\
                 eval App.total -> 5\npublish\ncommit_end\n\
                 commit_begin\ndirty App.count 5 -> 3\n\
                 eval App.total -> 8\neval App.shown -> 6\npublish\ncommit_end\n\
                 commit_begin\neval App.shown -> 9\npublish\ncommit_end\n"
            )
        );
    }

    // The advance-root law's POINT (§4.12), pinned as its own regression: the
    // schedule may not depend on entry rank.
    //
    // Before the law landed it did. Step 3 set a recurrent's dirty bit from its
    // held value, so a reader declared AFTER the recurrent evaluated inside the
    // advancing commit — over the PRE-advance value — while a reader declared
    // BEFORE it was already past in the walk and did not (FND-38). Neither does
    // now: the advance is a silent root at the NEXT commit for both, so the two
    // graphs settle identically and their traces are byte-identical.
    #[test]
    fn declaration_order_settles_a_reader_of_a_recurrent_identically() {
        fn settle(reader_first: bool) -> (String, String, String) {
            let count = input(&["App", "count"], Prim::I64, int("0"));
            let accumulator = recurrent(
                &["App", "total"],
                Prim::I64,
                2,
                &[&["App", "count"]],
                1,
                int("0"),
            );
            let reader = derived(&["App", "shown"], Prim::I64, 3, &[&["App", "total"]]);

            let cells = if reader_first {
                vec![count, reader, accumulator]
            } else {
                vec![count, accumulator, reader]
            };

            let witness = module(vec![scope(&["App"], cells)]);
            let provider = StubProvider::new().with(2, sum()).with(3, increment());
            let mut sink = RecordingSink::new();
            let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

            write(&mut engine, &["App", "count"], Value::I64(5));
            engine.commit();
            write(&mut engine, &["App", "count"], Value::I64(3));
            engine.commit();
            engine.commit();

            let snapshot = engine.acquire_snapshot();
            let settled = (
                read(&snapshot, &["App", "total"]),
                read(&snapshot, &["App", "shown"]),
            );

            engine.teardown();

            (settled.0, settled.1, sink.trace().to_string())
        }

        let before = settle(true);
        let after = settle(false);

        assert_eq!(before, after);
        assert_eq!(before.0, "8");
        assert_eq!(before.1, "9");
    }

    // R80 item 4 — construction order IS the static order, so a read of a
    // recurrent orders its reader after it whatever the declaration order says.
    //
    // `App.shown` is declared BEFORE the recurrent it reads and must still
    // construct from the recurrent's CONSTRUCTED value: `App.total` constructs
    // to `sum(count = 4, past1 = 0)` = 4, so `App.shown` is `increment(4)` = 5.
    // A rank-ordered construction pass would hand it the allocation placeholder
    // and answer 1 — which is what the doc on `placeholder` promises can happen
    // only inside a cycle.
    #[test]
    fn a_reader_declared_before_its_recurrent_constructs_from_its_value() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "count"], Prim::I64, int("4")),
                derived(&["App", "shown"], Prim::I64, 3, &[&["App", "total"]]),
                recurrent(
                    &["App", "total"],
                    Prim::I64,
                    2,
                    &[&["App", "count"]],
                    1,
                    int("0"),
                ),
            ],
        )]);
        let provider = StubProvider::new().with(2, sum()).with(3, increment());
        let mut sink = RecordingSink::new();
        let engine = Engine::start(loaded(&witness, &provider), &mut sink);

        assert_eq!(
            engine.graph.static_order,
            [CellIndex(2), CellIndex(1)],
            "the recurrent constructs first even though it is declared last"
        );

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "total"]), "4");
        assert_eq!(read(&snapshot, &["App", "shown"]), "5");

        engine.teardown();

        assert_eq!(sink.trace(), STARTUP);
    }

    // §6.1's witness 2 — what a history read answers before any advance. The
    // CONSTRUCTION evaluation is the first one, and it reads every depth's own
    // fallback (3 and 5 here, so `0 + 3 + 5`). At the first commit `past(1)`
    // has become the constructed value while `past(2)` is still ITS fallback —
    // the per-`k` law, visible in one line.
    #[test]
    fn the_first_evaluations_read_their_own_per_depth_fallbacks() {
        let mut history = recurrent(
            &["App", "r"],
            Prim::I64,
            2,
            &[&["App", "tick"]],
            2,
            int("0"),
        );
        history.init = None;
        history.fallbacks = vec![
            FallbackEntry {
                k: 1,
                value: int("3"),
            },
            FallbackEntry {
                k: 2,
                value: int("5"),
            },
        ];

        let witness = module(vec![scope(
            &["App"],
            vec![input(&["App", "tick"], Prim::I64, int("0")), history],
        )]);
        let provider = StubProvider::new().with(2, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "r"]), "8");

        write(&mut engine, &["App", "tick"], Value::I64(1));
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}commit_begin\n\
                 dirty App.tick 0 -> 1\n\
                 eval App.r -> 14\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // §6.1's witness 5 — the corpus's `fib`, per-`k` fallbacks and all. Each
    // slot shifts by exactly one per advance, so the sequence is 1, 1, 2, 3, 5
    // and never 1, 1, 1, 2, 3 (which is what a ring that lost its constructed
    // value would produce).
    #[test]
    fn per_depth_fallbacks_seed_the_corpus_fibonacci() {
        let mut fib = recurrent(
            &["App", "fib"],
            Prim::I64,
            2,
            &[&["App", "tick"]],
            2,
            int("0"),
        );
        fib.init = None;
        fib.fallbacks = vec![
            FallbackEntry {
                k: 1,
                value: int("1"),
            },
            FallbackEntry {
                k: 2,
                value: int("0"),
            },
        ];

        let witness = module(vec![scope(
            &["App"],
            vec![input(&["App", "tick"], Prim::I64, int("0")), fib],
        )]);
        let provider = StubProvider::new().with(2, fibonacci());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "fib"]), "1");

        for tick in 1..=4 {
            write(&mut engine, &["App", "tick"], Value::I64(tick));
            engine.commit();
        }

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "fib"]), "5");

        engine.teardown();

        let evaluations: Vec<&str> = sink
            .trace()
            .lines()
            .filter(|line| line.starts_with("eval App.fib"))
            .collect();

        assert_eq!(
            evaluations,
            [
                "eval App.fib -> 1",
                "eval App.fib -> 2",
                "eval App.fib -> 3",
                "eval App.fib -> 5"
            ]
        );
    }

    /// Two recurrents that read each other's CURRENT values, each also reading
    /// its own past — the lockstep pair.
    fn lockstep_pair() -> Module {
        module(vec![scope(
            &["App"],
            vec![
                input(&["App", "tick"], Prim::I64, int("0")),
                recurrent(
                    &["App", "a"],
                    Prim::I64,
                    2,
                    &[&["App", "tick"], &["App", "b"]],
                    1,
                    int("0"),
                ),
                recurrent(
                    &["App", "b"],
                    Prim::I64,
                    2,
                    &[&["App", "tick"], &["App", "a"]],
                    1,
                    int("0"),
                ),
            ],
        )])
    }

    // §6.1's witness 3 — lockstep across a pair (023-35). At the second commit
    // `a` advances to 4 BEFORE `b` evaluates in the same pass, and `b` still
    // reads 1: the value `a` held at the commit's start. A `b` that saw the
    // just-advanced value would answer 7, so the two readings are one line
    // apart in the trace.
    #[test]
    fn a_recurrent_never_sees_another_recurrents_just_advanced_value() {
        let witness = lockstep_pair();
        let provider = StubProvider::new().with(2, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "tick"], Value::I64(1));
        engine.commit();
        write(&mut engine, &["App", "tick"], Value::I64(2));
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "a"]), "4");
        assert_eq!(read(&snapshot, &["App", "b"]), "4");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.tick 0 -> 1\n\
                 eval App.a -> 1\neval App.b -> 1\npublish\ncommit_end\n\
                 commit_begin\ndirty App.tick 1 -> 2\n\
                 eval App.a -> 4\neval App.b -> 4\npublish\ncommit_end\n"
            )
        );
    }

    // §6.1's witness 4, the running half — a recurrent whose only input is its
    // own PAST. `depth` is what makes that a read: the ring is its whole
    // argument list, and no `inputs` entry names the cell (an entry that did
    // would be `recurrent_self_reference`, rejected at load).
    //
    // Nothing drives it after construction, and that is the assertion: with no
    // value input there is no staleness trigger, so it evaluates once at
    // construction and never again, whatever the host commits.
    #[test]
    fn a_recurrent_whose_only_input_is_its_own_past_runs_on_its_ring() {
        let witness = module(vec![scope(
            &["App"],
            vec![recurrent(&["App", "r"], Prim::I64, 2, &[], 1, int("7"))],
        )]);
        let provider = StubProvider::new().with(2, sum());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "r"]), "7");

        engine.teardown();

        assert_eq!(sink.trace(), format!("{STARTUP}{STARTUP}{STARTUP}"));
    }

    // The one read a construction placeholder could reach, and what it gets
    // instead: `step` reads `acc` and `acc` reads `step`, so the schedule
    // releases `step` first and it reads `acc`'s slot before `acc` has run.
    //
    // That slot holds `acc`'s depth-1 FALLBACK — the spelled `init 5` — not a
    // typed zero. `step` therefore constructs at `5 + 1 = 6` and `acc` at
    // `6 + 5 = 11`; a typed-zero seed would give `1` and `6`. The value a read
    // of an unadvanced recurrent yields is the one §4.9 already names
    // (§13.11.3's previous-committed reading, before any advance exists).
    #[test]
    fn a_read_of_an_unconstructed_recurrent_yields_its_fallback() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("1")),
                recurrent(
                    &["App", "acc"],
                    Prim::I64,
                    2,
                    &[&["App", "step"]],
                    1,
                    int("5"),
                ),
                derived(
                    &["App", "step"],
                    Prim::I64,
                    2,
                    &[&["App", "acc"], &["App", "x"]],
                ),
            ],
        )]);
        let provider = StubProvider::new().with(2, sum());
        let mut sink = RecordingSink::new();
        let engine = Engine::start(loaded(&witness, &provider), &mut sink);
        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "step"]), "6");
        assert_eq!(read(&snapshot, &["App", "acc"]), "11");

        engine.teardown();
    }

    // §6.1's witness 6 — a `depth 3` ring, read through a behavior that
    // weights each argument by its position, so one value spells the whole
    // ring. Each advance moves every slot by exactly one: 1 becomes the tens
    // digit, then the hundreds, then the thousands.
    #[test]
    fn a_depth_three_ring_shifts_every_slot_by_one_per_advance() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("0")),
                recurrent(&["App", "r"], Prim::I64, 2, &[&["App", "x"]], 3, int("0")),
            ],
        )]);
        let provider = StubProvider::new().with(2, positional());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        for x in 1..=4 {
            write(&mut engine, &["App", "x"], Value::I64(x));
            engine.commit();
        }

        engine.teardown();

        let evaluations: Vec<&str> = sink
            .trace()
            .lines()
            .filter(|line| line.starts_with("eval"))
            .collect();

        assert_eq!(
            evaluations,
            [
                "eval App.r -> 1",
                "eval App.r -> 12",
                "eval App.r -> 223",
                "eval App.r -> 4434"
            ]
        );
    }

    /// Answers a constant while `mode` is zero, and its own `past2` once
    /// `mode` changes — the stub that makes an equal-valued advance's effect on
    /// the ring readable. Arguments: `mode`, `tick`, `past1`, `past2`.
    fn constant_then_past_two() -> Behavior {
        Rc::new(|inputs| {
            if i64_of(&inputs[0]) == 0 {
                Value::I64(7)
            } else {
                Value::I64(i64_of(&inputs[3]))
            }
        })
    }

    // §6.1's witness 7 — R77's regression guard, both halves in one witness.
    // The first commit's evaluation produces the value the cell already held,
    // so it marks itself CLEAN and the derived below it never runs. The ring
    // still shifted, and the second commit proves it: `past2` answers with the
    // constructed 7, which is only in that slot because the equal-valued
    // advance consumed one. Had the ring not moved, `past2` would still be its
    // fallback and the line would read `eval App.r -> 0`.
    #[test]
    fn an_equal_valued_advance_still_shifts_the_ring_and_propagates_nothing() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "mode"], Prim::I64, int("0")),
                input(&["App", "tick"], Prim::I64, int("0")),
                recurrent(
                    &["App", "r"],
                    Prim::I64,
                    2,
                    &[&["App", "mode"], &["App", "tick"]],
                    2,
                    int("0"),
                ),
                derived(&["App", "down"], Prim::I64, 3, &[&["App", "r"]]),
            ],
        )]);
        let provider = StubProvider::new()
            .with(2, constant_then_past_two())
            .with(3, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "tick"], Value::I64(1));
        engine.commit();
        write(&mut engine, &["App", "mode"], Value::I64(1));
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "down"]), "8");

        engine.teardown();

        assert!(!sink.trace().contains("eval App.down"));
        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.tick 0 -> 1\neval App.r -> 7\npublish\ncommit_end\n\
                 commit_begin\ndirty App.mode 0 -> 1\neval App.r -> 7\npublish\ncommit_end\n"
            )
        );
    }

    // §6.1's witness 8 — fork 2's discriminating case (R72). The middle commit
    // is empty, so the recurrent does not evaluate and consumes no slot: at the
    // third commit `past1` is the FIRST commit's value and `past2` is still the
    // constructed one. A ring that shifted per commit would have duplicated the
    // held value into `past1` and pushed the first commit's 1 into `past2`,
    // making the last line read 112.
    #[test]
    fn a_commit_without_an_evaluation_consumes_no_slot() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("0")),
                recurrent(&["App", "r"], Prim::I64, 2, &[&["App", "x"]], 2, int("0")),
            ],
        )]);
        let provider = StubProvider::new().with(2, positional());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "x"], Value::I64(1));
        engine.commit();
        engine.commit();
        write(&mut engine, &["App", "x"], Value::I64(2));
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.x 0 -> 1\neval App.r -> 1\npublish\ncommit_end\n\
                 {STARTUP}\
                 commit_begin\ndirty App.x 1 -> 2\neval App.r -> 12\npublish\ncommit_end\n"
            )
        );
    }

    /// Answers 1 when its own `past2` is bit-for-bit `NaN`, and echoes `x`
    /// otherwise. Arguments: `x`, `past1`, `past2`.
    fn nan_in_the_ring() -> Behavior {
        Rc::new(|inputs| {
            let past2 = f64_of(&inputs[2]);

            if past2.to_bits() == f64::NAN.to_bits() {
                Value::F64(1.0)
            } else {
                Value::F64(f64_of(&inputs[0]))
            }
        })
    }

    // §6.1's witness 10 — R65's bitwise law applied to a history slot. The NaN
    // written at the first commit rides two advances and comes back out of
    // `past2` with the SAME bit pattern, which is the only reason the last line
    // reads 1.
    #[test]
    fn a_nan_survives_the_ring_bit_for_bit() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(
                    &["App", "x"],
                    Prim::F64,
                    ValueLiteral::float("0.0".to_string()),
                ),
                recurrent(
                    &["App", "r"],
                    Prim::F64,
                    2,
                    &[&["App", "x"]],
                    2,
                    ValueLiteral::float("0.0".to_string()),
                ),
            ],
        )]);
        let provider = StubProvider::new().with(2, nan_in_the_ring());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "x"], Value::F64(f64::NAN));
        engine.commit();
        write(&mut engine, &["App", "x"], Value::F64(2.0));
        engine.commit();
        write(&mut engine, &["App", "x"], Value::F64(3.0));
        engine.commit();
        engine.teardown();

        let evaluations: Vec<&str> = sink
            .trace()
            .lines()
            .filter(|line| line.starts_with("eval"))
            .collect();

        assert_eq!(
            evaluations,
            ["eval App.r -> NaN", "eval App.r -> 2", "eval App.r -> 1"]
        );
    }

    // §6.1's witness 9 — `lookback` input history, and the ABI order it
    // arrives in. The positional stub spells the argument list out: units are
    // `x`, tens the cell's own `past1`, hundreds `x_past1`, thousands
    // `x_past2` (R71's §4.9 order). The empty third commit is the cadence
    // half: an input ring steps on the READING recurrent's advances, so the
    // skipped commit leaves it untouched and the last line reads 2323 rather
    // than 3323.
    #[test]
    fn lookback_rings_arrive_in_abi_order_and_step_per_advance() {
        let mut reader = recurrent(&["App", "r"], Prim::I64, 2, &[&["App", "x"]], 1, int("0"));
        reader.lookback = vec![LookbackEntry {
            cell: path(&["App", "x"]),
            k: 2,
        }];

        let witness = module(vec![scope(
            &["App"],
            vec![input(&["App", "x"], Prim::I64, int("0")), reader],
        )]);
        let provider = StubProvider::new().with(2, positional());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "x"], Value::I64(1));
        engine.commit();
        write(&mut engine, &["App", "x"], Value::I64(2));
        engine.commit();
        engine.commit();
        write(&mut engine, &["App", "x"], Value::I64(3));
        engine.commit();
        engine.teardown();

        let evaluations: Vec<&str> = sink
            .trace()
            .lines()
            .filter(|line| line.starts_with("eval"))
            .collect();

        assert_eq!(
            evaluations,
            ["eval App.r -> 1", "eval App.r -> 112", "eval App.r -> 2323"]
        );
    }

    /// A recurrent whose `lookback` names ANOTHER recurrent — the shape that
    /// makes step 4's drain order visible if the advance is written as one
    /// pass. `b` accumulates; `a` reads `b`'s current value and `b`'s value one
    /// advance back.
    fn lookback_across_recurrents() -> Module {
        let mut reader = recurrent(
            &["App", "a"],
            Prim::I64,
            3,
            &[&["App", "tick"], &["App", "b"]],
            1,
            int("0"),
        );
        reader.lookback = vec![LookbackEntry {
            cell: path(&["App", "b"]),
            k: 1,
        }];

        module(vec![scope(
            &["App"],
            vec![
                input(&["App", "tick"], Prim::I64, int("0")),
                recurrent(
                    &["App", "b"],
                    Prim::I64,
                    2,
                    &[&["App", "tick"]],
                    1,
                    int("0"),
                ),
                reader,
            ],
        )])
    }

    // Lockstep reaches INPUT history too: what `a`'s ring records for `b` is
    // the value `a` saw — `b`'s value before this commit's advance — not the
    // one `b` advanced to in the same pass. At the second commit that makes the
    // thousands digit 0 rather than 1; a step 4 that published each held value
    // before the next cell's rings advanced would read `b`'s just-advanced
    // value and answer 1112.
    #[test]
    fn an_input_ring_records_the_value_its_reader_saw() {
        let witness = lookback_across_recurrents();
        let provider = StubProvider::new().with(2, sum()).with(3, positional());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "tick"], Value::I64(1));
        engine.commit();
        write(&mut engine, &["App", "tick"], Value::I64(2));
        engine.commit();
        engine.teardown();

        let evaluations: Vec<&str> = sink
            .trace()
            .lines()
            .filter(|line| line.starts_with("eval App.a"))
            .collect();

        assert_eq!(evaluations, ["eval App.a -> 1", "eval App.a -> 112"]);
    }

    // Step 4's drain order is UNOBSERVABLE, asserted directly rather than
    // argued: the same hold list drained forwards and backwards leaves the same
    // pending values, the same rings — self history and input history alike —
    // and the same recorded advance roots (§4.12). Every read this commit
    // resolved in step 3, so an advance has nothing left to race with (§4.4).
    #[test]
    fn the_drain_order_of_step_four_cannot_leak() {
        let witness = lookback_across_recurrents();
        let provider = StubProvider::new().with(2, sum()).with(3, positional());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "tick"], Value::I64(1));
        engine.commit();

        let first = engine.graph.index[&path(&["App", "a"])];
        let second = engine.graph.index[&path(&["App", "b"])];
        let held = [(first, Value::I64(9)), (second, Value::I64(4))];
        let flipped: Vec<(CellIndex, Value)> = held.iter().rev().copied().collect();
        let mut forwards = engine.store.clone();
        let mut backwards = engine.store.clone();

        let mut forward_advances = vec![false; engine.graph.cells.len()];
        let mut backward_advances = vec![false; engine.graph.cells.len()];

        forwards.begin_pending();
        backwards.begin_pending();
        advance_recurrents(&engine.graph, &mut forwards, &held, &mut forward_advances);
        advance_recurrents(
            &engine.graph,
            &mut backwards,
            &flipped,
            &mut backward_advances,
        );

        assert_eq!(forward_advances, backward_advances);

        for cell in [first, second] {
            assert!(forwards.pending(cell).bits_eq(&backwards.pending(cell)));
            assert!(forwards.past(cell, 1).bits_eq(&backwards.past(cell, 1)));
        }

        assert!(
            forwards
                .input_past(first, 0, 1)
                .bits_eq(&backwards.input_past(first, 0, 1))
        );
    }

    /// One `i64` input feeding one `deferred` at the given latency — the
    /// smallest graph a deferred can live in.
    fn delayed(depth: u64) -> Module {
        module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("0")),
                deferred(&["App", "later"], Prim::I64, 2, &[&["App", "x"]], depth),
            ],
        )])
    }

    // §6.2's witness 1 — the latency-1 round trip, and the shape of every
    // witness below it. The write at the first commit dirties `App.x` and
    // dispatches the run; the run's value appears at the NEXT commit, as a
    // `dirty` line the host did not stage (R75: a landing IS a dirty root, so
    // it reuses the event a root gets).
    //
    // The dispatch itself leaves nothing in the trace, because it publishes
    // nothing. Under M0.md R1's sequential rendering the behavior ran at
    // dispatch and the value has merely been waiting; a later threaded
    // milestone that really schedules the run must produce THIS trace, which is
    // what makes R1's "as-if" law checkable rather than merely stated.
    #[test]
    fn a_latency_one_deferred_lands_at_the_next_commit() {
        let witness = delayed(1);
        let provider = StubProvider::new().with(2, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "later"]), "1");

        write(&mut engine, &["App", "x"], Value::I64(5));
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "later"]), "1");
        assert!(engine.has_staged_work());

        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "later"]), "6");
        assert!(!engine.has_staged_work());

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.x 0 -> 5\npublish\ncommit_end\n\
                 commit_begin\ndirty App.later 1 -> 6\npublish\ncommit_end\n"
            )
        );
    }

    // §6.2's witnesses 2 and 3 — the latency counts COMMITS, and the two
    // commits it spans are ordinary empty blocks: nothing lands early, and the
    // cell serves its last published value throughout. The `{STARTUP}`s in the
    // middle of the expected trace are that emptiness — a commit crossed with a
    // run in flight is indistinguishable from a commit with nothing to do.
    #[test]
    fn a_latency_three_deferred_serves_its_old_value_for_three_commits() {
        let witness = delayed(3);
        let provider = StubProvider::new().with(2, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "x"], Value::I64(5));

        for _ in 0..3 {
            engine.commit();

            assert_eq!(read(&engine.acquire_snapshot(), &["App", "later"]), "1");
            assert!(engine.has_staged_work());
        }

        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "later"]), "6");
        assert!(!engine.has_staged_work());

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.x 0 -> 5\npublish\ncommit_end\n\
                 {STARTUP}{STARTUP}\
                 commit_begin\ndirty App.later 1 -> 6\npublish\ncommit_end\n"
            )
        );
    }

    // §6.2's witness 4 — the landing is a dirty ROOT, so the deriveds below it
    // evaluate over it in steps 2–3 exactly as over a changed signal. The
    // second block is the whole assertion: a `dirty` line no host staged,
    // followed by the `eval` it caused.
    #[test]
    fn a_landing_drives_the_deriveds_below_it() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("0")),
                deferred(&["App", "later"], Prim::I64, 2, &[&["App", "x"]], 1),
                derived(&["App", "shown"], Prim::I64, 3, &[&["App", "later"]]),
            ],
        )]);
        let provider = StubProvider::new()
            .with(2, increment())
            .with(3, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "x"], Value::I64(5));
        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "later"]), "6");
        assert_eq!(read(&snapshot, &["App", "shown"]), "7");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.x 0 -> 5\npublish\ncommit_end\n\
                 commit_begin\ndirty App.later 1 -> 6\neval App.shown -> 7\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // §6.2's witness 5 — 023-64's pipeline: "Up to `N` runs of a `deferred[N]`
    // cell's one behavior may be in flight at once — a pipeline yielding
    // full-rate output at N-commit latency." Writes at four consecutive commits
    // give landings at four consecutive commits, two behind, and the last two
    // land after the writes have stopped.
    #[test]
    fn consecutive_writes_give_a_full_rate_pipeline_two_commits_behind() {
        let witness = delayed(2);
        let provider = StubProvider::new().with(2, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        for x in 1..=4 {
            write(&mut engine, &["App", "x"], Value::I64(x));
            engine.commit();
        }

        assert!(engine.has_staged_work());

        engine.commit();
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "later"]), "5");
        assert!(!engine.has_staged_work());

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.x 0 -> 1\npublish\ncommit_end\n\
                 commit_begin\ndirty App.x 1 -> 2\npublish\ncommit_end\n\
                 commit_begin\ndirty App.x 2 -> 3\ndirty App.later 1 -> 2\n\
                 publish\ncommit_end\n\
                 commit_begin\ndirty App.x 3 -> 4\ndirty App.later 2 -> 3\n\
                 publish\ncommit_end\n\
                 commit_begin\ndirty App.later 3 -> 4\npublish\ncommit_end\n\
                 commit_begin\ndirty App.later 4 -> 5\npublish\ncommit_end\n"
            )
        );
    }

    // §6.2's witness 6 — R76's single rank sequence. The second commit carries
    // a host write at rank 0 and two landings at ranks 1 and 3, and the three
    // `dirty` lines come out in ONE rank order. A reader cannot tell which line
    // a host staged and which a run produced; neither can a consumer, which is
    // why the trace does not say (R24).
    #[test]
    fn landings_and_host_writes_interleave_in_one_rank_sequence() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "a"], Prim::I64, int("0")),
                deferred(&["App", "f"], Prim::I64, 2, &[&["App", "a"]], 1),
                input(&["App", "b"], Prim::I64, int("0")),
                deferred(&["App", "g"], Prim::I64, 2, &[&["App", "b"]], 1),
            ],
        )]);
        let provider = StubProvider::new().with(2, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "b"], Value::I64(20));
        write(&mut engine, &["App", "a"], Value::I64(10));
        engine.commit();
        write(&mut engine, &["App", "a"], Value::I64(30));
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.a 0 -> 10\ndirty App.b 0 -> 20\n\
                 publish\ncommit_end\n\
                 commit_begin\ndirty App.a 10 -> 30\ndirty App.f 1 -> 11\n\
                 dirty App.g 1 -> 21\npublish\ncommit_end\n"
            )
        );
    }

    // §6.2's witness 7 — R65's bitwise gating at BOTH ends of a deferred's
    // life, in one sequence.
    //
    // The first commit writes a cell the deferred does not read: its provenance
    // is clean, so nothing dispatches (C-28) and `has_staged_work` stays false.
    // The second writes `App.x` and dispatches. The third is where the landing
    // gating shows: the run lands the value the cell already holds, so there is
    // no `dirty` line and `App.down` never evaluates — and yet the run IS
    // consumed, which `has_staged_work` going false is the only way to see.
    #[test]
    fn an_unchanged_provenance_dispatches_nothing_and_an_equal_landing_propagates_nothing() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("0")),
                input(&["App", "other"], Prim::I64, int("0")),
                deferred(&["App", "f"], Prim::I64, 2, &[&["App", "x"]], 1),
                derived(&["App", "down"], Prim::I64, 3, &[&["App", "f"]]),
            ],
        )]);
        let provider = StubProvider::new()
            .with(2, constant(Value::I64(7)))
            .with(3, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "other"], Value::I64(1));
        engine.commit();

        assert!(!engine.has_staged_work());

        write(&mut engine, &["App", "x"], Value::I64(5));
        engine.commit();

        assert!(engine.has_staged_work());

        engine.commit();

        assert!(!engine.has_staged_work());
        assert_eq!(read(&engine.acquire_snapshot(), &["App", "down"]), "8");

        engine.teardown();

        assert!(!sink.trace().contains("eval App.down"));
        assert!(!sink.trace().contains("dirty App.f"));
        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.other 0 -> 1\npublish\ncommit_end\n\
                 commit_begin\ndirty App.x 0 -> 5\npublish\ncommit_end\n\
                 {STARTUP}"
            )
        );
    }

    // §6.2's witness 8 — chained deferreds, and C-29's rule that makes them
    // compose. The landing at the second commit sets `App.f`'s dirty bit, which
    // is `App.g`'s provenance, so `App.g` dispatches at that SAME commit's end
    // and lands one commit later: the write at commit 1 reaches `App.g` at
    // commit 3, latencies adding.
    #[test]
    fn a_chained_deferred_dispatches_on_the_landing_and_adds_its_latency() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("0")),
                deferred(&["App", "f"], Prim::I64, 2, &[&["App", "x"]], 1),
                deferred(&["App", "g"], Prim::I64, 2, &[&["App", "f"]], 1),
            ],
        )]);
        let provider = StubProvider::new().with(2, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "x"], Value::I64(5));
        engine.commit();
        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "f"]), "6");
        assert_eq!(read(&snapshot, &["App", "g"]), "7");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.x 0 -> 5\npublish\ncommit_end\n\
                 commit_begin\ndirty App.f 1 -> 6\npublish\ncommit_end\n\
                 commit_begin\ndirty App.g 2 -> 7\npublish\ncommit_end\n"
            )
        );
    }

    // §6.2's witness 9 — C-23's construction evaluation. A deferred has no
    // `init` clause and none is added: its startup value comes from a
    // SYNCHRONOUS evaluation like a derived's (016-298), which is why the cell
    // reads 5 rather than the allocation placeholder 0 or its input's 4. The
    // first commit does not re-run it — the trace's opening block is the empty
    // one every trace opens with, and nothing is in flight.
    #[test]
    fn a_deferred_takes_its_startup_value_from_construction() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("4")),
                deferred(&["App", "later"], Prim::I64, 2, &[&["App", "x"]], 1),
            ],
        )]);
        let provider = StubProvider::new().with(2, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "later"]), "5");
        assert!(!engine.has_staged_work());

        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "later"]), "5");
        assert!(!engine.has_staged_work());

        engine.teardown();

        assert_eq!(sink.trace(), format!("{STARTUP}{STARTUP}"));
    }

    // §6.2's witnesses 10 and 11 — R74a's dispatch POSITION, and a shape that
    // still discriminates it under the advance-root law.
    //
    // A deferred's provenance asks a RECURRENT input for its `advanced` bit —
    // "it advanced to new bits in THIS commit" — so the advance and the dispatch
    // that reads it are one commit, and the walk is:
    //
    //   commit 1 (tick 0 -> 3): total = sum(3, past1=0) = 3, advanced; step 4½
    //                           reads pending total = 3 (POST-advance) and
    //                           dispatches increment(3) = 4, due at commit 2.
    //                           Dispatch before step 4 would have read 0 and
    //                           produced 1 — `echo`'s own value, a landing that
    //                           would not even show. That is the
    //                           discrimination.
    //   commit 2 (tick 3 -> 5): `echo` lands 4 as a dirty root; total =
    //                           sum(5, past1=3) = 8, advanced, and step 4½
    //                           dispatches increment(8) = 9 for commit 3.
    //   commit 3 (quiescent):   `echo` lands 9. `total` is a silent advance root
    //                           here but does not advance — nothing it reads is
    //                           dirty — so nothing dispatches, and the graph
    //                           goes quiet with every advance captured exactly
    //                           once.
    //
    // And `publish` is still the last value-bearing line of every block: the
    // dispatch between step 4 and step 5 emits nothing, because it publishes
    // nothing.
    #[test]
    fn a_dispatch_reads_the_advanced_values_and_leaves_publish_last() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "tick"], Prim::I64, int("0")),
                recurrent(
                    &["App", "total"],
                    Prim::I64,
                    2,
                    &[&["App", "tick"]],
                    1,
                    int("0"),
                ),
                deferred(&["App", "echo"], Prim::I64, 3, &[&["App", "total"]], 1),
            ],
        )]);
        let provider = StubProvider::new().with(2, sum()).with(3, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "tick"], Value::I64(3));
        engine.commit();
        write(&mut engine, &["App", "tick"], Value::I64(5));
        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "total"]), "8");
        assert_eq!(read(&snapshot, &["App", "echo"]), "9");

        engine.teardown();

        for block in sink.trace().split_terminator("commit_end\n") {
            assert!(block.ends_with("publish\n"), "{block}");
        }

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.tick 0 -> 3\neval App.total -> 3\n\
                 publish\ncommit_end\n\
                 commit_begin\ndirty App.tick 3 -> 5\ndirty App.echo 1 -> 4\n\
                 eval App.total -> 8\npublish\ncommit_end\n\
                 commit_begin\ndirty App.echo 4 -> 9\npublish\ncommit_end\n"
            )
        );
    }

    // The regression the `advanced`-bit provenance is FOR: a deferred reading a
    // recurrent carries every advance, exactly once, in advance order.
    //
    // Asking a recurrent input for its `dirty` bit instead loses values and
    // invents them. The bit is set one commit after the advance (§4.12), while
    // step 4½'s arguments come from this commit's — so a run of consecutive
    // advances would be dispatched one commit late, over the NEXT advance's
    // value (`p` would carry 4 and 7 and never 2), and a commit that rooted but
    // did not advance would dispatch a fourth run over a value that had not
    // moved.
    //
    // `r` is `sum`, so it accumulates: 1, 3, 6 across the three writes and then
    // sits still. `p` is `increment` over it, one commit behind by its own
    // latency: 2, 4, 7 — one landing per advance, no fourth.
    #[test]
    fn a_deferred_reading_a_recurrent_carries_every_advance_exactly_once() {
        let witness = module(vec![scope(
            &["App"],
            vec![
                input(&["App", "x"], Prim::I64, int("0")),
                recurrent(&["App", "r"], Prim::I64, 2, &[&["App", "x"]], 1, int("0")),
                deferred(&["App", "p"], Prim::I64, 3, &[&["App", "r"]], 1),
            ],
        )]);
        let provider = StubProvider::new().with(2, sum()).with(3, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        for x in 1..=3 {
            write(&mut engine, &["App", "x"], Value::I64(x));
            engine.commit();
        }

        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "r"]), "6");
        assert_eq!(read(&snapshot, &["App", "p"]), "7");

        engine.teardown();

        let landings: Vec<&str> = sink
            .trace()
            .lines()
            .filter(|line| line.starts_with("dirty App.p"))
            .collect();

        assert_eq!(
            landings,
            [
                "dirty App.p 1 -> 2",
                "dirty App.p 2 -> 4",
                "dirty App.p 4 -> 7"
            ],
            "{}",
            sink.trace()
        );

        // And the graph goes QUIET: the fifth commit has nothing to land and
        // nothing to dispatch, so it is the empty block idempotence promises.
        assert!(sink.trace().ends_with(STARTUP), "{}", sink.trace());
    }

    // A deferred is COMPUTED, so the host cannot write it — the same refusal a
    // derived and a recurrent get (§13.14.2), and the cell's value cell being
    // an ordinary slot of the buffers does not make it a signal.
    #[test]
    fn a_host_write_to_a_deferred_is_refused() {
        let witness = delayed(1);
        let provider = StubProvider::new().with(2, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        let refused = engine
            .write_signal(&path(&["App", "later"]), Value::I64(9))
            .expect_err("a deferred cell");

        assert_eq!(refused.class, DiagClass::WriteToNonInput);
        assert_eq!(
            refused.path.as_ref().map(CellPath::render),
            Some("App.later".to_string())
        );

        engine.commit();
        engine.teardown();

        assert_eq!(sink.trace(), format!("{STARTUP}{STARTUP}"));
    }

    // RW-8's honest half: a run held at teardown is simply DROPPED. Its
    // behavior already ran, so there is nothing to cancel and nothing to await;
    // the value is never published and the trace ends where the last commit
    // left it.
    #[test]
    fn teardown_drops_a_run_that_never_landed() {
        let witness = delayed(2);
        let provider = StubProvider::new().with(2, increment());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "x"], Value::I64(5));
        engine.commit();

        assert!(engine.has_staged_work());
        assert_eq!(read(&engine.acquire_snapshot(), &["App", "later"]), "1");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!("{STARTUP}commit_begin\ndirty App.x 0 -> 5\npublish\ncommit_end\n")
        );
    }

    fn bool_of(value: &Value) -> bool {
        match value {
            Value::Bool(payload) => *payload,
            other => panic!("a bool argument, got {other:?}"),
        }
    }

    /// Answers its single `bool` argument — the predicate stub most gate
    /// witnesses use, so the host opens and closes a gate by writing one input
    /// cell.
    fn follows() -> Behavior {
        Rc::new(|inputs| Value::Bool(bool_of(&inputs[0])))
    }

    /// Whether its single `i64` argument is non-zero — a predicate whose INPUT
    /// can change without its answer changing, which is the only way to
    /// witness "a dependency moved, the boolean did not".
    fn nonzero() -> Behavior {
        Rc::new(|inputs| Value::Bool(i64_of(&inputs[0]) != 0))
    }

    /// A gated derived over a live input: `App.d` recomputes `App.x` while
    /// `App.g` is open and freezes while it is closed, and `App.x` is inside
    /// the guarded set so its liveness under a closed gate is witnessed too.
    ///
    /// `open` is the gate's STARTING state, which the predicate reads from
    /// `App.on`.
    fn gated_derived(open: bool) -> Module {
        let start = if open {
            ValueLiteral::True
        } else {
            ValueLiteral::False
        };

        module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(input(&["App", "on"], Prim::Bool, start)),
                Entry::Cell(gated(
                    input(&["App", "x"], Prim::I64, int("2")),
                    &["App", "g"],
                )),
                Entry::Cell(gated(
                    derived(&["App", "d"], Prim::I64, 1, &[&["App", "x"]]),
                    &["App", "g"],
                )),
                Entry::Gate(gate(
                    &["App", "g"],
                    2,
                    &[&["App", "on"]],
                    &[&["App", "x"], &["App", "d"]],
                )),
            ],
        )])
    }

    fn gate_provider() -> StubProvider {
        StubProvider::new().with(1, sum()).with(2, follows())
    }

    // §6.3's witness 1 — closed at startup (§13.2.6 / 016-144, RW-7). Gates
    // gate propagation and never existence, so the subtree CONSTRUCTS and then
    // freezes: `App.d` holds the value it really computed, that value is
    // readable, and no `eval` follows. The trace is the empty startup block —
    // a gate that begins closed had no TRANSITION to record.
    #[test]
    fn a_gate_closed_at_startup_freezes_a_constructed_subtree() {
        let witness = gated_derived(false);
        let provider = gate_provider();
        let mut sink = RecordingSink::new();
        let engine = Engine::start(loaded(&witness, &provider), &mut sink);
        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "d"]), "2");
        assert_eq!(read(&snapshot, &["App", "x"]), "2");

        engine.teardown();

        assert_eq!(sink.trace(), STARTUP);
    }

    // §6.3's witnesses 2 and 5, and the whole of the freeze law in one
    // sequence: an input inside a closed gate STAYS LIVE (the write stages and
    // dirties), the derived above it does NOT recompute, and on the effective
    // open it evaluates in that same commit — forced stale, reading CURRENT
    // values (022-59), which is why it lands on 5 rather than on the 2 it
    // froze at.
    #[test]
    fn a_gate_open_snaps_the_subtree_to_current_values() {
        let witness = gated_derived(false);
        let provider = gate_provider();
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "x"], Value::I64(5));
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "d"]), "2");

        write(&mut engine, &["App", "on"], Value::Bool(true));
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "d"]), "2");

        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "d"]), "5");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.x 2 -> 5\npublish\ncommit_end\n\
                 commit_begin\ndirty App.on false -> true\npublish\ncommit_end\n\
                 commit_begin\ngate App.g open\neval App.d -> 5\npublish\ncommit_end\n"
            )
        );
    }

    // §6.3's witness 4 — R73's literal prior-commit reading, and this test is
    // its regression guard: it FAILS under any other reading of fork 3.
    //
    // The write that flips the predicate publishes at commit N. Step 1a reads
    // the LIVE buffer at commit start, which at N still holds the pre-write
    // value, so the flip and its snap happen at N+1. Both halves are asserted:
    // no `gate` line in N's block, and the line plus the snap in N+1's.
    #[test]
    fn a_predicate_sees_a_write_one_commit_after_it_publishes() {
        let witness = gated_derived(false);
        let provider = gate_provider();
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "on"], Value::Bool(true));
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "on"]), "true");

        engine.commit();
        engine.teardown();

        // The write's own commit block carries the `dirty` line and NOTHING
        // else — no `gate` line, no snap. The transition is in the block after
        // it, which is the whole of the ruling.
        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.on false -> true\npublish\ncommit_end\n\
                 commit_begin\ngate App.g open\neval App.d -> 2\npublish\ncommit_end\n"
            )
        );
    }

    // §6.3's witness 3 — the close edge. The subtree's last values freeze and
    // stay readable (§13.9.7: "reads return frozen values"), and a write
    // arriving afterwards dirties its input without reaching the derived.
    #[test]
    fn a_gate_close_freezes_the_subtree_and_reads_still_answer() {
        let witness = gated_derived(true);
        let provider = gate_provider();
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "on"], Value::Bool(false));
        engine.commit();
        engine.commit();

        write(&mut engine, &["App", "x"], Value::I64(9));
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "x"]), "9");
        assert_eq!(read(&snapshot, &["App", "d"]), "2");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.on true -> false\npublish\ncommit_end\n\
                 commit_begin\ngate App.g close\npublish\ncommit_end\n\
                 commit_begin\ndirty App.x 2 -> 9\npublish\ncommit_end\n"
            )
        );
    }

    /// Two gates, `App.inner` nested under `App.outer` — written from the
    /// OUTSIDE, with the outer gate naming the inner one in its `guards`,
    /// which is the same nesting `gate_parent App.outer` spells from within.
    fn nested_gates() -> Module {
        module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(input(&["App", "outer_on"], Prim::Bool, ValueLiteral::True)),
                Entry::Cell(input(&["App", "inner_on"], Prim::Bool, ValueLiteral::True)),
                Entry::Cell(input(&["App", "x"], Prim::I64, int("1"))),
                Entry::Cell(gated(
                    derived(&["App", "d"], Prim::I64, 1, &[&["App", "x"]]),
                    &["App", "inner"],
                )),
                Entry::Gate(gate(
                    &["App", "outer"],
                    2,
                    &[&["App", "outer_on"]],
                    &[&["App", "inner"]],
                )),
                Entry::Gate(nested(
                    gate(
                        &["App", "inner"],
                        2,
                        &[&["App", "inner_on"]],
                        &[&["App", "d"]],
                    ),
                    &["App", "outer"],
                )),
            ],
        )])
    }

    // §6.3's witness 6 — nesting, and the C-39/C-40 asymmetry it exists to
    // pin. Four separate claims, in one sequence:
    //
    // 1. closing the ANCESTOR closes the descendant EFFECTIVELY, so both lines
    //    appear — in entry rank order, outer before inner;
    // 2. the inner gate's own predicate keeps evaluating under the closed
    //    ancestor and its own state updates, but nothing traces, because the
    //    effective value did not move;
    // 3. reopening the ancestor restores each descendant to ITS OWN state —
    //    `App.inner` stays closed, `App.d` stays frozen, and no replay
    //    happens;
    // 4. only when the inner gate's own state flips back does the subtree
    //    reopen and snap, to the value written during the gap.
    #[test]
    fn a_closed_ancestor_freezes_a_nested_subtree_and_reopening_replays_nothing() {
        let witness = nested_gates();
        let provider = StubProvider::new().with(1, increment()).with(2, follows());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "outer_on"], Value::Bool(false));
        engine.commit();
        engine.commit();

        write(&mut engine, &["App", "inner_on"], Value::Bool(false));
        engine.commit();
        engine.commit();

        write(&mut engine, &["App", "x"], Value::I64(5));
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "d"]), "2");

        write(&mut engine, &["App", "outer_on"], Value::Bool(true));
        engine.commit();
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "d"]), "2");

        write(&mut engine, &["App", "inner_on"], Value::Bool(true));
        engine.commit();
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "d"]), "6");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.outer_on true -> false\npublish\ncommit_end\n\
                 commit_begin\ngate App.outer close\ngate App.inner close\npublish\ncommit_end\n\
                 commit_begin\ndirty App.inner_on true -> false\npublish\ncommit_end\n\
                 {STARTUP}\
                 commit_begin\ndirty App.x 1 -> 5\npublish\ncommit_end\n\
                 commit_begin\ndirty App.outer_on false -> true\npublish\ncommit_end\n\
                 commit_begin\ngate App.outer open\npublish\ncommit_end\n\
                 commit_begin\ndirty App.inner_on false -> true\npublish\ncommit_end\n\
                 commit_begin\ngate App.inner open\neval App.d -> 6\npublish\ncommit_end\n"
            )
        );
    }

    /// A gated `recurrent` over a live input — the graph §6.3's witness 7
    /// needs, and the one whose `reset_on_reopen` variant is witness 8's.
    fn gated_recurrent(resets: bool, seed: &str) -> Module {
        let mut cell = gated(
            recurrent(&["App", "r"], Prim::I64, 1, &[&["App", "x"]], 1, int(seed)),
            &["App", "g"],
        );
        cell.reset_on_reopen = resets;

        module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(input(&["App", "on"], Prim::Bool, ValueLiteral::True)),
                Entry::Cell(input(&["App", "x"], Prim::I64, int("1"))),
                Entry::Cell(cell),
                Entry::Gate(gate(&["App", "g"], 2, &[&["App", "on"]], &[&["App", "r"]])),
            ],
        )])
    }

    // §6.3's witness 7 — the LOSS witness, the single most important
    // interaction in cut B. §13.9.7 in its own words: "Any input change that
    // would have triggered re-evaluation during a gated period is LOST — the
    // runtime does not queue triggers, and gate-open does not replay them."
    //
    // So this asserts three things a weaker implementation would get wrong.
    // The write during the gap dirties its input and the recurrent does not
    // advance. The REOPEN does not resume it — a recurrent is not snapped,
    // which is exactly where it differs from a derived. And the pre-gap value
    // holds until a NEW trigger arrives, at which point `past(1)` reads the
    // value from before the gap, because the gap consumed no ring slot (R72).
    #[test]
    fn a_frozen_recurrent_loses_its_triggers_and_reopening_does_not_replay_them() {
        let witness = gated_recurrent(false, "0");
        let provider = gate_provider();
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "r"]), "1");

        write(&mut engine, &["App", "on"], Value::Bool(false));
        engine.commit();
        engine.commit();

        write(&mut engine, &["App", "x"], Value::I64(4));
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "r"]), "1");

        write(&mut engine, &["App", "on"], Value::Bool(true));
        engine.commit();
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "r"]), "1");

        write(&mut engine, &["App", "x"], Value::I64(6));
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "r"]), "7");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.on true -> false\npublish\ncommit_end\n\
                 commit_begin\ngate App.g close\npublish\ncommit_end\n\
                 commit_begin\ndirty App.x 1 -> 4\npublish\ncommit_end\n\
                 commit_begin\ndirty App.on false -> true\npublish\ncommit_end\n\
                 commit_begin\ngate App.g open\npublish\ncommit_end\n\
                 commit_begin\ndirty App.x 4 -> 6\neval App.r -> 7\npublish\ncommit_end\n"
            )
        );
    }

    // §6.3's witness 8 — `@reset_on_reopen` at its ruled position (C-13). The
    // history clears in STEP 1a, before step 3's reads, so a recurrent that
    // evaluates in the very commit its gate reopens already reads FALLBACKS.
    //
    // The seed makes the position discriminating rather than merely asserted:
    // with the clear, `past(1)` is the 100 fallback and the answer is 103;
    // without it, `past(1)` is the 101 the cell advanced to at construction and
    // the answer is 104. The companion test below pins that other number.
    #[test]
    fn reset_on_reopen_clears_history_before_the_reopening_commit_reads_it() {
        let witness = gated_recurrent(true, "100");
        let provider = gate_provider();
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "r"]), "101");

        write(&mut engine, &["App", "on"], Value::Bool(false));
        engine.commit();
        engine.commit();

        write(&mut engine, &["App", "on"], Value::Bool(true));
        engine.commit();

        write(&mut engine, &["App", "x"], Value::I64(3));
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "r"]), "103");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.on true -> false\npublish\ncommit_end\n\
                 commit_begin\ngate App.g close\npublish\ncommit_end\n\
                 commit_begin\ndirty App.on false -> true\npublish\ncommit_end\n\
                 commit_begin\ndirty App.x 1 -> 3\ngate App.g open\neval App.r -> 103\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // The same sequence WITHOUT the flag: the ring survives the gap, so the
    // post-gap evaluation reads the value from before it. This is what makes
    // the test above an assertion about `reset_on_reopen` rather than about
    // gates.
    #[test]
    fn a_reopen_without_the_flag_keeps_the_history() {
        let witness = gated_recurrent(false, "100");
        let provider = gate_provider();
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "on"], Value::Bool(false));
        engine.commit();
        engine.commit();

        write(&mut engine, &["App", "on"], Value::Bool(true));
        engine.commit();

        write(&mut engine, &["App", "x"], Value::I64(3));
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "r"]), "104");

        engine.teardown();
    }

    /// A gated `deferred[2]` — §6.3's witness 9.
    fn gated_deferred() -> Module {
        module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(input(&["App", "on"], Prim::Bool, ValueLiteral::True)),
                Entry::Cell(input(&["App", "x"], Prim::I64, int("1"))),
                Entry::Cell(gated(
                    deferred(&["App", "f"], Prim::I64, 1, &[&["App", "x"]], 2),
                    &["App", "g"],
                )),
                Entry::Gate(gate(&["App", "g"], 2, &[&["App", "on"]], &[&["App", "f"]])),
            ],
        )])
    }

    // §6.3's witness 9 — gate × deferred, both halves. §13.9.7: "Deferreds do
    // not dispatch; gate-close CANCELS any in-flight runs — no publication, no
    // barrier", and the gate-open snap "dispatches a fresh run rather than
    // evaluating synchronously".
    //
    // Cancellation is asserted three ways: the run due at the closing commit
    // does not land, `has_staged_work` goes false across the window, and the
    // value the cancelled run carried never appears. The fresh run then lands
    // exactly `depth` commits after the reopen, as a `dirty` root (R75).
    #[test]
    fn a_gate_close_cancels_an_in_flight_run_and_a_gate_open_dispatches_a_fresh_one() {
        let witness = gated_deferred();
        let provider = StubProvider::new().with(1, increment()).with(2, follows());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "x"], Value::I64(5));
        engine.commit();

        assert!(engine.has_staged_work());

        write(&mut engine, &["App", "on"], Value::Bool(false));
        engine.commit();

        assert!(engine.has_staged_work());

        engine.commit();

        assert!(!engine.has_staged_work());
        assert_eq!(read(&engine.acquire_snapshot(), &["App", "f"]), "2");

        write(&mut engine, &["App", "on"], Value::Bool(true));
        engine.commit();
        engine.commit();

        assert!(engine.has_staged_work());

        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "f"]), "2");

        engine.commit();

        assert!(!engine.has_staged_work());
        assert_eq!(read(&engine.acquire_snapshot(), &["App", "f"]), "6");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.x 1 -> 5\npublish\ncommit_end\n\
                 commit_begin\ndirty App.on true -> false\npublish\ncommit_end\n\
                 commit_begin\ngate App.g close\npublish\ncommit_end\n\
                 commit_begin\ndirty App.on false -> true\npublish\ncommit_end\n\
                 commit_begin\ngate App.g open\npublish\ncommit_end\n\
                 {STARTUP}\
                 commit_begin\ndirty App.f 2 -> 6\npublish\ncommit_end\n"
            )
        );
    }

    // §6.3's witness 10 — a predicate's own dependency moves and its answer
    // does not. Nothing transitions, so there is no `gate` line and no snap:
    // the trace's second block records the write and stops.
    //
    // It is also the C-50 witness from the other side. The predicate DOES run
    // this commit — every predicate runs every commit, ungated by any dirtiness
    // test — and running it is unobservable, which is the whole justification
    // for not gating it.
    #[test]
    fn a_predicate_input_that_does_not_flip_the_boolean_transitions_nothing() {
        let witness = module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(input(&["App", "k"], Prim::I64, int("1"))),
                Entry::Cell(input(&["App", "x"], Prim::I64, int("2"))),
                Entry::Cell(gated(
                    derived(&["App", "d"], Prim::I64, 1, &[&["App", "x"]]),
                    &["App", "g"],
                )),
                Entry::Gate(gate(&["App", "g"], 2, &[&["App", "k"]], &[&["App", "d"]])),
            ],
        )]);
        let provider = StubProvider::new().with(1, increment()).with(2, nonzero());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "k"], Value::I64(2));
        engine.commit();
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.k 1 -> 2\npublish\ncommit_end\n\
                 {STARTUP}"
            )
        );
    }

    // The SELF-CONDITIONAL gate — SPEC §13.9.6, legalized by the owner's
    // 2026-08-24 ruling (a) on R73's rider. `App.g`'s predicate reads `App.d`,
    // which `App.g` guards. It LOADS: the load-time rejection and its
    // `gate_predicate_reads_guarded` class are deleted, and no special handling
    // replaces them, because step 1a already defines the read as a PRIOR-COMMIT
    // one. What that buys is a transition that trails its own guarded change by
    // exactly one commit:
    //
    //   startup:                d = increment(n = 1) = 2; nonzero(2) is true, so
    //                           the gate begins OPEN and records no line.
    //   commit 1 (n 1 -> -1):   the predicate reads the PRIOR d = 2 and holds
    //                           open, so `App.d` evaluates to 0 under it.
    //   commit 2 (quiescent):   the predicate now reads d = 0 and closes — one
    //                           commit after the change that closed it.
    //   commit 3 (quiescent):   `App.d` is frozen at 0, so the predicate answers
    //                           false again and nothing transitions. A gate that
    //                           decides its own subtree latches.
    #[test]
    fn a_self_conditional_gate_trails_its_own_guarded_change_by_one_commit() {
        let witness = module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(input(&["App", "n"], Prim::I64, int("1"))),
                Entry::Cell(gated(
                    derived(&["App", "d"], Prim::I64, 1, &[&["App", "n"]]),
                    &["App", "g"],
                )),
                Entry::Gate(gate(&["App", "g"], 2, &[&["App", "d"]], &[&["App", "d"]])),
            ],
        )]);
        let provider = StubProvider::new().with(1, increment()).with(2, nonzero());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "n"], Value::I64(-1));
        engine.commit();
        engine.commit();
        engine.commit();

        assert_eq!(read(&engine.acquire_snapshot(), &["App", "d"]), "0");

        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.n 1 -> -1\neval App.d -> 0\n\
                 publish\ncommit_end\n\
                 commit_begin\ngate App.g close\npublish\ncommit_end\n\
                 {STARTUP}"
            )
        );
    }

    // §6.3's witness 11 — R76's whole claim in one commit block. A host write,
    // a gate transition and a deferred landing arrive at interleaved entry
    // ranks, and the block's root lines are ONE rank sequence: `App.a` (rank
    // 0), `App.g` (rank 1), `App.f` (rank 2).
    //
    // The point is what a reader CANNOT tell from it. The landing renders as a
    // `dirty` line indistinguishable from the host's write, because a
    // downstream consumer cannot distinguish them either, and the trace is the
    // consumer's view (R24, R75).
    #[test]
    fn one_commit_folds_a_write_a_transition_and_a_landing_into_one_rank_sequence() {
        let witness = module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(input(&["App", "a"], Prim::I64, int("0"))),
                Entry::Gate(gate(&["App", "g"], 2, &[&["App", "k"]], &[])),
                Entry::Cell(deferred(&["App", "f"], Prim::I64, 1, &[&["App", "b"]], 1)),
                Entry::Cell(input(&["App", "b"], Prim::I64, int("0"))),
                Entry::Cell(input(&["App", "k"], Prim::I64, int("0"))),
            ],
        )]);
        let provider = StubProvider::new().with(1, increment()).with(2, nonzero());
        let mut sink = RecordingSink::new();
        let mut engine = Engine::start(loaded(&witness, &provider), &mut sink);

        write(&mut engine, &["App", "b"], Value::I64(7));
        write(&mut engine, &["App", "k"], Value::I64(3));
        engine.commit();

        write(&mut engine, &["App", "a"], Value::I64(5));
        engine.commit();
        engine.teardown();

        assert_eq!(
            sink.trace(),
            format!(
                "{STARTUP}\
                 commit_begin\ndirty App.b 0 -> 7\ndirty App.k 0 -> 3\npublish\ncommit_end\n\
                 commit_begin\ndirty App.a 0 -> 5\ngate App.g open\ndirty App.f 1 -> 8\n\
                 publish\ncommit_end\n"
            )
        );
    }

    // §13.9.4: "The predicate must have type `bool`. A non-bool predicate is a
    // compile error." The loader cannot see it — a behavior is an opaque
    // closure across the provider seam (D5) — so it surfaces where every
    // wrong-typed stub surfaces: a trap naming the construct (D8), the gate
    // here rather than a cell.
    #[test]
    #[should_panic(expected = "predicate for gate `App.g` produced a `i64`")]
    fn a_non_bool_predicate_traps_naming_its_gate() {
        let witness = gated_derived(true);
        let provider = StubProvider::new()
            .with(1, sum())
            .with(2, constant(Value::I64(1)));
        let mut sink = RecordingSink::new();

        Engine::start(loaded(&witness, &provider), &mut sink);
    }

    // And a predicate that traps carries the GATE's path, through the same
    // single call path a cell's behavior goes through.
    #[test]
    #[should_panic(expected = "behavior trap at App.g: predicate exploded")]
    fn a_trapping_predicate_names_its_gate() {
        let witness = gated_derived(true);
        let provider = StubProvider::new()
            .with(1, sum())
            .with(2, Rc::new(|_| panic!("predicate exploded")));
        let mut sink = RecordingSink::new();

        Engine::start(loaded(&witness, &provider), &mut sink);
    }
}
