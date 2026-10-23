//! Witness tests for `deferred` — the M1 latency family (M1.md §6.2,
//! corpus/README.md, R25).
//!
//! One `#[test]` fn per witness, named after its `.duir` file stem, so a witness
//! is grep-able by one word across `corpus/duir/m1/` and the tests.
//!
//! Each witness makes five claims, in order:
//!
//! 1. the `.duir` source lexes, parses and emits back to its own bytes — the
//!    emitter IS the canonicalizer (R54), so byte equality is also the claim
//!    that the hand-written file is canonical;
//! 2. the module loads under the stubs the witness registers;
//! 3. a scripted sequence of writes and commits drives it;
//! 4. the trace heard is BYTE-identical to the `.trace` golden;
//! 5. the SAME script over the same module built programmatically through the
//!    `ductus-ir` Rust API — no text anywhere — produces a byte-identical trace
//!    (M1.md §6.6's dual-construction track).
//!
//! # Every golden here is HAND-DERIVED
//!
//! Not one of these traces was captured from a run. Each is written out from the
//! semantics contract — §13.10.2's steps as pinned in M1.md §4, with step 4½ the
//! one M1 adds — and the derivation is the test fn's doc comment, block by
//! block. A golden captured from the engine would assert only that the engine
//! does what the engine does.
//!
//! # A deferred's commit, in the four facts every derivation below uses
//!
//! - **It LANDS at step 1b.** A run due at this commit is a dirty ROOT, folded
//!   into the one rank-ordered walk beside the host writes (R76), and rendered
//!   with the same `dirty <path> <old> -> <new>` line they get — a landing has
//!   no event of its own, because a downstream consumer cannot tell a landing
//!   from a write and the trace is the consumer's view (R24, R75).
//! - **It DISPATCHES at step 4½** — after step 4's recurrent advance and before
//!   step 5's flip (R74a) — so `publish` stays the last value-bearing line of
//!   every block, and the values a dispatch reads are exactly the ones the
//!   commit is about to publish. A dispatch produces NO line: it publishes no
//!   value.
//! - **Its body is never a step-2 node.** A deferred is absent from the static
//!   order, so nothing about it appears as an `eval`; a read OF one is not a
//!   scheduling edge either, because the landing already put the value in
//!   pending back at step 1.
//! - **Its startup value is a synchronous construction evaluation** (C-23,
//!   016-298), untraced like every other construction. It is visible in these
//!   goldens as the `old` half of the cell's first landing line.
//!
//! Provenance is per-input DIRTINESS (C-28): a deferred dispatches iff one of
//! its input cells' dirty bits is set this commit, which is bitwise for free
//! (R65) because a dirty bit already is. A landing is gated the same way at the
//! other end — a run landing the value the cell already holds is CONSUMED but
//! emits nothing and propagates nothing.
//!
//! # The sequential-rendering note (M1.md §6.2, witness 12)
//!
//! M1's core is sequential (M0.md R1): *"evaluate at dispatch, hold, publish at
//! dispatch+N — the as-if law makes the sequential rendering exact."* Every
//! behavior below therefore runs SYNCHRONOUSLY at its dispatch, over the values
//! that commit is about to publish, and what waits `depth` commits is the
//! PUBLICATION alone. §13.10.2's publication barrier — *"if the run due at this
//! commit is unfinished, the commit waits for it"* — is vacuous here, because a
//! run due at commit N finished at commit N-depth. These goldens are what a
//! threaded core must reproduce byte for byte when one arrives: the as-if law
//! makes them the oracle, not merely this rendering's output.

use std::rc::Rc;

use ductus_conformance::module_builder::{cell, module, path, scope};
use ductus_conformance::runner::{Witness, parsed, traced};
use ductus_conformance::stub_library::{add1_i64, registry_of, sum_i64};
use ductus_conformance::trace_compare::assert_trace;
use ductus_ir::graph::{Cell, CellKind, Entry, ValueLiteral};
use ductus_ir::module::Module;
use ductus_ir::types::Prim;
use ductus_runtime::engine::Engine;
use ductus_runtime::provider::Behavior;
use ductus_runtime::value::Value;

/// A loaded witness pair out of `corpus/duir/m1/` — the M1 milestone directory
/// (corpus/README.md), read by the same manifest-anchored law as M0's.
fn witness(name: &str) -> Witness {
    Witness::load_milestone("m1", name)
}

/// Stages a write, failing the test with the path when the engine refuses it.
fn write(engine: &mut Engine<'_>, names: &[&str], value: i64) {
    engine
        .write_signal(&path(names), Value::I64(value))
        .unwrap_or_else(|error| {
            panic!(
                "`{}` is a legal write: {}",
                path(names).render(),
                error.message
            )
        });
}

/// The canonically-rendered committed value at `names` — what a trace line
/// would carry for that cell, asked directly.
///
/// The in-flight witnesses need it because the claim they make is about a value
/// NOT changing, and an unchanged value emits no trace line at all: the absence
/// of a line says a landing did not happen, and this says what the cell served
/// instead.
fn reads(engine: &Engine<'_>, names: &[&str]) -> String {
    engine
        .acquire_snapshot()
        .read(&path(names))
        .expect("the cell exists")
        .render()
}

fn input(names: &[&str], init: &str) -> Cell {
    let mut built = cell(CellKind::Input, names, Prim::I64);
    built.init = Some(ValueLiteral::int(init.to_string()));

    built
}

fn derived(names: &[&str], uses: u32, inputs: &[&[&str]]) -> Cell {
    let mut built = cell(CellKind::Derived, names, Prim::I64);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();

    built
}

/// A `deferred` cell: a behavior, its inputs, and the latency in commits.
///
/// `depth` is not optional the way a recurrent's is (C-24): 033-245 renders it
/// unconditionally, and a deferred line that omits it has not said how long the
/// delay is. There is no `init` clause and none is added — the startup value
/// comes from construction (C-23).
fn deferred(names: &[&str], uses: u32, inputs: &[&[&str]], depth: u64) -> Cell {
    let mut built = cell(CellKind::Deferred, names, Prim::I64);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();
    built.depth = Some(depth);

    built
}

/// The one-scope module every witness here builds: scope `App` holding `cells`
/// in the order given, which is the entry order R64 ranks by.
fn app(cells: Vec<Cell>) -> Module {
    module(vec![scope(
        &["App"],
        cells.into_iter().map(Entry::Cell).collect(),
    )])
}

/// M1.md §6.2 witnesses 1, 9, 10 and 11: the round trip of a `depth 1`
/// deferred — dispatch at one commit, landing at the next — with the startup
/// value, the readiness surface and the trace shape read off the same run.
///
/// `App.late` is `App.x + 1`, so its value is never mistakable for the input's
/// and a landing that carried the wrong number could not still read correct.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction evaluates `App.late` ONCE, synchronously,
/// exactly as it evaluates a derived (C-23): `0 + 1 = 1`. It does NOT dispatch —
/// the latency governs commits and construction is not one — so the value the
/// cell serves until its first landing is this `1`. Construction is untraced,
/// and step 4's first commit finds `App.x`'s staged value equal to its committed
/// one, so the block is `commit_begin`/`publish`/`commit_end`.
///
/// **Block 2 (commit 1).** The write stages `5`, which differs bitwise from the
/// committed `0`, so `App.x` dirties `0 -> 5`. Nothing else happens at step 1b:
/// no run is due, because none has ever been dispatched. Step 3 evaluates
/// nothing — the static order is EMPTY here, `App.late`'s body being no node of
/// it. Then step 4½: `App.late`'s provenance is dirty (C-28 — `App.x`'s bit is
/// set), so its behavior runs over the PENDING `5`, giving `6`, and the engine
/// holds `(6, due = 1 + 1 = 2)`. That dispatch emits NOTHING: it published no
/// value. The block is one `dirty` line and a `publish`.
///
/// **Block 3 (commit 2).** No write, so `App.x` is clean. `App.late`'s run is due
/// at exactly this commit, so it LANDS as a dirty root at step 1b: the committed
/// `1` — construction's value — against the arriving `6`, giving
/// `dirty App.late 1 -> 6`. That line is the whole witness: a `dirty` line for a
/// cell the host never wrote, in the same rank walk a written cell's line would
/// occupy (R75, R76). Step 4½ dispatches nothing, because `App.x` did not dirty.
///
/// **Block 4 (commit 3).** Empty. A landing happens exactly once — the run was
/// CONSUMED at block 3 — so there is nothing left to arrive, and the absence of
/// a second `dirty App.late` line says so.
///
/// The readiness surface (R74b) is asserted around the sequence rather than by
/// counting commits: false before the dispatch, true across the flight, false
/// once the value has landed.
#[test]
fn latency1() {
    let witness = witness("latency1");
    let registry = registry_of(&[(0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        // Construction's value, before any commit of the script: C-23.
        assert_eq!(reads(engine, &["App", "late"]), "1");
        assert!(!engine.has_staged_work(), "nothing has dispatched yet");

        write(engine, &["App", "x"], 5);
        engine.commit();

        assert!(engine.has_staged_work(), "the run dispatched at commit 1");
        assert_eq!(reads(engine, &["App", "late"]), "1");

        engine.commit();

        assert!(!engine.has_staged_work(), "the run landed at commit 2");
        assert_eq!(reads(engine, &["App", "late"]), "6");

        engine.commit();
    };

    let from_text = traced(&parsed("latency1", &witness.source), &registry, script);

    assert_trace("latency1", &from_text, &witness.golden);

    // A dispatch publishes no value, so it leaves NO line (R74a/R75): the only
    // `App.late` line in the whole trace is its one landing.
    assert_eq!(from_text.matches("App.late").count(), 1);

    let from_api = traced(&built_latency1(), &registry, script);

    assert_trace("latency1 (constructed)", &from_api, &witness.golden);
}

/// `latency1.duir`, built through the `ductus-ir` API instead of parsed.
fn built_latency1() -> Module {
    app(vec![
        input(&["App", "x"], "0"),
        deferred(&["App", "late"], 0xd1, &[&["App", "x"]], 1),
    ])
}

/// M1.md §6.2 witness 2: `depth 3` — the value crosses exactly three commit
/// boundaries, and nothing lands early.
///
/// The two empty blocks in the middle are the assertion. A latency that leaked
/// by one commit would put `dirty App.slow` into one of them, and a latency that
/// collapsed would put it into block 2 beside the write.
///
/// # Derivation
///
/// **Block 1 (startup).** `App.slow = 0 + 1 = 1` at construction, untraced;
/// nothing is dirty at step 4's commit.
///
/// **Block 2 (commit 1).** `App.x` dirties `0 -> 9`. Step 4½ dispatches over the
/// pending `9` → `10`, held with `due = 1 + 3 = 4`.
///
/// **Blocks 3 and 4 (commits 2 and 3).** Nothing is staged and no run is due —
/// `landed` matches on the commit index alone, and neither 2 nor 3 is 4 — so
/// step 1b finds no root at all. Both blocks are empty, and both still
/// `publish`: §13.14.4's idempotence is unchanged by a run being in flight.
///
/// **Block 5 (commit 4).** The run is due. It lands as a dirty root:
/// `dirty App.slow 1 -> 10`. `App.x` is clean, so nothing dispatches, and the
/// flight is over.
#[test]
fn latency3() {
    let witness = witness("latency3");
    let registry = registry_of(&[(0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], 9);
        engine.commit();

        // Held across every commit of the flight, and only across them.
        assert!(engine.has_staged_work(), "dispatched at commit 1");

        engine.commit();

        assert!(engine.has_staged_work(), "still in flight at commit 2");
        assert_eq!(reads(engine, &["App", "slow"]), "1");

        engine.commit();

        assert!(engine.has_staged_work(), "still in flight at commit 3");
        assert_eq!(reads(engine, &["App", "slow"]), "1");

        engine.commit();

        assert!(!engine.has_staged_work(), "landed at commit 4");
        assert_eq!(reads(engine, &["App", "slow"]), "10");
    };

    let from_text = traced(&parsed("latency3", &witness.source), &registry, script);

    assert_trace("latency3", &from_text, &witness.golden);

    let from_api = traced(&built_latency3(), &registry, script);

    assert_trace("latency3 (constructed)", &from_api, &witness.golden);
}

/// `latency3.duir`, built through the `ductus-ir` API instead of parsed.
fn built_latency3() -> Module {
    app(vec![
        input(&["App", "x"], "0"),
        deferred(&["App", "slow"], 0xd1, &[&["App", "x"]], 3),
    ])
}

/// M1.md §6.2 witness 3: what the cell — and everything reading it — serves
/// while a run is in flight.
///
/// `App.mirror` is an ordinary derived over the deferred, and it is the reason
/// the claim is checkable at all: a deferred in flight emits no line, so the
/// trace's evidence is an ABSENCE, and the snapshot reads below say what the
/// absence means. `App.late = App.x + 1`, `App.mirror = App.late + 1`.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction walks its own order, in which a read of a
/// deferred IS an edge (C-23: a deferred's startup value comes from a
/// synchronous evaluation, so whoever reads it must follow it): `App.late` first,
/// `0 + 1 = 1`, then `App.mirror`, `1 + 1 = 2`. Untraced; step 4 finds nothing
/// dirty.
///
/// **Block 2 (commit 1).** `App.x` dirties `0 -> 5`. Step 3 walks the static
/// order, which holds `App.mirror` alone — a deferred's body is never a step-2
/// node, and a read of one is not a scheduling edge either. `App.mirror`'s one
/// dependency is `App.late`, whose bit is CLEAN, so it does not evaluate: the
/// block carries no `eval` line. Step 4½ dispatches `App.late` over the pending
/// `5` → `6`, held with `due = 1 + 2 = 3`.
///
/// **Block 3 (commit 2).** Empty. Nothing is staged and no run is due. This is
/// the block the witness exists for: `App.late` still reads `1` and
/// `App.mirror` still reads `2`, because `begin_pending` copied live into
/// pending and NOTHING overwrote the deferred's slot. A cell with a run in
/// flight is not a cell in a special state — it is an ordinary cell holding its
/// last published value.
///
/// **Block 4 (commit 3).** The run lands: `dirty App.late 1 -> 6` at step 1b.
/// Now `App.mirror` has a dirty dependency and evaluates over the PENDING `6`
/// the landing just wrote — never the committed `1` — giving
/// `eval App.mirror -> 7`. The landing and the propagation are one commit's work,
/// which is what makes a landing indistinguishable from a host write to the
/// consumer.
#[test]
fn in_flight_reads() {
    let witness = witness("in_flight_reads");
    let registry = registry_of(&[(0xd1, add1_i64()), (0xd2, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], 5);
        engine.commit();

        // Commit 1: dispatched, nothing published for it.
        assert_eq!(reads(engine, &["App", "late"]), "1");
        assert_eq!(reads(engine, &["App", "mirror"]), "2");

        engine.commit();

        // Commit 2: still in flight, and the last published value still stands.
        assert!(engine.has_staged_work(), "still in flight at commit 2");
        assert_eq!(reads(engine, &["App", "late"]), "1");
        assert_eq!(reads(engine, &["App", "mirror"]), "2");

        engine.commit();

        assert!(!engine.has_staged_work(), "landed at commit 3");
        assert_eq!(reads(engine, &["App", "late"]), "6");
        assert_eq!(reads(engine, &["App", "mirror"]), "7");
    };

    let from_text = traced(
        &parsed("in_flight_reads", &witness.source),
        &registry,
        script,
    );

    assert_trace("in_flight_reads", &from_text, &witness.golden);

    // The consumer evaluates ONCE, at the landing — never during the flight.
    assert_eq!(from_text.matches("eval App.mirror").count(), 1);

    let from_api = traced(&built_in_flight_reads(), &registry, script);

    assert_trace("in_flight_reads (constructed)", &from_api, &witness.golden);
}

/// `in_flight_reads.duir`, built through the `ductus-ir` API instead of parsed.
fn built_in_flight_reads() -> Module {
    app(vec![
        input(&["App", "x"], "0"),
        deferred(&["App", "late"], 0xd1, &[&["App", "x"]], 2),
        derived(&["App", "mirror"], 0xd2, &[&["App", "late"]]),
    ])
}

/// M1.md §6.2 witnesses 4 and 6, and R76 itself: a commit whose dirty roots are
/// a landing, a host write and a second landing — at INTERLEAVED ranks, in ONE
/// rank sequence.
///
/// The declaration order is the discriminating shape. `App.p` (rank 1) and
/// `App.q` (rank 3) are deferreds; `App.b` (rank 2) is an input sitting BETWEEN
/// them. So the middle commit's roots cannot come out right by concatenating
/// "the writes" and then "the landings", or the other way round: only one walk
/// in entry-rank order puts a landing, a write and a landing in that order.
///
/// `App.p = App.a + 1`, `App.q = App.b + 1`, `App.total = App.p + App.q`.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction's order is `App.p`, `App.q`, `App.total`
/// — the deferreds are ready at once and tie-break by rank, and `App.total`
/// waits for both. `p = 0 + 1 = 1`, `q = 0 + 1 = 1`, `total = 2`. Untraced;
/// step 4 finds nothing dirty.
///
/// **Block 2 (commit 1).** Both inputs are written, so both dirty, in rank
/// order: `App.a 0 -> 1` then `App.b 0 -> 2`. `App.total`'s dependencies are the
/// two DEFERREDS, whose bits are clean, so it does not evaluate. Step 4½ walks
/// the deferreds in rank order and dispatches both — `p` over the pending `1` →
/// `2`, `q` over the pending `2` → `3` — each held with `due = 1 + 1 = 2`.
///
/// **Block 3 (commit 2).** The script writes `App.b = 5` and both runs come due,
/// so step 1b's single walk visits, in entry rank:
///
/// - rank 0, `App.a`: staged `1` equals committed `1` → clean, no line;
/// - rank 1, `App.p`: a run is due → `dirty App.p 1 -> 2`;
/// - rank 2, `App.b`: staged `5` against committed `2` → `dirty App.b 2 -> 5`;
/// - rank 3, `App.q`: a run is due → `dirty App.q 1 -> 3`.
///
/// A reader cannot tell which of those three lines the host caused, and that is
/// the ruling (R75/R76): a downstream consumer cannot tell either. Step 3 then
/// evaluates `App.total` ONCE over both pending values → `2 + 3 = 5`; two dirty
/// dependencies still mean one evaluation, because step 3 walks cells, not
/// edges. Step 4½ dispatches `App.q` alone — `App.b`'s bit is set, `App.a`'s is
/// not (C-28) — over the pending `5` → `6`, `due = 2 + 1 = 3`.
///
/// **Block 4 (commit 3).** `App.q` lands `3 -> 6`. `App.p` is clean and its
/// pending slot still holds the committed `2`, so `App.total` re-evaluates over
/// `2 + 6 = 8` — a landing propagating through a join with a value from an
/// earlier commit, glitch-free.
#[test]
fn landing_dirty_root() {
    let witness = witness("landing_dirty_root");
    let registry = registry_of(&[(0xd1, add1_i64()), (0xd2, add1_i64()), (0xd3, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "a"], 1);
        write(engine, &["App", "b"], 2);
        engine.commit();

        write(engine, &["App", "b"], 5);
        engine.commit();

        assert_eq!(reads(engine, &["App", "total"]), "5");

        engine.commit();

        assert_eq!(reads(engine, &["App", "total"]), "8");
        assert!(!engine.has_staged_work(), "every run has landed");
    };

    let from_text = traced(
        &parsed("landing_dirty_root", &witness.source),
        &registry,
        script,
    );

    assert_trace("landing_dirty_root", &from_text, &witness.golden);

    // The discriminating fact on its own: the host write at rank 2 sits BETWEEN
    // the two landings, so the roots are one rank sequence and not two lists.
    let first = from_text.find("dirty App.p").expect("`App.p` lands");
    let host = from_text
        .find("dirty App.b 2 -> 5")
        .expect("`App.b` is written");
    let second = from_text.find("dirty App.q 1 -> 3").expect("`App.q` lands");

    assert!(first < host && host < second, "{from_text}");

    let from_api = traced(&built_landing_dirty_root(), &registry, script);

    assert_trace(
        "landing_dirty_root (constructed)",
        &from_api,
        &witness.golden,
    );
}

/// `landing_dirty_root.duir`, built through the `ductus-ir` API instead of
/// parsed.
fn built_landing_dirty_root() -> Module {
    app(vec![
        input(&["App", "a"], "0"),
        deferred(&["App", "p"], 0xd1, &[&["App", "a"]], 1),
        input(&["App", "b"], "0"),
        deferred(&["App", "q"], 0xd2, &[&["App", "b"]], 1),
        derived(&["App", "total"], 0xd3, &[&["App", "p"], &["App", "q"]]),
    ])
}

/// M1.md §6.2 witness 5 — 023-64's pipeline: *"Up to `N` runs of a
/// `deferred[N]` cell's one behavior may be in flight at once — a pipeline
/// yielding full-rate output at N-commit latency."*
///
/// `depth 2` and a write at each of three consecutive commits. The claim is
/// FULL RATE: from the moment the pipe fills, the cell lands at EVERY commit,
/// two behind, one value per commit and none merged or dropped. The three input
/// values are decades apart so each landing names the commit it came from.
///
/// # Derivation
///
/// **Block 1 (startup).** `App.pipe = 0 + 1 = 1` at construction. Untraced.
///
/// **Block 2 (commit 1).** `App.x` dirties `0 -> 10`; dispatch → `11`, due 3.
/// One run in flight.
///
/// **Block 3 (commit 2).** `App.x` dirties `10 -> 20`; nothing is due yet;
/// dispatch → `21`, due 4. TWO runs in flight — the depth is the ceiling, and
/// this is the commit that reaches it.
///
/// **Block 4 (commit 3).** Two roots, in rank order: the write dirties
/// `App.x 20 -> 30` at rank 0, and the run due at 3 lands at rank 1 —
/// `dirty App.pipe 1 -> 11`. The pipe is now full and stays full: dispatch → `31`,
/// due 5. The `1` in that landing line is still construction's value, which is
/// what the cell served for the two commits the first run was in flight.
///
/// **Blocks 5 and 6 (commits 4 and 5).** No writes, so nothing dispatches — but
/// a run lands at each: `11 -> 21`, then `21 -> 31`. Output at full rate from
/// input that stopped two commits ago; the flight, not the input, is what sets
/// the cadence.
///
/// **Block 7 (commit 6).** Empty. The pipe has drained, and `has_staged_work` is
/// false for the first time since block 2.
#[test]
fn pipeline_full_rate() {
    let witness = witness("pipeline_full_rate");
    let registry = registry_of(&[(0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], 10);
        engine.commit();
        write(engine, &["App", "x"], 20);
        engine.commit();
        write(engine, &["App", "x"], 30);
        engine.commit();

        // Commit 3 landed the first run while two more were already held.
        assert_eq!(reads(engine, &["App", "pipe"]), "11");
        assert!(engine.has_staged_work(), "two runs are still in flight");

        engine.commit();

        assert_eq!(reads(engine, &["App", "pipe"]), "21");
        assert!(engine.has_staged_work(), "one run is still in flight");

        engine.commit();

        assert_eq!(reads(engine, &["App", "pipe"]), "31");
        assert!(!engine.has_staged_work(), "the pipe has drained");

        engine.commit();
    };

    let from_text = traced(
        &parsed("pipeline_full_rate", &witness.source),
        &registry,
        script,
    );

    assert_trace("pipeline_full_rate", &from_text, &witness.golden);

    // Three writes in, three landings out: nothing was merged and nothing was
    // dropped, which is what "full rate" means.
    assert_eq!(from_text.matches("dirty App.pipe").count(), 3);

    let from_api = traced(&built_pipeline_full_rate(), &registry, script);

    assert_trace(
        "pipeline_full_rate (constructed)",
        &from_api,
        &witness.golden,
    );
}

/// `pipeline_full_rate.duir`, built through the `ductus-ir` API instead of
/// parsed.
fn built_pipeline_full_rate() -> Module {
    app(vec![
        input(&["App", "x"], "0"),
        deferred(&["App", "pipe"], 0xd1, &[&["App", "x"]], 2),
    ])
}

/// `|x|` over one `i64` — a SINGLE-USE stub, written here rather than in the
/// shared library so the reason `landing_gating`'s golden holds sits beside the
/// golden.
///
/// It is the smallest behavior that maps two different inputs to one output,
/// which is what makes an equal-valued LANDING observable at all: without it a
/// changed input always produces a changed landing, and the gating question
/// never arises.
///
/// # Panics
///
/// On any other argument shape — a harness-author bug, loud rather than papered
/// over. `wrapping_abs` keeps `i64::MIN` total, for the reason the library stubs
/// give: a fixture must not trap by accident.
fn abs_i64() -> Behavior {
    Rc::new(|inputs| match inputs {
        [Value::I64(payload)] => Value::I64(payload.wrapping_abs()),
        other => panic!("abs_i64 wants one i64 input, got {other:?}"),
    })
}

/// M1.md §6.2 witness 7, landing half: a run that lands the value the cell
/// ALREADY HOLDS is consumed, emits nothing, and propagates nothing.
///
/// Landing and propagating are two questions, and this witness separates them.
/// The run really arrives — `has_staged_work` goes false, so the hold was
/// dropped — but step 1b's comparison is [`Value::bits_eq`] against the
/// committed value (R65), and a bit-identical arrival is not a dirty root. No
/// `dirty` line, no dirty bit, and `App.report` never sees a reason to
/// re-evaluate.
///
/// `App.mag = |App.x|`, `App.report = App.mag + 1`.
///
/// # Derivation
///
/// **Block 1 (startup).** `mag = |3| = 3`, then `report = 3 + 1 = 4`. Untraced.
///
/// **Block 2 (commit 1).** The write stages `-3`, which differs bitwise from the
/// committed `3`, so `App.x` dirties `3 -> -3`. `App.report`'s dependency is
/// `App.mag`, clean, so it does not evaluate. Step 4½ dispatches over the pending
/// `-3` → `|-3| = 3`, held with `due = 2`.
///
/// **Block 3 (commit 2).** The run is due and IS taken — the hold list is
/// emptied — but the arriving `3` is bit-identical to the committed `3`, so no
/// root is recorded: the block is EMPTY. That empty block is the whole witness.
/// A trace records changes (R24), and nothing changed, even though a value
/// really did arrive.
///
/// **Block 4 (commit 3).** `App.x` dirties `-3 -> -8`; dispatch → `8`, due 4.
///
/// **Block 5 (commit 4).** Now the arriving `8` differs from the committed `3`,
/// so it lands as a root — `dirty App.mag 3 -> 8` — and `App.report` evaluates
/// over the pending `8` → `9`. The machinery was never asleep in block 3; the
/// value simply had not changed.
#[test]
fn landing_gating() {
    let witness = witness("landing_gating");
    let registry = registry_of(&[(0xd1, abs_i64()), (0xd2, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], -3);
        engine.commit();

        assert!(engine.has_staged_work(), "dispatched at commit 1");

        engine.commit();

        // The run LANDED — the hold is gone — and still changed nothing.
        assert!(
            !engine.has_staged_work(),
            "the equal-valued run was consumed"
        );
        assert_eq!(reads(engine, &["App", "mag"]), "3");
        assert_eq!(reads(engine, &["App", "report"]), "4");

        write(engine, &["App", "x"], -8);
        engine.commit();
        engine.commit();

        assert_eq!(reads(engine, &["App", "mag"]), "8");
        assert_eq!(reads(engine, &["App", "report"]), "9");
    };

    let from_text = traced(
        &parsed("landing_gating", &witness.source),
        &registry,
        script,
    );

    assert_trace("landing_gating", &from_text, &witness.golden);

    // Two landings happened; exactly one of them was a dirty root, and the
    // consumer evaluated exactly once.
    assert_eq!(from_text.matches("dirty App.mag").count(), 1);
    assert_eq!(from_text.matches("eval App.report").count(), 1);

    let from_api = traced(&built_landing_gating(), &registry, script);

    assert_trace("landing_gating (constructed)", &from_api, &witness.golden);
}

/// `landing_gating.duir`, built through the `ductus-ir` API instead of parsed.
fn built_landing_gating() -> Module {
    app(vec![
        input(&["App", "x"], "3"),
        deferred(&["App", "mag"], 0xd1, &[&["App", "x"]], 1),
        derived(&["App", "report"], 0xd2, &[&["App", "mag"]]),
    ])
}

/// M1.md §6.2 witness 7, dispatch half: an unchanged provenance dispatches
/// NOTHING — C-28's per-input dirty-bit basis, in the two shapes that can make
/// a bit stay clear.
///
/// `App.y` exists to be written and ignored: it is a live input of the graph
/// that is NOT an input of the deferred, so a commit in which only `App.y`
/// dirties is a commit with a busy graph and an idle deferred. The second shape
/// is the equal re-write of `App.x`, which never dirties in the first place
/// (§13.10.1: staged against the previous COMMIT).
///
/// `App.pick = App.x + 1`.
///
/// # Derivation
///
/// **Block 1 (startup).** `App.pick = 0 + 1 = 1`. Untraced.
///
/// **Block 2 (commit 1).** `App.y` dirties `0 -> 5`. At step 4½ `App.pick`'s
/// provenance is `App.x` alone, whose bit is clear, so nothing dispatches — no
/// hold is taken, and `has_staged_work` stays false. Dispatch is per-INPUT, not
/// per-commit.
///
/// **Block 3 (commit 2).** Empty — and it is the proof of block 2. Had the write
/// to `App.y` dispatched anything, a `depth 1` run would land HERE. Nothing
/// arrives.
///
/// **Block 4 (commit 3).** `App.x` dirties `0 -> 5`; dispatch → `6`, due 4.
///
/// **Block 5 (commit 4).** The run lands: `dirty App.pick 1 -> 6`.
///
/// **Block 6 (commit 5).** `App.x` is written `5` again — the value it already
/// holds — so step 1 finds no difference against the previous commit and no bit
/// is set. Nothing dispatches. Writing a value twice is not writing twice.
///
/// **Block 7 (commit 6).** Empty, which is the proof of block 6 exactly as
/// block 3 was of block 2.
#[test]
fn dispatch_gating() {
    let witness = witness("dispatch_gating");
    let registry = registry_of(&[(0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "y"], 5);
        engine.commit();

        // A dirty graph and an idle deferred: the bit that matters is `App.x`'s.
        assert!(
            !engine.has_staged_work(),
            "a write to a cell the deferred does not read dispatched nothing"
        );

        engine.commit();
        write(engine, &["App", "x"], 5);
        engine.commit();

        assert!(engine.has_staged_work(), "dispatched at commit 3");

        engine.commit();

        assert!(!engine.has_staged_work(), "landed at commit 4");
        assert_eq!(reads(engine, &["App", "pick"]), "6");

        write(engine, &["App", "x"], 5);
        engine.commit();

        assert!(
            !engine.has_staged_work(),
            "an equal write dirties nothing, so it dispatches nothing"
        );

        engine.commit();
    };

    let from_text = traced(
        &parsed("dispatch_gating", &witness.source),
        &registry,
        script,
    );

    assert_trace("dispatch_gating", &from_text, &witness.golden);

    // Six commits, two of them with a dirty graph, and exactly one landing.
    assert_eq!(from_text.matches("dirty App.pick").count(), 1);

    let from_api = traced(&built_dispatch_gating(), &registry, script);

    assert_trace("dispatch_gating (constructed)", &from_api, &witness.golden);
}

/// `dispatch_gating.duir`, built through the `ductus-ir` API instead of parsed.
fn built_dispatch_gating() -> Module {
    app(vec![
        input(&["App", "x"], "0"),
        input(&["App", "y"], "0"),
        deferred(&["App", "pick"], 0xd1, &[&["App", "x"]], 1),
    ])
}

/// M1.md §6.2 witness 8 and C-29: a landing at commit N makes a downstream
/// deferred's provenance dirty at N's END, so it dispatches at N and lands at
/// N + its own depth. Latency composes ADDITIVELY.
///
/// The two ends of a deferred's life sit at opposite ends of a commit, and this
/// witness is what that buys: `App.first` lands at step 1b and `App.second`
/// dispatches at step 4½ of the SAME commit, reading the value the landing just
/// put into pending. Nothing waits a commit to notice.
///
/// `App.first = App.x + 1`, `App.second = App.first + 1`, `App.out =
/// App.second + 1`.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction follows the deferred edges: `first =
/// 0 + 1 = 1`, `second = 1 + 1 = 2`, `out = 2 + 1 = 3`. Untraced.
///
/// **Block 2 (commit 1).** `App.x` dirties `0 -> 5`. Step 4½ walks the deferreds
/// in rank order: `App.first`'s provenance is dirty → dispatch over the pending
/// `5` → `6`, due `1 + 1 = 2`. `App.second`'s provenance is `App.first`, whose
/// bit is CLEAR — the first run has not landed yet — so it does not dispatch.
/// The chain does not run ahead of itself.
///
/// **Block 3 (commit 2).** `App.first` lands: `dirty App.first 1 -> 6`, which
/// sets its dirty bit. `App.out` reads `App.second`, still clean, so no `eval`.
/// Then step 4½: `App.second`'s provenance IS dirty now (C-29 — a landing counts
/// as provenance-dirty at the same commit's end), so it dispatches over the
/// pending `6` → `7`, due `2 + 2 = 4`. This block is the ruling's regression
/// guard: a reading in which the landing were noticed only at the NEXT commit
/// would push every line below it one block later.
///
/// **Block 4 (commit 3).** Empty. The second run is mid-flight.
///
/// **Block 5 (commit 4).** `App.second` lands `2 -> 7`, and `App.out` evaluates
/// over the pending `7` → `8`. The write at commit 1 became visible at commit 4:
/// `1 + 2` commits of latency, added, with no cost for the join between them.
///
/// **Block 6 (commit 5).** Empty; the chain is drained.
#[test]
fn chained_deferred() {
    let witness = witness("chained_deferred");
    let registry = registry_of(&[(0xd1, add1_i64()), (0xd2, add1_i64()), (0xd3, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], 5);
        engine.commit();
        engine.commit();

        // Commit 2 landed the first and dispatched the second, in one commit.
        assert!(engine.has_staged_work(), "the second link is in flight");
        assert_eq!(reads(engine, &["App", "first"]), "6");
        assert_eq!(reads(engine, &["App", "second"]), "2");

        engine.commit();
        engine.commit();

        assert!(!engine.has_staged_work(), "the chain has drained");
        assert_eq!(reads(engine, &["App", "second"]), "7");
        assert_eq!(reads(engine, &["App", "out"]), "8");

        engine.commit();
    };

    let from_text = traced(
        &parsed("chained_deferred", &witness.source),
        &registry,
        script,
    );

    assert_trace("chained_deferred", &from_text, &witness.golden);

    // Additive latency, read off the trace rather than off the golden: the
    // number of commit blocks OPENED between the two landings is exactly
    // `App.second`'s own depth, so the second link cost its own two commits and
    // not one more for noticing the first (C-29).
    let first = from_text
        .find("dirty App.first")
        .expect("`App.first` lands");
    let second = from_text
        .find("dirty App.second")
        .expect("`App.second` lands");

    assert!(first < second, "{from_text}");
    assert_eq!(from_text[first..second].matches("commit_begin").count(), 2);
    assert_eq!(from_text.matches("eval App.out").count(), 1);

    let from_api = traced(&built_chained_deferred(), &registry, script);

    assert_trace("chained_deferred (constructed)", &from_api, &witness.golden);
}

/// `chained_deferred.duir`, built through the `ductus-ir` API instead of parsed.
fn built_chained_deferred() -> Module {
    app(vec![
        input(&["App", "x"], "0"),
        deferred(&["App", "first"], 0xd1, &[&["App", "x"]], 1),
        deferred(&["App", "second"], 0xd2, &[&["App", "first"]], 2),
        derived(&["App", "out"], 0xd3, &[&["App", "second"]]),
    ])
}
