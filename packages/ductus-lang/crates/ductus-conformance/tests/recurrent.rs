//! Witness tests for `recurrent` — the M1 history construct (M1.md §6.1,
//! corpus/README.md, R25).
//!
//! One `#[test]` fn per witness, named after its `.duir` file stem, so a witness
//! is grep-able by one word across `corpus/duir/m1/` and the tests.
//!
//! Each witness makes the same five claims the M0 commit witnesses make:
//!
//! 1. the `.duir` source lexes, parses and emits back to its own bytes — the
//!    emitter IS the canonicalizer (R54), so byte equality is also the claim
//!    that the hand-written file is canonical, which is where R71's two new
//!    clauses (`lookback`, `fallbacks`) earn their round-trip;
//! 2. the module loads under the stubs the witness registers;
//! 3. a scripted sequence of writes and commits drives it;
//! 4. the trace heard is BYTE-identical to the `.trace` golden;
//! 5. the SAME script over the same module built programmatically through the
//!    `ductus-ir` Rust API — no text anywhere — produces a byte-identical trace
//!    (M1.md §6.6: `lookback` and `fallbacks` are the first clauses whose
//!    canonical SPELLING is a function of their values, so this track earns its
//!    keep here).
//!
//! # Every golden here is HAND-DERIVED
//!
//! Not one of these traces was captured from a run. Each is written out from
//! M1.md §4's semantics contract — the commit cycle 1a/1b/2/3/4/4½/5/6 — and the
//! derivation is the test fn's doc comment, block by block: which cells dirty,
//! which cells evaluate, what each history read resolves to, and which slot each
//! advance consumes. A golden captured from the engine would assert only that
//! the engine does what the engine does.
//!
//! # The four shape rules every derivation below applies
//!
//! - A trace OPENS with the empty block `commit_begin`/`publish`/`commit_end` —
//!   §13.14.1 step 4's first commit, with construction (step 3) already done and
//!   untraced. A recurrent's construction turn is inside that untraced region
//!   and it is where the FALLBACKS are read: the cell evaluates against them and
//!   then advances, because an evaluation is what shifts a ring (R77).
//! - Inside a block: `dirty` lines for written INPUT cells whose staged value
//!   differs BITWISE from the committed one (R65), in RANK order (R64); then
//!   `eval` lines in static topological order with rank as the tiebreak. A
//!   recurrent's advance reuses `eval <path> -> <value>` — it IS the production
//!   of the cell's value, so no new event kind exists for it (R75).
//! - A recurrent is stale on exactly a derived's test — a VALUE input is dirty.
//!   Self-history and `lookback` reads are neither cycle edges nor staleness
//!   triggers (M1.md §4.2), so they never make a cell evaluate and never make
//!   one wait.
//! - Step 3 HOLDS a recurrent's result; step 4 advances the ring and writes
//!   pending. So anything reading a recurrent LATER IN THE SAME COMMIT sees the
//!   cell's PREVIOUS value — that is lockstep (§13.2.4.1), and it is visible in
//!   more than one golden here.

use std::rc::Rc;

use ductus_conformance::module_builder::{cell, module, path, scope};
use ductus_conformance::runner::{Witness, parsed, traced};
use ductus_conformance::stub_library::{add1_i64, registry_of, sum_i64};
use ductus_conformance::trace_compare::assert_trace;
use ductus_ir::graph::{Cell, CellKind, Entry, FallbackEntry, LookbackEntry, ValueLiteral};
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

/// The same, at `f64` — [`nan_history`] is the one witness whose claim is about
/// BITS, so it needs a float-typed input to write into.
fn write_float(engine: &mut Engine<'_>, names: &[&str], value: f64) {
    engine
        .write_signal(&path(names), Value::F64(value))
        .unwrap_or_else(|error| {
            panic!(
                "`{}` is a legal write: {}",
                path(names).render(),
                error.message
            )
        });
}

/// The canonically-rendered committed value at `names` — the read surface a
/// host has (§13.14.6), used where a claim is about a value the TRACE cannot
/// carry because construction is untraced.
fn committed(engine: &Engine<'_>, names: &[&str]) -> String {
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

/// A `recurrent` carrying the UNIFORM history seed — `init v`, R71's shorthand
/// for "every depth `k` falls back to `v`".
fn recurrent(names: &[&str], uses: u32, inputs: &[&[&str]], depth: u64, init: &str) -> Cell {
    let mut built = cell(CellKind::Recurrent, names, Prim::I64);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();
    built.depth = Some(depth);
    built.init = Some(ValueLiteral::int(init.to_string()));

    built
}

/// The same cell carrying PER-DEPTH seeds instead — `fallbacks [k: v, …]`,
/// R71's general form, in the map's written order.
fn seeded(names: &[&str], uses: u32, inputs: &[&[&str]], fallbacks: &[(u64, &str)]) -> Cell {
    let mut built = cell(CellKind::Recurrent, names, Prim::I64);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();
    built.depth = Some(fallbacks.len() as u64);
    built.fallbacks = fallbacks
        .iter()
        .map(|(k, digits)| FallbackEntry {
            k: *k,
            value: ValueLiteral::int((*digits).to_string()),
        })
        .collect();

    built
}

/// The `f64` pair of builders, for [`nan_history`] — the same two cells at the
/// one type whose values carry bits the trace can tell apart.
fn float_input(names: &[&str], init: &str) -> Cell {
    let mut built = cell(CellKind::Input, names, Prim::F64);
    built.init = Some(ValueLiteral::float(init.to_string()));

    built
}

fn float_recurrent(names: &[&str], uses: u32, inputs: &[&[&str]], depth: u64, init: &str) -> Cell {
    let mut built = cell(CellKind::Recurrent, names, Prim::F64);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();
    built.depth = Some(depth);
    built.init = Some(ValueLiteral::float(init.to_string()));

    built
}

/// One `lookback` entry — an input cell and the deepest `.past(k, …)` read of
/// it.
fn lookback(names: &[&str], k: u64) -> LookbackEntry {
    LookbackEntry {
        cell: path(names),
        k,
    }
}

/// The one-scope module every witness here builds: scope `App` holding `cells`
/// in the order given, which is the entry order R64 ranks by.
fn app(cells: Vec<Cell>) -> Module {
    module(vec![scope(
        &["App"],
        cells.into_iter().map(Entry::Cell).collect(),
    )])
}

/// The `i64` payload of an argument the ABI promised.
///
/// # Panics
///
/// On any other value type — a harness-author bug, loud rather than papered
/// over.
fn i64_of(value: &Value) -> i64 {
    match value {
        Value::I64(payload) => *payload,
        other => panic!("an i64 argument, got {other:?}"),
    }
}

/// The `f64` payload of an argument the ABI promised.
///
/// # Panics
///
/// On any other value type — a harness-author bug, loud rather than papered
/// over.
fn f64_of(value: &Value) -> f64 {
    match value {
        Value::F64(payload) => *payload,
        other => panic!("an f64 argument, got {other:?}"),
    }
}

/// `x, past1, past2 -> x + past2` — the SINGLE-USE stub of
/// [`equal_value_advance`], written here beside the golden it explains.
///
/// The unread middle argument is the point. Reading the OLDEST slot and
/// ignoring the newest is what makes a wasted ring slot visible: under the
/// rejected reading of R77 — where an evaluation producing an unchanged value
/// consumes no slot — the value this stub reads two commits later is a
/// different one, so the golden's arithmetic, not merely its line count,
/// discriminates the two readings.
///
/// # Panics
///
/// On any other argument shape — which for a `depth 2` recurrent with one input
/// means the ABI order (R71) is not what §4.9 pinned. The addition WRAPS, for
/// the reason the library stubs give: a fixture must not trap by accident.
fn plus_past2_i64() -> Behavior {
    Rc::new(|inputs| match inputs {
        [x, _past1, past2] => Value::I64(i64_of(x).wrapping_add(i64_of(past2))),
        other => panic!("plus_past2_i64 wants (x, past1, past2), got {other:?}"),
    })
}

/// `x, past1, x_past1, x_past2 -> 100·x + 10·x_past1 + x_past2` — the
/// SINGLE-USE stub of [`lookback_read`].
///
/// The weights make the ring READABLE: driven with single-digit inputs, the
/// value on each `eval` line spells the last three values of `App.x` as decimal
/// digits, newest first. A ring that shifted the wrong number of slots, or that
/// shifted on commits rather than on advances, changes a digit and the whole
/// number reads wrong.
///
/// It also pins the ABI ORDER (R71 §4.9), and the deliberately-unread `past1` is
/// how: were the `lookback` arguments to arrive BEFORE the self-history ones,
/// this stub would read `x_past2` where it expects `x_past1` and the
/// three-digit-number invariant would collapse into whatever the previous
/// three-digit result happened to be.
///
/// # Panics
///
/// On any other argument shape — arity here is `1 input + depth 1 + lookback 2`,
/// so a match failure IS the arity assertion.
fn window_i64() -> Behavior {
    Rc::new(|inputs| match inputs {
        [x, _past1, x_past1, x_past2] => Value::I64(
            i64_of(x)
                .wrapping_mul(100)
                .wrapping_add(i64_of(x_past1).wrapping_mul(10))
                .wrapping_add(i64_of(x_past2)),
        ),
        other => panic!("window_i64 wants (x, past1, x_past1, x_past2), got {other:?}"),
    })
}

/// `tick, past1, past2 -> past1 + past2` — the SINGLE-USE stub of
/// [`fallback_map`], and the corpus's own `fib` body.
///
/// The trigger argument is READ AND DISCARDED on purpose: a recurrent evaluates
/// because a VALUE input is dirty (M1.md §4.2), and this stub's value depends on
/// nothing but the two history slots, so every number in that witness's golden
/// comes from the ring and from the `fallbacks` map that seeded it.
///
/// # Panics
///
/// On any other argument shape. The addition WRAPS, for the reason the library
/// stubs give: a fixture must not trap by accident.
fn fib_step_i64() -> Behavior {
    Rc::new(|inputs| match inputs {
        [_tick, past1, past2] => Value::I64(i64_of(past1).wrapping_add(i64_of(past2))),
        other => panic!("fib_step_i64 wants (tick, past1, past2), got {other:?}"),
    })
}

/// `x, past1, past2 -> 1.0 when past2 is bit-for-bit NaN, else x` — the
/// SINGLE-USE stub of [`nan_history`].
///
/// The comparison is on BITS and not on `==`, which is the whole witness: IEEE
/// equality answers false for every NaN, so a stub written with `==` could never
/// tell a preserved NaN from a mangled one. `past2` is the slot that makes the
/// claim reach across two advances.
///
/// # Panics
///
/// On any other argument shape — which for a `depth 2` recurrent with one input
/// means the ABI order (R71) is not what §4.9 pinned.
fn nan_in_the_ring() -> Behavior {
    Rc::new(|inputs| match inputs {
        [x, _past1, past2] => {
            if f64_of(past2).to_bits() == f64::NAN.to_bits() {
                Value::F64(1.0)
            } else {
                Value::F64(f64_of(x))
            }
        }
        other => panic!("nan_in_the_ring wants (x, past1, past2), got {other:?}"),
    })
}

/// IR_GRAMMAR §5's own example — `recurrent total uses B@d2 inputs [App.count]
/// depth 1 init 0` — driven by three writes: the accumulator climbs by ONE
/// advance per commit, never by two.
///
/// `B@d1` is `sum_i64`, so the cell's body is literally `count + total.past(1)`
/// and every line of the golden is one addition a reader can check.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction evaluates `App.total` once, against the
/// FALLBACK — no advance exists yet — so `0 + 0 = 0`; the cell is seeded and
/// then ADVANCES, because an evaluation is what shifts a ring (R77). The ring
/// now holds `past(1) = 0`. All of that is untraced: construction is not a
/// commit. Step 4's first commit then finds `App.count`'s staged value equal to
/// its committed one, so the block is `commit_begin`/`publish`/`commit_end`.
///
/// **Block 2.** The write stages `5`, which differs bitwise from the committed
/// `0`, so `App.count` dirties. `App.total` reads a dirty VALUE input and is
/// stale on exactly a derived's test, so step 3 evaluates it: the input comes
/// from PENDING (`5`) and `past(1)` from the RING (`0`, construction's advance)
/// → `5`. The result is HELD; step 4 shifts the ring to `past(1) = 5` and writes
/// pending.
///
/// **Block 3.** `count` dirties `5 -> 7`; `total` reads `7 + past(1)` where
/// `past(1)` is the `5` step 4 wrote LAST commit — one advance ago — → `12`.
///
/// **Block 4.** `count` dirties `7 -> 2`; `total` reads `2 + 12` → `14`.
///
/// The three values `5`, `12`, `14` are the advance cadence in arithmetic: each
/// is this commit's input plus the PREVIOUS evaluation's result, and nothing
/// else. A ring that never shifted would leave `past(1)` on construction's `0`
/// and print `5`, `7`, `2`.
#[test]
fn depth1_accumulator() {
    let pair = witness("depth1_accumulator");
    let registry = registry_of(&[(0xd1, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "count"], 5);
        engine.commit();
        write(engine, &["App", "count"], 7);
        engine.commit();
        write(engine, &["App", "count"], 2);
        engine.commit();
    };

    let from_text = traced(
        &parsed("depth1_accumulator", &pair.source),
        &registry,
        script,
    );

    assert_trace("depth1_accumulator", &from_text, &pair.golden);

    let from_api = traced(&built_depth1_accumulator(), &registry, script);

    assert_trace("depth1_accumulator (constructed)", &from_api, &pair.golden);
}

/// `depth1_accumulator.duir`, built through the `ductus-ir` API instead of
/// parsed.
fn built_depth1_accumulator() -> Module {
    app(vec![
        input(&["App", "count"], "0"),
        recurrent(&["App", "total"], 0xd1, &[&["App", "count"]], 1, "0"),
    ])
}

/// What `.past(1)` reads before any advance exists: the FALLBACK, and nothing
/// else (M1.md §4.9).
///
/// The fallback is `100` and the input's `init` is `0`, so the constructed value
/// `100` can have come from ONE place. That value is asserted directly, through
/// the host read surface, because the read that proves it happens at
/// CONSTRUCTION and construction is untraced — the trace then shows the same
/// `100` riding the ring into the first traced evaluation.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction evaluates `App.total` with `App.x` at its
/// `init` `0` and `past(1)` at its FALLBACK `100` → `100`. Seed, then advance:
/// the ring holds `past(1) = 100`. Untraced. The first commit finds nothing
/// dirty and publishes the empty block — and the snapshot the script reads
/// before writing anything is exactly that publication, so `App.total` reads
/// `100`.
///
/// **Block 2.** `App.x` dirties `0 -> 1`. `App.total` evaluates over the pending
/// `1` and the ring's `100` → `101`. If a fallback were skipped and `past(1)`
/// answered with a zero, this line would read `1`.
///
/// **Block 3.** `App.x` dirties `1 -> 2`; `total` reads `2 + 101` → `103`. The
/// fallback is gone for good — one advance is all it takes to leave the
/// pre-history behind.
#[test]
fn first_evaluation() {
    let pair = witness("first_evaluation");
    let registry = registry_of(&[(0xd1, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        assert_eq!(
            committed(engine, &["App", "total"]),
            "100",
            "the construction evaluation read the fallback"
        );

        write(engine, &["App", "x"], 1);
        engine.commit();
        write(engine, &["App", "x"], 2);
        engine.commit();
    };

    let from_text = traced(&parsed("first_evaluation", &pair.source), &registry, script);

    assert_trace("first_evaluation", &from_text, &pair.golden);

    let from_api = traced(&built_first_evaluation(), &registry, script);

    assert_trace("first_evaluation (constructed)", &from_api, &pair.golden);
}

/// `first_evaluation.duir`, built through the `ductus-ir` API instead of parsed.
fn built_first_evaluation() -> Module {
    app(vec![
        input(&["App", "x"], "0"),
        recurrent(&["App", "total"], 0xd1, &[&["App", "x"]], 1, "100"),
    ])
}

/// Lockstep across a pair (023-35), and the DELAY CHAIN it produces: `App.b`
/// reads `App.a`, and a recurrent's advance reaches its readers at the NEXT
/// commit (§4.12) — so `b` advances exactly one commit behind `a` and never sees
/// a value `a` has not yet published.
///
/// Both handles resolve to `sum_i64`, so `a = x + a.past(1)` and
/// `b = a + b.past(1)`. The two fallbacks differ (`0` and `10`) so no number in
/// the golden can be confused for one produced by the other cell's history.
///
/// # Derivation
///
/// **Block 1 (startup).** `b` reads `a`, and a read of a recurrent ORDERS the
/// one construction/commit walk (R80 item 4), so `a` is constructed first —
/// which here is also rank order. `a = x(1) + fallback(0) = 1`, seeded and
/// advanced (`a.past(1) = 1`); then `b = a(1) + fallback(10) = 11`, seeded and
/// advanced (`b.past(1) = 11`). Construction's advance is NOT an advance root,
/// so the first commit still finds nothing dirty. Untraced.
///
/// **Block 2.** `App.x` dirties `1 -> 5`. Step 3 walks `a` then `b`:
///
/// - `a` reads pending `x = 5` and its ring's `1` → `6`. The result is HELD, so
///   NOTHING is written to `a`'s pending slot, and — the amended law — `a` sets
///   NO dirty bit;
/// - `b`'s only dependency is `a`, which is clean, so `b` DOES NOT EVALUATE.
///   The missing line is the assertion: under the wave-1 build `b` would have
///   evaluated here over `a`'s pre-advance `1` and printed `12`, and a `b`
///   declared BEFORE `a` would not have — a schedule that depended on
///   declaration order, which no reading of the corpus permits (FND-38).
///
/// Step 4 advances `a` to `6` and records the bits-changed advance.
///
/// **Block 3.** `App.x` dirties `5 -> 2`, and `a`'s advance is a SILENT dirty
/// root at its own rank:
///
/// - `a` reads `2 + 6` → `8`, held again;
/// - `b` is stale because `a` is a changed dependency, and reads `a` out of
///   PENDING — the `6` step 5 published last commit, not the `8` held aside this
///   one. With its own ring's `11` that gives `17`.
///
/// `17` is the whole claim. A `b` that saw `a`'s just-advanced `8` would print
/// `19`. `b` therefore trails `a` by exactly one commit forever, which is what
/// "no internal ordering between them is observable" buys.
#[test]
fn lockstep_pair() {
    let pair = witness("lockstep_pair");
    let registry = registry_of(&[(0xd1, sum_i64()), (0xd2, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], 5);
        engine.commit();
        write(engine, &["App", "x"], 2);
        engine.commit();
    };

    let from_text = traced(&parsed("lockstep_pair", &pair.source), &registry, script);

    assert_trace("lockstep_pair", &from_text, &pair.golden);

    // The discriminating fact on its own: `b` reads the value `a` PUBLISHED,
    // never the one it is holding aside this commit.
    assert!(from_text.contains("eval App.b -> 17"), "{from_text}");
    assert!(!from_text.contains("eval App.b -> 19"), "{from_text}");

    let from_api = traced(&built_lockstep_pair(), &registry, script);

    assert_trace("lockstep_pair (constructed)", &from_api, &pair.golden);
}

/// `lockstep_pair.duir`, built through the `ductus-ir` API instead of parsed.
fn built_lockstep_pair() -> Module {
    app(vec![
        input(&["App", "x"], "1"),
        recurrent(&["App", "a"], 0xd1, &[&["App", "x"]], 1, "0"),
        recurrent(&["App", "b"], 0xd2, &[&["App", "a"]], 1, "10"),
    ])
}

/// A cycle closed through a recurrent LOADS — the `derived_cycle` check must
/// not fire on it (RW-4).
///
/// `App.acc` reads `App.step` and `App.step` reads `App.acc`: a ring. Both reads
/// are ordering edges — R80 item 4 drops only self-history and `lookback`, and
/// neither is an `inputs` read — so the ring deadlocks the ordering pass, and
/// the pass RELEASES `App.step`: §13.11.3, *"reads of a recurrent cell always
/// return the previous-committed value. This is the rule that breaks reactive
/// cycles"*, applied to the schedule. §13.11.4 says the same from the other end,
/// and §13.2.6's init-time-cycle rule points at the other category in the same
/// sentence — *"distinct from runtime cycles (§13.11), which the per-commit DAG
/// handles via recurrents-as-delays"*. The witness is the smallest program that
/// is legal for that reason alone.
///
/// The one edge that survives is `step -> acc`, and it INVERTS the ranks: `acc`
/// is declared first and evaluates second, in both walks.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction runs `step` first (the release forces
/// it): it reads `acc`'s slot, which no evaluation has filled yet, and `x`'s
/// `init` `1` → `1`. Then `acc` reads `step`'s just-constructed `1` and its own
/// FALLBACK `0` → `1`, seeds, and advances (`acc.past(1) = 1`). Untraced; the
/// first commit finds nothing dirty.
///
/// **What `step` read is now stated rather than dodged.** A recurrent's slot is
/// allocated at its DEPTH-1 FALLBACK, not at a typed zero, so the released
/// reader gets the value the source spells — here `init 0`, which is why this
/// golden's numbers are unchanged and why they no longer depend on the two
/// happening to coincide. `engine.rs`'s
/// `a_read_of_an_unconstructed_recurrent_yields_its_fallback` pins the same
/// property where the two DIFFER. What is still open is WHICH nodes a cycle
/// releases: the release is rank-least among every waiting node whose unmet
/// predecessors are all recurrent reads, which is a wider set than the cycle's
/// members — a plain reader downstream of `App.acc` would be eligible too, and
/// declared before `App.step` it would take the fallback instead (FND-40). This
/// witness has exactly one candidate, so its numbers are forced.
///
/// **Block 2.** `App.x` dirties `1 -> 5`. Step 3's order is `step`, `acc`:
///
/// - `step` is stale (`x` is dirty) and reads `acc` from pending — the committed
///   `1` — plus `5` → `6`;
/// - `acc` is stale because `step` just dirtied, and reads `step`'s PENDING `6`
///   — this commit's value, written a line ago — plus its ring's `1` → `7`.
///
/// One turn of the loop per commit, and the two readings are distinguishable:
/// `acc` reads `step` FORWARD (this commit's value, an ordinary derived read)
/// while `step` reads `acc` BACKWARD (the previous commit's, the cycle-breaking
/// read).
///
/// **Block 3.** `App.x` dirties `5 -> 2`. `step` reads `acc`'s committed `7`
/// plus `2` → `9`; `acc` reads `9 + 7` → `16`.
///
/// **Block 4 — the free run, and the point of the extra commit.** NOTHING is
/// written. `acc` advanced to a new value last block, so under §4.12 it is a
/// SILENT dirty root here: `step` is stale on it and reads the committed `16`
/// plus the unchanged `x = 2` → `18`; `acc` is stale on `step` and reads
/// `18 + 16` → `34`. The loop therefore keeps turning after the input stops —
/// once per commit, forever — because each advance roots the next commit. That
/// is what a cycle closed through a recurrent IS: a delay loop, and a delay loop
/// with no input still ticks.
///
/// It is also the boundary of §13.14.4's empty-commit idempotence, stated
/// exactly: idempotence holds for a graph with NO PENDING ADVANCE ROOT, and this
/// graph never has none. What stays true unconditionally is the finiteness law —
/// one evaluation per cell per commit, whatever the ring is doing.
#[test]
fn self_cycle() {
    let pair = witness("self_cycle");
    let registry = registry_of(&[(0xd1, sum_i64()), (0xd2, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], 5);
        engine.commit();
        write(engine, &["App", "x"], 2);
        engine.commit();
        engine.commit();
    };

    let from_text = traced(&parsed("self_cycle", &pair.source), &registry, script);

    assert_trace("self_cycle", &from_text, &pair.golden);

    // The free run on its own: the last block was driven by NO write, so a
    // trace whose commits all needed an input would be two blocks short here.
    assert_eq!(from_text.matches("dirty App.x").count(), 2, "{from_text}");
    assert_eq!(from_text.matches("eval App.acc").count(), 3, "{from_text}");

    // The edge inverts the ranks: the rank-2 derived is scheduled before the
    // rank-1 recurrent that reads it, in every block.
    let source = from_text
        .find("eval App.step")
        .expect("`App.step` evaluates");
    let sink = from_text.find("eval App.acc").expect("`App.acc` evaluates");

    assert!(source < sink, "{from_text}");

    let from_api = traced(&built_self_cycle(), &registry, script);

    assert_trace("self_cycle (constructed)", &from_api, &pair.golden);
}

/// `self_cycle.duir`, built through the `ductus-ir` API instead of parsed.
fn built_self_cycle() -> Module {
    app(vec![
        input(&["App", "x"], "1"),
        recurrent(&["App", "acc"], 0xd1, &[&["App", "step"]], 1, "0"),
        derived(&["App", "step"], 0xd2, &[&["App", "acc"], &["App", "x"]]),
    ])
}

/// `depth 3` with PER-DEPTH seeds — the ring shifts by exactly one slot per
/// advance, each `k` reads ITS OWN fallback until `k` advances exist, and the
/// oldest value falls off the end (R71 + R72).
///
/// `B@d1` is `sum_i64`, so the body is `x + past(1) + past(2) + past(3)` — every
/// slot contributes, and each `eval` line is therefore a fingerprint of the
/// WHOLE window. The seeds `100`, `20`, `3` are chosen to be readable as digits
/// of their sum: a construction value of `123` says `k = 1`, `2` and `3` each
/// answered from their own entry, in that order. A uniform seed could not say
/// it, and a ring that filled its DEEPEST slot first would print
/// `1 + 100 + 20 + 123 = 244` at the second block instead of what the golden
/// holds.
///
/// # Derivation
///
/// **Block 1 (startup).** No advance exists, so all three depths answer from
/// `fallbacks`: `0 + 100 + 20 + 3 = 123`. Seed, then advance — the ring holds
/// `past(1) = 123` and `filled = 1`. Untraced; the first commit finds nothing
/// dirty.
///
/// **Block 2.** `x` dirties `0 -> 1`. One advance exists, so `past(1)` is real
/// (`123`) while `past(2)` and `past(3)` are still on their seeds:
/// `1 + 123 + 20 + 3 = 147`. Advance → `[147, 123]`, `filled = 2`.
///
/// **Block 3.** `x` dirties `1 -> 2`; only `past(3)` is still seeded:
/// `2 + 147 + 123 + 3 = 275`. Advance → `[275, 147, 123]`, `filled = 3` — the
/// ring is now full and no seed is readable again.
///
/// **Block 4.** `x` dirties `2 -> 4`; every slot is real:
/// `4 + 275 + 147 + 123 = 549`. Advance → `[549, 275, 147]`: the `123` FELL OFF.
///
/// **Block 5.** `x` dirties `4 -> 8`: `8 + 549 + 275 + 147 = 979`. Had the ring
/// been one slot deeper, or had it kept the dropped value, this line would read
/// `955`.
///
/// **Block 6.** `x` dirties `8 -> 16`: `16 + 979 + 549 + 275 = 1819`.
#[test]
fn depth3_shift() {
    let pair = witness("depth3_shift");
    let registry = registry_of(&[(0xd1, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        for value in [1, 2, 4, 8, 16] {
            write(engine, &["App", "x"], value);
            engine.commit();
        }
    };

    let from_text = traced(&parsed("depth3_shift", &pair.source), &registry, script);

    assert_trace("depth3_shift", &from_text, &pair.golden);

    let from_api = traced(&built_depth3_shift(), &registry, script);

    assert_trace("depth3_shift (constructed)", &from_api, &pair.golden);
}

/// `depth3_shift.duir`, built through the `ductus-ir` API instead of parsed.
fn built_depth3_shift() -> Module {
    app(vec![
        input(&["App", "x"], "0"),
        seeded(
            &["App", "window"],
            0xd1,
            &[&["App", "x"]],
            &[(1, "100"), (2, "20"), (3, "3")],
        ),
    ])
}

/// The corpus's own two-fallback example — `depth 2 fallbacks [1: 1, 2: 0]`,
/// the `fib` of M1.md §4.9 — made to run.
///
/// Key `k` is the value `.past(k)` returns BEFORE `k` advances exist, and this
/// witness is where that definition is load-bearing: the two seeds are the two
/// starting terms of the sequence, so the golden reads `1, 2, 3, 5, 8`. A
/// uniform seed cannot produce it — `init 1` would trace `3, 5, 8, 13, 21` and
/// `init 0` nothing but zeros — so the golden is the general spelling's
/// regression guard, and the round-trip claim is its canonical one.
///
/// `App.tick` exists only to make the cell stale: a recurrent evaluates when a
/// VALUE input is dirty (M1.md §4.2), and `fib_step_i64` discards the trigger,
/// so every number below comes from the ring alone.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction reads both seeds — no advance exists —
/// `1 + 0 = 1`, seeds the cell and advances: `[1]`, `filled = 1`. Untraced; the
/// first commit finds nothing dirty.
///
/// **Block 2.** `tick` dirties `0 -> 1`. `past(1)` is the construction value
/// `1`; `past(2)` is still seed `0` → `1`. The `eval` line appears even though
/// the value is UNCHANGED — a line records an evaluation that happened, and the
/// dirty bit is what value-change gating sets (R24, §13.2.4.4). Advance → `[1,
/// 1]`, `filled = 2`, and from here no seed is ever read again.
///
/// **Block 3.** `tick` dirties `1 -> 2`: `1 + 1 = 2`. Advance → `[2, 1]`.
///
/// **Block 4.** `2 + 1 = 3` → `[3, 2]`. **Block 5.** `3 + 2 = 5` → `[5, 3]`.
/// **Block 6.** `5 + 3 = 8`.
#[test]
fn fallback_map() {
    let pair = witness("fallback_map");
    let registry = registry_of(&[(0xd1, fib_step_i64())]);
    let script = |engine: &mut Engine<'_>| {
        for tick in 1..=5 {
            write(engine, &["App", "tick"], tick);
            engine.commit();
        }
    };

    let from_text = traced(&parsed("fallback_map", &pair.source), &registry, script);

    assert_trace("fallback_map", &from_text, &pair.golden);

    let from_api = traced(&built_fallback_map(), &registry, script);

    assert_trace("fallback_map (constructed)", &from_api, &pair.golden);
}

/// `fallback_map.duir`, built through the `ductus-ir` API instead of parsed.
fn built_fallback_map() -> Module {
    app(vec![
        input(&["App", "tick"], "0"),
        seeded(
            &["App", "fib"],
            0xd1,
            &[&["App", "tick"]],
            &[(1, "1"), (2, "0")],
        ),
    ])
}

/// R77 made observable: a recurrent that evaluates to the value it ALREADY
/// HOLDS still shifts its ring, and still propagates nothing.
///
/// The two halves are separate laws and this witness asserts both. The ring
/// shifts because an EVALUATION is what advances it — value-change gating
/// governs *"downstream propagation only"* — and nothing downstream runs because
/// the ADVANCE's own comparison decides whether the recurrent is a dirty root at
/// the next commit (§13.2.4.4, R65, §4.12). An equal-valued advance is no root,
/// so it reaches no reader; a changed one is, one commit later.
///
/// `App.acc` is `x + acc.past(2)`, which is what makes the shift visible at all:
/// the slot the stub reads is the one an un-shifted ring would still be holding
/// two commits later.
///
/// # Derivation
///
/// **Block 1 (startup).** `acc = x(0) + fallback(0) = 0`, seeded and advanced →
/// `[0]`, `filled = 1`. `report = acc(0) + 1 = 1` — constructed AFTER `acc`,
/// because a read of a recurrent orders the walk (R80 item 4). Construction's
/// advance is not an advance root, so the first commit finds nothing dirty.
/// Untraced.
///
/// **Block 2.** `x` dirties `0 -> 3`. `acc` reads `3 + past(2)`, and `past(2)`
/// is still its seed `0` → `3`; the result is HELD and sets no dirty bit, so
/// `report`'s only dependency is clean and it does NOT evaluate. Advance →
/// `[3, 0]`, and `0 -> 3` is a change, so `acc` is a root at block 3.
///
/// **Block 3.** `x` dirties `3 -> 1`, and `acc`'s advance dirties `acc` SILENTLY
/// at its own rank. `acc = 1 + past(2)(0) = 1`; `report` is stale on that root
/// and reads `acc`'s committed `3` → `4`. Advance → `[1, 3]`; `3 -> 1` is a
/// change, so `acc` is a root again.
///
/// **Block 4.** `x` dirties `1 -> 0`; `acc = 0 + past(2)(3) = 3`; `report` reads
/// `acc`'s committed `1` → `2`. Advance → `[3, 1]`; `1 -> 3` is a change.
///
/// **Block 5.** `x` dirties `0 -> 2`; `acc = 2 + past(2)(1) = 3`, BIT-IDENTICAL
/// to the committed `3`. The `eval` line is still emitted, and `report` still
/// evaluates — off block 4's advance root, reading the committed `3` → `4`. The
/// ring advances anyway → `[3, 3]`, and THIS advance changes nothing, so it is
/// NO root.
///
/// **Block 6 — the claim, both halves.** `x` dirties `2 -> 0`;
/// `acc = 0 + past(2)`. The line reads `3`, which is only true if block 5
/// consumed a slot: under the rejected reading — no advance on an unchanged
/// value — the ring would still be `[3, 1]` and this line would read `1`. And
/// `report` does NOT evaluate, because block 5's equal-valued advance was no
/// root — value-change gating, moved to the advance where the published value
/// is what a reader would read. The golden discriminates both readings, in the
/// arithmetic and in the line count.
#[test]
fn equal_value_advance() {
    let pair = witness("equal_value_advance");
    let registry = registry_of(&[(0xd1, plus_past2_i64()), (0xd2, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        for value in [3, 1, 0, 2, 0] {
            write(engine, &["App", "x"], value);
            engine.commit();
        }
    };

    let from_text = traced(
        &parsed("equal_value_advance", &pair.source),
        &registry,
        script,
    );

    assert_trace("equal_value_advance", &from_text, &pair.golden);

    // The suppressed half, on its own: `acc` evaluated in all five commits and
    // `report` in only the three where `acc`'s value actually changed.
    assert_eq!(from_text.matches("eval App.acc").count(), 5);
    assert_eq!(from_text.matches("eval App.report").count(), 3);

    let from_api = traced(&built_equal_value_advance(), &registry, script);

    assert_trace("equal_value_advance (constructed)", &from_api, &pair.golden);
}

/// `equal_value_advance.duir`, built through the `ductus-ir` API instead of
/// parsed.
fn built_equal_value_advance() -> Module {
    app(vec![
        input(&["App", "x"], "0"),
        recurrent(&["App", "acc"], 0xd1, &[&["App", "x"]], 2, "0"),
        derived(&["App", "report"], 0xd2, &[&["App", "acc"]]),
    ])
}

/// R72's discriminating witness: a commit in which the recurrent does not
/// evaluate consumes NO slot, so `.past(k)` counts ADVANCES and never commits.
///
/// The skipped commit is not an idle one — `App.other` dirties and `App.echo`
/// re-evaluates in it, so the block is real work. What does not happen is
/// `App.acc`'s evaluation: its only VALUE input, `App.x`, is clean, and a
/// recurrent is stale on exactly that test.
///
/// `App.acc` is `x + acc.past(1) + acc.past(2)` — `sum_i64` over the three
/// arguments the ABI hands it — so the ring's whole content is readable off one
/// line.
///
/// # Derivation
///
/// **Block 1 (startup).** `acc = 0 + 0 + 0 = 0` off the seeds, advanced → `[0]`,
/// `filled = 1`; `echo = other(0) + 1 = 1`. Untraced; the first commit finds
/// nothing dirty.
///
/// **Block 2.** `x` dirties `0 -> 7`: `7 + 0 + seed(0) = 7`. Advance → `[7, 0]`.
///
/// **Block 3.** `x` dirties `7 -> 20`: `20 + 7 + 0 = 27`. Advance → `[27, 7]`.
///
/// **Block 4 — the skip.** Only `App.other` is written, so the rank-ordered walk
/// dirties `App.other` alone and `App.echo` evaluates to `2`. `App.acc` is not
/// stale, emits nothing, and — the point — does not advance: the ring is still
/// `[27, 7]` when the block ends.
///
/// **Block 5 — the evidence.** `x` dirties `20 -> 0`: `0 + 27 + 7 = 34`. Under
/// the rejected per-COMMIT reading, block 4 would have shifted a duplicate `27`
/// into the ring, `past(2)` would answer `27`, and this line would read `54`.
/// One number separates the two readings of fork 2.
///
/// **Block 6.** `x` dirties `0 -> 1`: `1 + 34 + 27 = 62`.
#[test]
fn skipped_commit() {
    let pair = witness("skipped_commit");
    let registry = registry_of(&[(0xd1, sum_i64()), (0xd2, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], 7);
        engine.commit();
        write(engine, &["App", "x"], 20);
        engine.commit();
        write(engine, &["App", "other"], 1);
        engine.commit();
        write(engine, &["App", "x"], 0);
        engine.commit();
        write(engine, &["App", "x"], 1);
        engine.commit();
    };

    let from_text = traced(&parsed("skipped_commit", &pair.source), &registry, script);

    assert_trace("skipped_commit", &from_text, &pair.golden);

    // Five commits, four advances: the block that dirtied only `App.other`
    // produced no `eval App.acc` line and therefore no shift.
    assert_eq!(from_text.matches("eval App.acc").count(), 4);

    let from_api = traced(&built_skipped_commit(), &registry, script);

    assert_trace("skipped_commit (constructed)", &from_api, &pair.golden);
}

/// `skipped_commit.duir`, built through the `ductus-ir` API instead of parsed.
fn built_skipped_commit() -> Module {
    app(vec![
        input(&["App", "x"], "0"),
        input(&["App", "other"], "0"),
        recurrent(&["App", "acc"], 0xd1, &[&["App", "x"]], 2, "0"),
        derived(&["App", "echo"], 0xd2, &[&["App", "other"]]),
    ])
}

/// `lookback` input history: the ABI order R71 pinned, and an input ring that
/// steps with the recurrent's ADVANCES rather than with commits (R71 + R72).
///
/// The cell is M1.md §6.1's own shape — `inputs [App.x] depth 1
/// lookback [App.x: 2]` — so its behavior takes `(x, past1, x_past1, x_past2)`
/// in exactly that order: inputs in list order, then self-history ascending,
/// then per `lookback` entry ascending. `window_i64` weights the three input
/// values by 100, 10 and 1, so every `eval` line SPELLS the last three values of
/// `App.x` as decimal digits, newest first.
///
/// # Derivation
///
/// **Block 1 (startup).** The input's pre-history has no spelling in the IR —
/// `fallbacks` seeds self-history alone — so construction seeds the `App.x` ring
/// with `x`'s own constructed value, `0`. The cell then evaluates against every
/// seed: `x = 0`, `x_past1 = 0`, `x_past2 = 0` → `0`. Seed, then advance: the
/// self ring takes `0` and the input ring takes `x`'s value as this evaluation
/// SAW it — `0` — so `x_past1 = 0`, `filled = 1`. Untraced; the first commit
/// finds nothing dirty.
///
/// **Block 2.** `x` dirties `0 -> 5`. One input advance exists, so
/// `x_past1 = 0` (construction's) and `x_past2` is still a seed `0`:
/// `500 + 0 + 0 = 500`. Advance: the input ring takes the value this evaluation
/// read, the PENDING `5` → `[5, 0]`, `filled = 2`.
///
/// **Block 3.** `x` dirties `5 -> 4`: `400 + 50 + 0 = 450` — `x_past1` is the
/// `5` of one advance ago and `x_past2` is still the seed. Advance → `[4, 5]`.
///
/// **Block 4.** `x` dirties `4 -> 3`: `300 + 40 + 5 = 345`. The ring is full;
/// every digit is now a real value of `App.x`. Advance → `[3, 4]`.
///
/// **Block 5 — the empty commit.** Nothing is staged, so nothing dirties,
/// nothing evaluates, and `publish` still fires (§13.14.4's idempotence). The
/// recurrent did not advance, so NEITHER RING MOVED — the input ring is still
/// `[3, 4]` when the block ends.
///
/// **Block 6 — the evidence.** `x` dirties `3 -> 2`: `200 + 30 + 4 = 234`. Under
/// a per-COMMIT input ring, block 5 would have pushed a duplicate `3` and this
/// line would read `233`. The input ring follows the reading recurrent's
/// advances, exactly as the self ring does (C-10).
///
/// **Block 7.** `x` dirties `2 -> 1`: `100 + 20 + 3 = 123`.
#[test]
fn lookback_read() {
    let pair = witness("lookback_read");
    let registry = registry_of(&[(0xd1, window_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], 5);
        engine.commit();
        write(engine, &["App", "x"], 4);
        engine.commit();
        write(engine, &["App", "x"], 3);
        engine.commit();
        engine.commit();
        write(engine, &["App", "x"], 2);
        engine.commit();
        write(engine, &["App", "x"], 1);
        engine.commit();
    };

    let from_text = traced(&parsed("lookback_read", &pair.source), &registry, script);

    assert_trace("lookback_read", &from_text, &pair.golden);

    // The skipped commit on its own: had the input ring stepped with commits,
    // the block after it would carry a repeated digit.
    assert!(from_text.contains("eval App.window -> 234"), "{from_text}");
    assert!(!from_text.contains("eval App.window -> 233"), "{from_text}");

    let from_api = traced(&built_lookback_read(), &registry, script);

    assert_trace("lookback_read (constructed)", &from_api, &pair.golden);
}

/// `lookback_read.duir`, built through the `ductus-ir` API instead of parsed.
fn built_lookback_read() -> Module {
    let mut window = recurrent(&["App", "window"], 0xd1, &[&["App", "x"]], 1, "0");
    window.lookback = vec![lookback(&["App", "x"], 2)];

    app(vec![input(&["App", "x"], "0"), window])
}

/// R65's bitwise law applied to a HISTORY SLOT: a `NaN` written into the graph
/// rides two advances and comes back out of `past(2)` with the same bit pattern.
///
/// The value-change comparison is bitwise everywhere (R65) — otherwise a
/// NaN-holding cell would be perpetually dirty, since IEEE `==` answers false
/// for it — and this witness carries that law into the ring, which is the one
/// place a value is copied rather than compared. `nan_in_the_ring` answers `1`
/// when and only when `past(2)` is bit-for-bit `NaN`, so the last line of the
/// golden is a one-bit assertion dressed as arithmetic.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction evaluates `App.r` against its seeds:
/// `x = 0`, `past(1) = 0`, `past(2) = 0`. `past(2)` is not `NaN`, so the stub
/// echoes `x` → `0`. Seed, then advance — the ring holds `[0]`, `filled = 1`.
/// Untraced; the first commit finds nothing dirty.
///
/// **Block 2.** `App.x` dirties `0 -> NaN`: the staged bits differ from the
/// committed ones, which is the ONLY reason this line exists — under IEEE
/// equality a `NaN` write would be indistinguishable from no write at all.
/// `App.r` evaluates with `past(2)` still on its seed `0` → echoes `x` → `NaN`.
/// Advance → `[NaN, 0]`, `filled = 2`.
///
/// **Block 3.** `App.x` dirties `NaN -> 2` — again a bitwise comparison, and
/// again one IEEE `==` would have missed. `past(1)` is `NaN` now and `past(2)`
/// is the construction `0`, so the stub still echoes → `2`. Advance →
/// `[2, NaN]`: the `NaN` moved one slot, untouched.
///
/// **Block 4 — the claim.** `App.x` dirties `2 -> 3`. `past(2)` is the `NaN`
/// written two advances ago, its bits intact, so the stub answers `1` rather
/// than echoing `3`. Any bit lost in the ring — a canonicalized payload, a
/// float rounded through a wider slot — and this line would read `3`.
#[test]
fn nan_history() {
    let pair = witness("nan_history");
    let registry = registry_of(&[(0xd1, nan_in_the_ring())]);
    let script = |engine: &mut Engine<'_>| {
        write_float(engine, &["App", "x"], f64::NAN);
        engine.commit();
        write_float(engine, &["App", "x"], 2.0);
        engine.commit();
        write_float(engine, &["App", "x"], 3.0);
        engine.commit();
    };

    let from_text = traced(&parsed("nan_history", &pair.source), &registry, script);

    assert_trace("nan_history", &from_text, &pair.golden);

    // The claim on its own: the last evaluation answered the sentinel, which it
    // can only do off a `past(2)` whose bits never changed.
    assert!(from_text.contains("eval App.r -> 1"), "{from_text}");

    let from_api = traced(&built_nan_history(), &registry, script);

    assert_trace("nan_history (constructed)", &from_api, &pair.golden);
}

/// `nan_history.duir`, built through the `ductus-ir` API instead of parsed.
fn built_nan_history() -> Module {
    app(vec![
        float_input(&["App", "x"], "0.0"),
        float_recurrent(&["App", "r"], 0xd1, &[&["App", "x"]], 2, "0.0"),
    ])
}
