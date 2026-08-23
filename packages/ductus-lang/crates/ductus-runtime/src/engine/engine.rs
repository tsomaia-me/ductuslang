//! The `Engine` struct: the write→commit→read loop, and §13.10.2's six steps.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use ductus_ir::diag::Diagnostic;
use ductus_ir::path::CellPath;

use super::transaction::Transaction;
use super::write_check::write_check;
use crate::loader::{CellIndex, CellRole, LoadedCell, LoadedGraph};
use crate::store::{CellStore, Snapshot};
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
///   born at step 1, read and extended at step 3, and cleared at step 6, so it
///   never outlives a commit; nothing reads it between commits.
/// - `sink` — the trace seam.
///
/// M0 is strictly sequential: there is one driving context (§13.14.2), it is
/// the caller's thread, and `&mut self` is what says so.
pub struct Engine<'s> {
    graph: LoadedGraph,
    store: CellStore,
    dirty: Vec<bool>,
    sink: &'s mut dyn TraceSink,
}

impl<'s> Engine<'s> {
    /// Starts the runtime on a loaded graph — §13.14.1 startup steps 2–4
    /// (step 1, loading the IR, is [`load`](crate::loader::load())'s).
    ///
    /// **Step 2 — allocate cell storage.** One slot per cell in each buffer,
    /// sized once and never resized (§13.12.4's fixed-extent cost shape).
    /// Inputs start at their converted `init`; a derived's slot starts at a
    /// typed placeholder that step 3 overwrites before anything can read it —
    /// every derived is in `static_order`, so no placeholder survives
    /// construction.
    ///
    /// **Step 3 — initialize the cells.** Every derived evaluates ONCE, in
    /// `static_order`, reading the live buffer, and its result is seeded into
    /// both committed buffers and the staged slot. This is CONSTRUCTION, not a
    /// commit: the runtime is "constructing" through step 3 and only "live"
    /// after step 4, there is no previous snapshot to preserve and nothing to
    /// publish — so it is UNTRACED. The trace records commits (R24), and
    /// construction is not one. A trap during construction aborts exactly as a
    /// trap during a commit does, because both go through the one invoke path
    /// below.
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
        let initial = graph
            .cells
            .iter()
            .map(|cell| match &cell.role {
                CellRole::Input { init } => *init,
                CellRole::Derived { .. } => placeholder(cell.value_type),
            })
            .collect();

        let dirty = vec![false; graph.cells.len()];
        let mut engine = Engine {
            graph,
            store: CellStore::new(initial),
            dirty,
            sink,
        };

        engine.construct();
        engine.commit();

        engine
    }

    /// §13.14.1 step 3 — the construction pass, in `static_order` so a derived
    /// reading another derived reads a value already computed.
    fn construct(&mut self) {
        let Engine { graph, store, .. } = self;

        for &cell_index in &graph.static_order {
            let value = invoke(graph, store, cell_index, ArgumentSource::Live);

            store.seed(cell_index, value);
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
    /// functions in step order, including the thin step 2 and the empty step 4,
    /// so the spec section and this function read as one list. Nothing else
    /// happens here; the trace's `commit_begin`/`commit_end` bracket the six.
    ///
    /// A commit with nothing dirty still runs all six and still publishes:
    /// §13.14.4's idempotence — a fresh snapshot whose values are identical.
    pub fn commit(&mut self) {
        let Engine {
            graph,
            store,
            dirty,
            sink,
        } = self;

        sink.event(&TraceEvent::CommitBegin);

        dirty_set(graph, store, dirty, *sink);

        let order = evaluation_order(graph);

        evaluate(graph, order, store, dirty, *sink);
        advance_recurrents();
        publish(store, *sink);
        clear_dirty(dirty);

        sink.event(&TraceEvent::CommitEnd);
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
    /// In the M0 slice that sequence collapses to almost nothing: there are no
    /// effects to deliver reconciler `teardown` to, no in-flight commit to
    /// drain (commits are synchronous and this call needs `self` by value, so
    /// none can be running), no wires or pools, and no string pool. What
    /// remains is "release cell storage" — which is what consuming `self` does.
    /// The verb exists because it is mandatory core (§13.14.11) and because M1+
    /// grows into it; writing it as an empty body now is what keeps the host's
    /// shutdown call site unchanged when it stops being empty.
    ///
    /// Consuming `self` also ends the sink borrow, which is how a host gets its
    /// trace sink back to inspect.
    pub fn teardown(self) {}
}

/// §13.10.2 **step 1 — compute the dirty set.**
///
/// Refills the pending buffer from live, then walks every cell in RANK order
/// and dirties the INPUT cells whose staged value differs from their
/// previously-committed one, copying each dirtied value into pending and
/// emitting its `dirty` line.
///
/// Three laws are carried here, and each is a line of code:
///
/// - The comparison is [`Value::bits_eq`] — bitwise on the stored
///   representation, not IEEE equality (R65): an unchanged NaN bit pattern is
///   CLEAN, and a `0.0` overwritten with `-0.0` is DIRTY.
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
    sink: &mut dyn TraceSink,
) {
    store.begin_pending();

    for (rank, cell) in graph.cells.iter().enumerate() {
        let index = CellIndex(rank);
        let staged = store.staged(index);
        let committed = store.live()[rank];
        let changed = matches!(cell.role, CellRole::Input { .. }) && !staged.bits_eq(&committed);

        if changed {
            dirty[rank] = true;

            store.set_pending(index, staged);
            sink.event(&TraceEvent::Dirty {
                path: cell.path.clone(),
                old: committed,
                new: staged,
            });
        }
    }
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
/// walks this order and skips what is not dirty, which is the same schedule the
/// spec's per-commit sort would produce.
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
/// **The result.** It is type-checked and invoked through the one invoke path
/// below, its `eval` line is emitted (only for evaluations that ACTUALLY
/// happen — a skipped node emits nothing, per R24's no-suppressed-evals rule),
/// its dirty bit is set to whether the value CHANGED, and it is written to
/// pending. The dirty bit is where value-change gating on outputs lives
/// (§13.2.4.4, R65): a derived that re-evaluates to a bit-identical value marks
/// itself clean, so the deriveds downstream of it see no dirty dependency and
/// do not evaluate at all.
fn evaluate(
    graph: &LoadedGraph,
    order: &[CellIndex],
    store: &mut CellStore,
    dirty: &mut [bool],
    sink: &mut dyn TraceSink,
) {
    for &cell_index in order {
        let cell = &graph.cells[cell_index.0];
        let stale = dependencies(cell).iter().any(|input| dirty[input.0]);

        if stale {
            let value = invoke(graph, store, cell_index, ArgumentSource::Pending);
            let committed = store.live()[cell_index.0];

            sink.event(&TraceEvent::Eval {
                path: cell.path.clone(),
                value,
            });

            dirty[cell_index.0] = !value.bits_eq(&committed);

            store.set_pending(cell_index, value);
        }
    }
}

/// §13.10.2 **step 4 — advance recurrents.** Empty in M0.
///
/// There are no recurrent cells in the M0 slice (R1 puts them in M1), so there
/// is nothing to advance and the step has no body. It is kept as a named step
/// under R23: the six-step correspondence is the audit surface, and M1's work —
/// writing step 3's held-aside recurrent results into their cells, after which
/// recurrent reads return the advanced values — lands HERE, between evaluation
/// and publication, which is the only position that makes lockstep true.
fn advance_recurrents() {}

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
/// # The `catch_unwind` is a STAMP, not a boundary
///
/// §13.13.1 is explicit: the runtime does not isolate traps, there is no
/// errored-cell sentinel, no `catch_unwind` boundary, and no continuation past
/// a trap. This wrapper does not create one. It catches the unwind and
/// IMMEDIATELY re-panics — nothing runs in between, no value is invented, no
/// state is repaired, and control never returns to the commit. What it buys is
/// the trapping cell's PATH in the message, which is the M0 trap shape the
/// owner ruled (D8: a harness-visible panic carrying the cell path). The
/// tension with §13.13.1's wording is real and is filed as FND-24
/// (`docs/implementation/FINDINGS.md`), awaiting the owner's ruling; the
/// reading applied here is that the prohibition is on ISOLATION — surviving a
/// trap — not on observing one in flight to name it.
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

    let CellRole::Derived { behavior, inputs } = &cell.role else {
        panic!(
            "only derived cells evaluate; `{}` is an input",
            cell.path.render()
        );
    };

    let arguments: Vec<Value> = inputs
        .iter()
        .map(|&input| match source {
            ArgumentSource::Live => store.live()[input.0],
            ArgumentSource::Pending => store.pending(input),
        })
        .collect();

    let outcome = catch_unwind(AssertUnwindSafe(|| behavior(&arguments)));

    match outcome {
        Ok(value) => checked(cell, value),
        Err(payload) => trap(&cell.path, payload),
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

/// The cells a cell reads — its dependency edges for step 3's staleness test.
///
/// An input has none: it is written, never computed, and the static order does
/// not carry it.
fn dependencies(cell: &LoadedCell) -> &[CellIndex] {
    match &cell.role {
        CellRole::Input { .. } => &[],
        CellRole::Derived { inputs, .. } => inputs,
    }
}

/// A derived cell's slot value before construction computes it (§13.14.1
/// step 2).
///
/// Never observable: every derived is in `static_order`, so step 3 overwrites
/// every placeholder before step 4's commit publishes anything. It exists
/// because the store is allocated to its final size in one shot — the
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
    use ductus_ir::graph::ValueLiteral;
    use ductus_ir::module::Module;
    use ductus_ir::types::Prim;

    use super::super::test_support::{
        RecordingSink, STARTUP, StubProvider, derived, input, loaded, module, path, scope,
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
}
