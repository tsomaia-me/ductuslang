//! Witness tests for `gate` — the M1 gate family (corpus/README.md, R25,
//! M1.md §6.3).
//!
//! One `#[test]` fn per witness, named after its `.duir` file stem, so a witness
//! is grep-able by one word across the corpus and the tests.
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
//! semantics contract — SPEC §13.10.2's steps as widened in M1.md §4, and
//! §13.9.7's freeze law as pinned in M1.md §4.7 — and the derivation is the test
//! fn's doc comment, block by block. A golden captured from the engine would
//! assert only that the engine does what the engine does.
//!
//! # The four gate shape rules every derivation below applies
//!
//! - **The predicate reads PRIOR-COMMIT values** (R73, step 1a). A staged write
//!   publishes at commit N and the predicate sees it at commit N+1, so a flip
//!   and its snap are always one commit BEHIND the write that caused them. Every
//!   witness that moves a gate therefore spends two commit blocks doing it, and
//!   the block with the `dirty` line on the predicate's input is never the block
//!   with the `gate` line.
//! - **A `gate <path> open|close` line records EFFECTIVE activation** — own gate
//!   AND every ancestor (C-39/C-40) — and appears only on a CHANGE. A gate that
//!   re-answers what it already answered contributes nothing, exactly as a clean
//!   input contributes no `dirty` line.
//! - **Root lines are ONE rank sequence** (R76). Gate transitions and `dirty`
//!   lines interleave by graph-section entry rank (C-45), so a gate declared
//!   between two cells emits its line between their `dirty` lines.
//! - **Activation is checked BEFORE staleness** (RW-9). A frozen cell is
//!   ineligible even with dirty dependencies, so a closed gate turns an `eval`
//!   line into nothing at all — and the MISSING line is the assertion.
//!
//! A trace still OPENS with the empty block `commit_begin`/`publish`/
//! `commit_end` — §13.14.1 step 4's first commit, with construction (step 3)
//! already done and untraced. Under gates that block carries one extra claim: a
//! gate closed at startup emits NO line, because the first commit's pre-pass
//! recomputes the same predicate over the same values and finds no TRANSITION
//! (RW-7).

use std::rc::Rc;

use ductus_conformance::module_builder::{cell, module, path, scope};
use ductus_conformance::runner::{Witness, parsed, traced};
use ductus_conformance::stub_library::{add1_i64, registry_of, sum_i64};
use ductus_conformance::trace_compare::assert_trace;
use ductus_ir::diag::Span;
use ductus_ir::graph::{Cell, CellKind, Entry, Gate, ValueLiteral};
use ductus_ir::module::Module;
use ductus_ir::types::Prim;
use ductus_runtime::engine::Engine;
use ductus_runtime::provider::Behavior;
use ductus_runtime::store::Snapshot;
use ductus_runtime::value::Value;

/// A loaded witness pair out of `corpus/duir/m1/` — the M1 milestone directory
/// (corpus/README.md), read by the same manifest-anchored law as M0's.
fn witness(name: &str) -> Witness {
    Witness::load_milestone("m1", name)
}

/// Stages a write, failing the test with the path when the engine refuses it.
fn write(engine: &mut Engine<'_>, names: &[&str], value: Value) {
    engine
        .write_signal(&path(names), value)
        .unwrap_or_else(|error| {
            panic!(
                "`{}` is a legal write: {}",
                path(names).render(),
                error.message
            )
        });
}

/// The rendered value at `names` — the canonical rendering is what a trace
/// carries, so the frozen-value assertions compare renderings rather than
/// values.
fn read(snapshot: &Snapshot<'_>, names: &[&str]) -> String {
    snapshot
        .read(&path(names))
        .expect("the cell exists")
        .render()
}

fn input(names: &[&str], prim: Prim, init: ValueLiteral) -> Cell {
    let mut built = cell(CellKind::Input, names, prim);
    built.init = Some(init);

    built
}

fn derived(names: &[&str], prim: Prim, uses: u32, inputs: &[&[&str]]) -> Cell {
    let mut built = cell(CellKind::Derived, names, prim);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();

    built
}

/// A `recurrent` with the UNIFORM `init` fallback — every depth falls back to
/// one value (R71's shorthand), which is what both history witnesses here want.
fn recurrent(
    names: &[&str],
    prim: Prim,
    uses: u32,
    inputs: &[&[&str]],
    depth: u64,
    init: ValueLiteral,
) -> Cell {
    let mut built = cell(CellKind::Recurrent, names, prim);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();
    built.depth = Some(depth);
    built.init = Some(init);

    built
}

/// A `deferred` with its behavior handle, input paths and latency in commits —
/// the three clauses 033-245 renders, and a deferred admits no others.
fn deferred(names: &[&str], prim: Prim, uses: u32, inputs: &[&[&str]], depth: u64) -> Cell {
    let mut built = cell(CellKind::Deferred, names, prim);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();
    built.depth = Some(depth);

    built
}

/// A `gate` entry with its predicate handle, the cells the predicate reads, the
/// constructs it guards, and an optional enclosing gate.
///
/// `guards` is MATERIALIZED in every witness here (C-43, R80 item 5): §15.4.1
/// calls the list "the materialized inverse" of the guarded construct's own
/// back-reference, and an inverse is a restatement — so the loader checks the
/// two sets for EQUALITY and a module spelling only one end no longer loads.
/// Every `.duir` file in the family therefore names its guarded constructs, and
/// every module built here names the same ones.
fn gate(
    names: &[&str],
    pred: u32,
    inputs: &[&[&str]],
    guards: &[&[&str]],
    parent: Option<&[&str]>,
) -> Gate {
    Gate {
        path: path(names),
        span: Span::new(0, 0),
        pred,
        inputs: inputs.iter().map(|names| path(names)).collect(),
        guards: guards.iter().map(|names| path(names)).collect(),
        gate_parent: parent.map(path),
    }
}

/// The same cell, carrying a `gate PATH` back-reference —
/// `gated(derived(…), &["App", "g"])`.
fn gated(mut cell: Cell, gate: &[&str]) -> Cell {
    cell.gate = Some(path(gate));

    cell
}

/// The same cell, carrying `reset_on_reopen` — a recurrent's clause and no other
/// kind's (C-42).
fn resetting(mut cell: Cell) -> Cell {
    cell.reset_on_reopen = true;

    cell
}

/// The gate predicate of eight of the eleven witnesses: it answers the `bool`
/// input it reads, unchanged.
///
/// Written INLINE rather than in the shared stub library because it is this
/// file's fixture — a gate witness needs a predicate whose answer the SCRIPT
/// controls, and passing a written `bool` straight through is the only shape in
/// which "the predicate flipped at commit N+1" is a statement about the write
/// rather than about arithmetic.
///
/// # Panics
///
/// On any other argument shape — a harness-author bug, loud rather than papered
/// over.
fn open_when_true() -> Behavior {
    Rc::new(|inputs| match inputs {
        [Value::Bool(open)] => Value::Bool(*open),
        other => panic!("open_when_true wants one bool input, got {other:?}"),
    })
}

/// `|level| level > 0` — the predicate of the three witnesses whose gate reads
/// an `i64` rather than a written `bool`.
///
/// It is what lets `predicate_stable` move its input three times and its boolean
/// once, what gives `rank_interleaved_roots` a transition off an ordinary
/// numeric write, and what makes `self_conditional_gate` decidable at all: a
/// predicate over a GUARDED cell has to read a value that cell computes, and a
/// `bool` passthrough would need the guarded cell to be the boolean itself.
///
/// # Panics
///
/// On any other argument shape — a harness-author bug.
fn positive_i64() -> Behavior {
    Rc::new(|inputs| match inputs {
        [Value::I64(level)] => Value::Bool(*level > 0),
        other => panic!("positive_i64 wants one i64 input, got {other:?}"),
    })
}

fn int(digits: &str) -> ValueLiteral {
    ValueLiteral::int(digits.to_string())
}

/// §13.2.6 / 016-144: a gate that evaluates false at the end of startup begins
/// INACTIVE, *"with its other cells holding their just-computed initial
/// values"*. The subtree is constructed unconditionally and only then freezes —
/// gates gate propagation, never existence (RW-7).
///
/// `App.free` and `App.held` are the same behavior over the same input and
/// differ only in the `gate` clause, so the block below reads as a controlled
/// experiment: whatever the gate does to `held`, it demonstrably does not do to
/// `free`.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction walks every computed cell once, gate
/// clauses ignored: `free = 2 + 1 = 3` and `held = 2 + 1 = 3`. Untraced —
/// construction is not a commit. The predicate then runs over those constructed
/// values, reads `App.open = false`, and `App.g` begins CLOSED. Step 4's first
/// commit re-runs the same predicate over the same buffer, finds no TRANSITION,
/// and so emits no `gate` line: the block is `commit_begin`/`publish`/
/// `commit_end`, exactly as an ungated graph's.
///
/// **Block 2.** The write stages `5`, which differs bitwise from the committed
/// `2`, so `App.x` dirties `2 -> 5`. Step 3 walks the static order. `App.free`
/// and `App.held` are the same topological level and tie-break by rank, so
/// `free` comes first: it is active, its dependency is dirty, and it evaluates
/// to `6`. `held` is next and is NOT effectively active, so the activation test
/// — checked BEFORE staleness (RW-9) — makes it ineligible with a dirty
/// dependency in hand. It emits nothing, and its committed `3` stands.
#[test]
fn closed_at_startup() {
    let pair = witness("closed_at_startup");
    let registry = registry_of(&[(0xd0, open_when_true()), (0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], Value::I64(5));
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        // The constructed value survives the freeze and is readable — reads of
        // a frozen cell are always defined (§13.9.7), never `Option[T]`.
        assert_eq!(read(&snapshot, &["App", "held"]), "3");
        assert_eq!(read(&snapshot, &["App", "free"]), "6");
    };

    let from_text = traced(
        &parsed("closed_at_startup", &pair.source),
        &registry,
        script,
    );

    assert_trace("closed_at_startup", &from_text, &pair.golden);

    // A gate closed at startup is not a TRANSITION, so nothing about it is
    // observable in the trace at all.
    assert!(!from_text.contains("gate "), "{from_text}");
    assert!(!from_text.contains("eval App.held"), "{from_text}");

    let from_api = traced(&built_closed_at_startup(), &registry, script);

    assert_trace("closed_at_startup (constructed)", &from_api, &pair.golden);
}

/// `closed_at_startup.duir`, built through the `ductus-ir` API instead of
/// parsed.
fn built_closed_at_startup() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(&["App", "open"], Prim::Bool, ValueLiteral::False)),
            Entry::Gate(gate(
                &["App", "g"],
                0xd0,
                &[&["App", "open"]],
                &[&["App", "held"]],
                None,
            )),
            Entry::Cell(input(&["App", "x"], Prim::I64, int("2"))),
            Entry::Cell(derived(&["App", "free"], Prim::I64, 0xd1, &[&["App", "x"]])),
            Entry::Cell(gated(
                derived(&["App", "held"], Prim::I64, 0xd1, &[&["App", "x"]]),
                &["App", "g"],
            )),
        ],
    )])
}

/// The open edge, and R73's regression guard in the same four blocks: the write
/// that flips the predicate publishes at commit N, the flip and its SNAP happen
/// at commit N+1, and the snapped derived evaluates over CURRENT values though
/// nothing dirtied that commit.
///
/// This witness fails under any other reading of fork 3. A predicate reading
/// THIS commit's staged values would put the `gate` line and the `eval` in block
/// 3 beside `dirty App.open false -> true`, and block 4 would not exist.
///
/// # Derivation
///
/// **Block 1 (startup).** `held = 2 + 1 = 3` at construction, untraced. The
/// predicate reads `App.open = false`, so `App.g` begins closed and emits no
/// line.
///
/// **Block 2.** `App.x` dirties `2 -> 5`. `App.held` is frozen, so the
/// activation test refuses it before staleness is even asked (RW-9): a `dirty`
/// line with no `eval` after it. The value `5` is now COMMITTED and waiting.
///
/// **Block 3.** The write stages `true` for `App.open`. Step 1a runs the
/// predicate against the LIVE buffer — the values as they stand at commit start
/// — where `App.open` is still `false`. So there is no transition and no `gate`
/// line; the block carries only `dirty App.open false -> true`. `App.held`'s own
/// dependency `App.x` is clean this commit, so it would not have evaluated even
/// if it were active.
///
/// **Block 4.** Nothing is written. Step 1a now reads the PUBLISHED `true`, so
/// `App.g`'s effective activation goes false → true and the transition joins
/// step 1b's rank walk: `App.g` has entry rank 1 and `App.x` has rank 2, so
/// `gate App.g open` is emitted where the gate was declared. Nothing is staged,
/// so no `dirty` line joins it.
///
/// Step 3 then applies 022-59: *"The gate-open re-evaluation snap is scheduled
/// within the same commit that flips the predicate false→true — not the next
/// commit."* `App.held` is forced stale by the transition alone and evaluates
/// over CURRENT values — the pending `App.x`, which is the `5` block 2
/// committed — giving `6`. The `6` is the whole assertion: a snap that read the
/// pre-gap value would print `3`, and a snap deferred by a commit would print
/// nothing here at all.
#[test]
fn open_snap() {
    let pair = witness("open_snap");
    let registry = registry_of(&[(0xd0, open_when_true()), (0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], Value::I64(5));
        engine.commit();
        write(engine, &["App", "open"], Value::Bool(true));
        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "held"]), "6");
    };

    let from_text = traced(&parsed("open_snap", &pair.source), &registry, script);

    assert_trace("open_snap", &from_text, &pair.golden);

    // The one-commit delay, stated as an ordering rather than as a block count:
    // the flip is strictly after the write that caused it published.
    let written = from_text
        .find("dirty App.open")
        .expect("the predicate's input is written");
    let flipped = from_text.find("gate App.g open").expect("the gate opens");

    assert!(written < flipped, "{from_text}");

    let from_api = traced(&built_open_snap(), &registry, script);

    assert_trace("open_snap (constructed)", &from_api, &pair.golden);
}

/// `open_snap.duir`, built through the `ductus-ir` API instead of parsed.
fn built_open_snap() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(&["App", "open"], Prim::Bool, ValueLiteral::False)),
            Entry::Gate(gate(
                &["App", "g"],
                0xd0,
                &[&["App", "open"]],
                &[&["App", "held"]],
                None,
            )),
            Entry::Cell(input(&["App", "x"], Prim::I64, int("2"))),
            Entry::Cell(gated(
                derived(&["App", "held"], Prim::I64, 0xd1, &[&["App", "x"]]),
                &["App", "g"],
            )),
        ],
    )])
}

/// The close edge: the subtree's last committed values FREEZE and stay readable,
/// and the `gate App.g close` line takes its place in the rank sequence.
///
/// The module is `open_snap`'s with the gate starting OPEN, so the two witnesses
/// read as one law from both directions.
///
/// # Derivation
///
/// **Block 1 (startup).** `held = 3` at construction. The predicate reads
/// `App.open = true`, so `App.g` begins OPEN — and an already-open gate is no
/// more a transition than an already-closed one, so again no line.
///
/// **Block 2.** `App.x` dirties `2 -> 5`; `App.held` is active and stale, so it
/// evaluates to `6`. This is the value the freeze will preserve.
///
/// **Block 3.** The write stages `false`. Step 1a still reads the live `true`,
/// so no transition and no line — the one-commit delay again, in the closing
/// direction.
///
/// **Block 4.** The write stages `9` for `App.x`. Step 1a now reads the
/// published `false`, so `App.g` goes true → false. Step 1b walks entry rank:
/// `App.open` (rank 0) is clean, then the gate at rank 1 emits
/// `gate App.g close`, then `App.x` at rank 2 emits `dirty App.x 5 -> 9`. THAT
/// order is R76's single rank sequence — a reader cannot tell which root source
/// produced which line, and the gate line lands where the gate was declared
/// rather than at the head or the tail of the block.
///
/// Step 3 then refuses `App.held`: it is frozen, so a dirty dependency buys it
/// nothing, and *"Deriveds do not recompute. They hold their last committed
/// value."* `App.x` publishes `9` all the same — a closed gate freezes
/// propagation, not the input cells themselves.
#[test]
fn close_freeze() {
    let pair = witness("close_freeze");
    let registry = registry_of(&[(0xd0, open_when_true()), (0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], Value::I64(5));
        engine.commit();
        write(engine, &["App", "open"], Value::Bool(false));
        engine.commit();
        write(engine, &["App", "x"], Value::I64(9));
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        // The frozen value is the LAST COMMITTED one, and it is readable — not
        // the value `App.x = 9` would have produced.
        assert_eq!(read(&snapshot, &["App", "held"]), "6");
        assert_eq!(read(&snapshot, &["App", "x"]), "9");
    };

    let from_text = traced(&parsed("close_freeze", &pair.source), &registry, script);

    assert_trace("close_freeze", &from_text, &pair.golden);
    assert_eq!(from_text.matches("eval App.held").count(), 1, "{from_text}");

    let from_api = traced(&built_close_freeze(), &registry, script);

    assert_trace("close_freeze (constructed)", &from_api, &pair.golden);
}

/// `close_freeze.duir`, built through the `ductus-ir` API instead of parsed.
fn built_close_freeze() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(&["App", "open"], Prim::Bool, ValueLiteral::True)),
            Entry::Gate(gate(
                &["App", "g"],
                0xd0,
                &[&["App", "open"]],
                &[&["App", "held"]],
                None,
            )),
            Entry::Cell(input(&["App", "x"], Prim::I64, int("2"))),
            Entry::Cell(gated(
                derived(&["App", "held"], Prim::I64, 0xd1, &[&["App", "x"]]),
                &["App", "g"],
            )),
        ],
    )])
}

/// §13.9.7's first row, alone in a witness: *"Input cells stay live. Wires
/// delivering into the gated node still write their values into the
/// destination's input cells."*
///
/// `App.x` carries the `gate` clause ITSELF here, which is what makes the claim
/// non-vacuous: the input BELONGS to the guarded subtree and still stages,
/// dirties and publishes. Freezing is about propagation, and an input cell
/// propagates to nothing by itself.
///
/// # Derivation
///
/// **Block 1 (startup).** `held = 2 + 1 = 3`; the predicate reads
/// `App.open = false`, so `App.g` begins closed and emits nothing.
///
/// **Block 2.** The write stages `5`. Step 1 asks an input for its staged value
/// without asking whether it is active — the activation test lives in step 3,
/// not step 1 — so `App.x` dirties `2 -> 5` and the value is copied into
/// pending. Step 3 then reaches `App.held`, finds it inactive, and stops: the
/// `dirty` line stands alone.
///
/// **Block 3.** The same again, `5 -> 7`. A second dirty line with no `eval`
/// says the first was the law rather than a one-commit accident.
///
/// The MISSING `eval` lines are the assertion, and the final read is its other
/// half: `App.x` is at `7` — genuinely live — while `App.held` is at the `3`
/// construction computed.
#[test]
fn live_inputs_frozen_derived() {
    let pair = witness("live_inputs_frozen_derived");
    let registry = registry_of(&[(0xd0, open_when_true()), (0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], Value::I64(5));
        engine.commit();
        write(engine, &["App", "x"], Value::I64(7));
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "x"]), "7");
        assert_eq!(read(&snapshot, &["App", "held"]), "3");
    };

    let from_text = traced(
        &parsed("live_inputs_frozen_derived", &pair.source),
        &registry,
        script,
    );

    assert_trace("live_inputs_frozen_derived", &from_text, &pair.golden);

    // Nothing evaluated in the whole session: every write reached a live input
    // and stopped there.
    assert!(!from_text.contains("eval "), "{from_text}");

    let from_api = traced(&built_live_inputs_frozen_derived(), &registry, script);

    assert_trace(
        "live_inputs_frozen_derived (constructed)",
        &from_api,
        &pair.golden,
    );
}

/// `live_inputs_frozen_derived.duir`, built through the `ductus-ir` API instead
/// of parsed.
fn built_live_inputs_frozen_derived() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(&["App", "open"], Prim::Bool, ValueLiteral::False)),
            Entry::Gate(gate(
                &["App", "g"],
                0xd0,
                &[&["App", "open"]],
                &[&["App", "x"], &["App", "held"]],
                None,
            )),
            Entry::Cell(gated(
                input(&["App", "x"], Prim::I64, int("2")),
                &["App", "g"],
            )),
            Entry::Cell(gated(
                derived(&["App", "held"], Prim::I64, 0xd1, &[&["App", "x"]]),
                &["App", "g"],
            )),
        ],
    )])
}

/// Effective activation is the AND down the `gate_parent` chain, and the three
/// consequences C-39/C-40 draw from it: an inner gate's own predicate keeps
/// running under a closed ancestor, only the EFFECTIVE change traces, and an
/// ancestor's reopen restores each descendant to ITS OWN state with no replay.
///
/// Nine blocks. A gate move always costs two — one commit to publish the
/// predicate's input, one for the predicate to see it (R73) — so a witness about
/// nesting is necessarily long.
///
/// # Derivation
///
/// **Block 1 (startup).** `deep = 2 + 1 = 3`. Both predicates read `true`, so
/// both gates begin open and neither transitions.
///
/// **Block 2.** `App.outer_open` dirties `true -> false`. Step 1a still read the
/// live `true`, so nothing moved yet.
///
/// **Block 3.** Step 1a now reads the published `false`. `App.outer` goes
/// true → false. `App.inner`'s OWN predicate answers `true` — its own input has
/// not changed — but its EFFECTIVE activation is `own AND every ancestor`, which
/// is now false, so it transitions too. TWO lines, in gate rank order:
/// `gate App.outer close` then `gate App.inner close`. Closing one gate freezes
/// the whole subtree beneath it, and the trace says so once per gate.
///
/// **Block 4.** `App.x` dirties `2 -> 5` and `App.deep` does not recompute. The
/// value `5` is committed and waiting on the far side of the gap.
///
/// **Block 5.** `App.inner_open` dirties `true -> false`. Step 1a read the live
/// `true`, so `App.inner`'s own value is still `true` this commit.
///
/// **Block 6.** `App.outer_open` dirties `false -> true`. Step 1a now reads
/// `App.inner_open = false`, so `App.inner`'s OWN value goes true → false — and
/// NOTHING is emitted, because its effective activation was already false and
/// stayed false. That silence is C-39/C-40's asymmetry: own state updates under
/// a closed ancestor, effective state is what traces.
///
/// **Block 7.** Step 1a reads the published `App.outer_open = true`, so
/// `App.outer` reopens: `gate App.outer open`. `App.inner` recomposes to
/// `own(false) AND outer(true)` = false — unchanged — so no line, and `App.deep`
/// stays frozen. Reopening an ancestor restored the descendant to its own gate
/// state, which happens to be CLOSED, and nothing was replayed.
///
/// **Block 8.** `App.inner_open` dirties `false -> true`.
///
/// **Block 9.** Step 1a reads it: `App.inner` goes false → true effectively, so
/// `gate App.inner open`, and the snap evaluates `App.deep` over CURRENT values
/// — the pending `App.x`, which is block 4's `5` — giving `6`. The `6` is the
/// assertion that the snap reads across the gap rather than resuming from the
/// pre-gap `3`.
#[test]
fn nested_gates() {
    let pair = witness("nested_gates");
    let registry = registry_of(&[(0xd0, open_when_true()), (0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "outer_open"], Value::Bool(false));
        engine.commit();
        engine.commit();
        write(engine, &["App", "x"], Value::I64(5));
        engine.commit();
        write(engine, &["App", "inner_open"], Value::Bool(false));
        engine.commit();
        write(engine, &["App", "outer_open"], Value::Bool(true));
        engine.commit();
        engine.commit();
        write(engine, &["App", "inner_open"], Value::Bool(true));
        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "deep"]), "6");
    };

    let from_text = traced(&parsed("nested_gates", &pair.source), &registry, script);

    assert_trace("nested_gates", &from_text, &pair.golden);

    // Gate-line/state agreement. `App.inner`'s OWN boolean changes twice — at
    // block 6 and at block 9 — and its EFFECTIVE activation also changes twice,
    // but not at the same commits: it closes at block 3, driven entirely by its
    // ancestor, and opens at block 9. Two lines, and neither of them at block 6.
    // Over- or under-reporting a nested flip is what this count catches.
    assert_eq!(
        from_text.matches("gate App.inner").count(),
        2,
        "{from_text}"
    );
    assert_eq!(
        from_text.matches("gate App.outer").count(),
        2,
        "{from_text}"
    );

    // The ancestor's reopen replayed nothing: the only evaluation in the whole
    // session is the one the INNER gate's own reopen snapped.
    assert_eq!(from_text.matches("eval App.deep").count(), 1, "{from_text}");

    let from_api = traced(&built_nested_gates(), &registry, script);

    assert_trace("nested_gates (constructed)", &from_api, &pair.golden);
}

/// `nested_gates.duir`, built through the `ductus-ir` API instead of parsed.
fn built_nested_gates() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(
                &["App", "outer_open"],
                Prim::Bool,
                ValueLiteral::True,
            )),
            Entry::Cell(input(
                &["App", "inner_open"],
                Prim::Bool,
                ValueLiteral::True,
            )),
            Entry::Gate(gate(
                &["App", "outer"],
                0xd0,
                &[&["App", "outer_open"]],
                &[&["App", "inner"]],
                None,
            )),
            Entry::Gate(gate(
                &["App", "inner"],
                0xd0,
                &[&["App", "inner_open"]],
                &[&["App", "deep"]],
                Some(&["App", "outer"]),
            )),
            Entry::Cell(input(&["App", "x"], Prim::I64, int("2"))),
            Entry::Cell(gated(
                derived(&["App", "deep"], Prim::I64, 0xd1, &[&["App", "x"]]),
                &["App", "inner"],
            )),
        ],
    )])
}

/// §13.9.7's recurrent row, and the loss it names: *"Any input change that would
/// have triggered re-evaluation during a gated period is **lost** — the runtime
/// does not queue triggers, and gate-open does not replay them."*
///
/// This is the single most important interaction witness in cut B, and its whole
/// weight rests on block 5 being EMPTY of evaluations. `App.acc` is
/// `x + past(1)`, so its value is an accumulator whose arithmetic makes both the
/// loss and the ring's stillness readable.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction evaluates `App.acc` once, reading
/// FALLBACKS for its history — no advance exists yet — so `acc = 1 + 0 = 1`, and
/// then ADVANCES, because evaluation is what shifts a ring (R77). The ring's
/// `past(1)` is now `1`. Untraced. The predicate reads `App.open = true`, so the
/// gate begins open and emits nothing.
///
/// **Block 2.** `App.open` dirties `true -> false`; step 1a still read the live
/// `true`, so the gate has not moved. `App.acc`'s value input `App.x` is clean,
/// so it does not evaluate — a recurrent is stale on exactly a derived's test.
///
/// **Block 3.** Step 1a reads the published `false`: `gate App.g close` at entry
/// rank 1, then `dirty App.x 1 -> 5` at rank 2. `App.acc` is frozen, so it does
/// NOT advance and the trigger `App.x` just fired is LOST — not queued, not
/// remembered. The ring still holds the construction advance.
///
/// **Block 4.** `App.open` dirties `false -> true`.
///
/// **Block 5.** Step 1a reads it and the gate reopens: `gate App.g open`.
/// Nothing else. A DERIVED would snap here (022-59), but a recurrent is
/// pointedly excluded: snapping one would manufacture the advance §13.9.7 says
/// never happens. So there is no `eval App.acc` line, and the cell still reads
/// `1` — its pre-gap value — although `App.x` has been `5` since block 3.
///
/// **Block 6.** A NEW trigger finally arrives: `App.x` dirties `5 -> 6`, and
/// `App.acc` evaluates to `6 + past(1)`. `past(1)` is `1`, the value construction
/// advanced — the gap consumed no slot, because a commit in which a recurrent
/// did not evaluate consumes none (R72). So the line reads `7`.
///
/// The `7` discriminates all three wrong answers: `11` if the lost trigger had
/// been replayed at the reopen (`6 + 5`), `12` if the ring had somehow advanced
/// over `App.x`'s frozen-period value, and `6` if the history had been cleared —
/// which is `reopen_reset`'s law, and this cell does not carry the flag.
#[test]
fn gate_recurrent_loss() {
    let pair = witness("gate_recurrent_loss");
    let registry = registry_of(&[(0xd0, open_when_true()), (0xd2, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "open"], Value::Bool(false));
        engine.commit();
        write(engine, &["App", "x"], Value::I64(5));
        engine.commit();
        write(engine, &["App", "open"], Value::Bool(true));
        engine.commit();
        engine.commit();

        let reopened = engine.acquire_snapshot();

        // The reopen alone resumed nothing: the pre-gap value still stands
        // while the input that would have triggered it is long committed.
        assert_eq!(read(&reopened, &["App", "acc"]), "1");
        assert_eq!(read(&reopened, &["App", "x"]), "5");

        // The read view's borrow ends at that last use, which is the compiler —
        // not the runtime — releasing the engine for the next write (R22).
        write(engine, &["App", "x"], Value::I64(6));
        engine.commit();

        let resumed = engine.acquire_snapshot();

        assert_eq!(read(&resumed, &["App", "acc"]), "7");
    };

    let from_text = traced(
        &parsed("gate_recurrent_loss", &pair.source),
        &registry,
        script,
    );

    assert_trace("gate_recurrent_loss", &from_text, &pair.golden);

    // One evaluation in the whole session, and it is the post-gap one: the
    // gated period produced none and the reopen produced none.
    assert_eq!(from_text.matches("eval App.acc").count(), 1, "{from_text}");

    let from_api = traced(&built_gate_recurrent_loss(), &registry, script);

    assert_trace("gate_recurrent_loss (constructed)", &from_api, &pair.golden);
}

/// `gate_recurrent_loss.duir`, built through the `ductus-ir` API instead of
/// parsed.
fn built_gate_recurrent_loss() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(&["App", "open"], Prim::Bool, ValueLiteral::True)),
            Entry::Gate(gate(
                &["App", "g"],
                0xd0,
                &[&["App", "open"]],
                &[&["App", "acc"]],
                None,
            )),
            Entry::Cell(input(&["App", "x"], Prim::I64, int("1"))),
            Entry::Cell(gated(
                recurrent(
                    &["App", "acc"],
                    Prim::I64,
                    0xd2,
                    &[&["App", "x"]],
                    1,
                    int("0"),
                ),
                &["App", "g"],
            )),
        ],
    )])
}

/// `reset_on_reopen`, at its RULED position: the history clears in step 1a,
/// BEFORE step 3's reads (C-13), so the first post-gap evaluation reads
/// FALLBACKS — and it is observable in the SAME commit as the reopen.
///
/// The module is `gate_recurrent_loss`'s with the flag added — the two `.duir`
/// files differ by exactly one clause — and what the flag buys is the last
/// block's arithmetic: `9 + fallback` here where an unflagged recurrent would
/// read `9 + past(1)`.
///
/// The script is what makes the step-1a position visible: the reopen commit must
/// ALSO carry a dirty value input, because a recurrent does not snap on
/// gate-open. So `App.open` is published one commit ahead and `App.x` is written
/// into the commit the flip lands in.
///
/// # Derivation
///
/// **Block 1 (startup).** `acc = 1 + fallback(0) = 1`, then advance — `past(1)`
/// is `1`. The gate begins open.
///
/// **Block 2.** `App.x` dirties `1 -> 2`; `App.acc` evaluates to
/// `2 + past(1) = 2 + 1 = 3` and advances, so `past(1)` is now `3`.
///
/// **Block 3.** `App.open` dirties `true -> false`; step 1a still read `true`.
///
/// **Block 4.** Step 1a reads the published `false`: `gate App.g close` at rank
/// 1, `dirty App.x 2 -> 5` at rank 2. `App.acc` is frozen; the trigger is lost
/// and the ring still holds `3`.
///
/// **Block 5.** `App.open` dirties `false -> true`.
///
/// **Block 6.** Step 1a reads it. Three things happen in this one commit, in
/// this order:
///
/// 1. the effective reopen is recorded, so `gate App.g open` will be emitted;
/// 2. `reset_on_reopen` fires — still in step 1a — and clears the self history,
///    so every `past(k)` read answers from the fallbacks again;
/// 3. step 1b emits `dirty App.x 5 -> 9`.
///
/// Step 3 then evaluates `App.acc` because its value input is dirty, and it
/// reads `9 + fallback(0)` = `9`. The `9` is the whole assertion: `12` would
/// mean the clear happened after the read (or not at all), and no line at all
/// would mean the recurrent had snapped rather than been triggered.
#[test]
fn reopen_reset() {
    let pair = witness("reopen_reset");
    let registry = registry_of(&[(0xd0, open_when_true()), (0xd2, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], Value::I64(2));
        engine.commit();
        write(engine, &["App", "open"], Value::Bool(false));
        engine.commit();
        write(engine, &["App", "x"], Value::I64(5));
        engine.commit();
        write(engine, &["App", "open"], Value::Bool(true));
        engine.commit();
        write(engine, &["App", "x"], Value::I64(9));
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "acc"]), "9");
    };

    let from_text = traced(&parsed("reopen_reset", &pair.source), &registry, script);

    assert_trace("reopen_reset", &from_text, &pair.golden);

    // The reset and the evaluation that reads its result are in ONE block: the
    // `gate` line and the `eval` line are separated by a single `dirty` line and
    // no `publish`.
    let reopen = from_text.find("gate App.g open").expect("the gate reopens");
    let post_gap = from_text
        .find("eval App.acc -> 9")
        .expect("the recurrent evaluates over fallbacks");
    let published = from_text[reopen..post_gap].contains("publish");

    assert!(reopen < post_gap && !published, "{from_text}");

    let from_api = traced(&built_reopen_reset(), &registry, script);

    assert_trace("reopen_reset (constructed)", &from_api, &pair.golden);
}

/// `reopen_reset.duir`, built through the `ductus-ir` API instead of parsed.
fn built_reopen_reset() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(&["App", "open"], Prim::Bool, ValueLiteral::True)),
            Entry::Gate(gate(
                &["App", "g"],
                0xd0,
                &[&["App", "open"]],
                &[&["App", "acc"]],
                None,
            )),
            Entry::Cell(input(&["App", "x"], Prim::I64, int("1"))),
            Entry::Cell(gated(
                resetting(recurrent(
                    &["App", "acc"],
                    Prim::I64,
                    0xd2,
                    &[&["App", "x"]],
                    1,
                    int("0"),
                )),
                &["App", "g"],
            )),
        ],
    )])
}

/// §13.9.7's deferred row, both halves: *"Deferreds do not dispatch; gate-close
/// **cancels** any in-flight runs — no publication, no barrier — and the frozen
/// cell serves its last-published value"*, and on gate-open the snap DISPATCHES
/// a fresh run rather than evaluating synchronously.
///
/// `depth 2` is what makes the cancellation observable at all: a run needs to be
/// in flight across a commit boundary for a close to catch it. `B@d3` is the
/// one-argument sum, i.e. the identity, so a landing's value is readable as the
/// input it was snapshotted from.
///
/// The commit index matters here and is written out block by block, because a
/// run's `due` is measured against it. The startup commit is index 0.
///
/// # Derivation
///
/// **Block 1 (startup, index 0).** Construction evaluates `App.echo`
/// SYNCHRONOUSLY, like a derived (C-23), giving `1`; it does not dispatch,
/// because a latency governs commits and construction is not one. The gate
/// begins open. Nothing is dirty, nothing dispatches, and the block is empty.
///
/// **Block 2 (index 1).** `App.x` dirties `1 -> 5`. Step 4½ then finds
/// `App.echo`'s provenance dirty — its input's dirty bit is set (C-28) — and
/// dispatches: the behavior runs over the pending `5` and the value is HELD with
/// `due = 1 + 2 = 3`. Dispatch publishes nothing, so it emits NO line (R24), and
/// `has_staged_work` is now true.
///
/// **Block 3 (index 2).** `App.open` dirties `true -> false`; step 1a still read
/// the live `true`, so the gate has not moved and the held run survives. Nothing
/// lands: the run is due at 3.
///
/// **Block 4 (index 3).** Step 1a reads the published `false` and `App.g`
/// closes. The cancellation runs THERE, before step 1b: the held run's cell is
/// no longer effectively active, so the hold is DROPPED. Step 1b then reaches
/// `App.echo` at the very commit its run was due and finds nothing — no landing,
/// no `dirty App.echo` line. `gate App.g close` is the block's only line, and
/// `has_staged_work` is false. Cancellation, not postponement: a postponed run
/// would still be waiting.
///
/// **Block 5 (index 4).** `App.open` dirties `false -> true`.
///
/// **Block 6 (index 5).** Step 1a reads it: `gate App.g open`. Step 4½ then
/// treats the effective open as provenance-dirt and dispatches a FRESH run over
/// the current pending `App.x = 5`, held with `due = 5 + 2 = 7`. Still no line —
/// dispatch is not a publication — and `has_staged_work` is true again.
///
/// **Block 7 (index 6).** Nothing at all: the run is due at 7, one commit away.
/// The empty block is the latency being spent.
///
/// **Block 8 (index 7).** The run lands. A landing IS a dirty root by the
/// corpus's own words, so it reuses the `dirty` line and reads
/// `dirty App.echo 1 -> 5` — `1` being the construction value the frozen cell
/// served throughout. No `land` event exists, because naming the mechanism that
/// produced a value would record implementation shape (R24, R75).
#[test]
fn gate_deferred_cancel() {
    let pair = witness("gate_deferred_cancel");
    let registry = registry_of(&[(0xd0, open_when_true()), (0xd3, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], Value::I64(5));
        engine.commit();

        // A run is in flight, due two commits out.
        assert!(engine.has_staged_work());

        write(engine, &["App", "open"], Value::Bool(false));
        engine.commit();

        assert!(engine.has_staged_work());

        engine.commit();

        // The close cancelled it at the very commit it was due: no landing, and
        // nothing left waiting.
        assert!(!engine.has_staged_work());

        write(engine, &["App", "open"], Value::Bool(true));
        engine.commit();

        assert!(!engine.has_staged_work());

        engine.commit();

        // The open dispatched a FRESH run rather than reviving the cancelled
        // one, so the readiness surface answers true again.
        assert!(engine.has_staged_work());

        engine.commit();
        engine.commit();

        assert!(!engine.has_staged_work());

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "echo"]), "5");
    };

    let from_text = traced(
        &parsed("gate_deferred_cancel", &pair.source),
        &registry,
        script,
    );

    assert_trace("gate_deferred_cancel", &from_text, &pair.golden);

    // Two runs were dispatched and exactly ONE landed. The cancelled one left
    // no trace of any kind, which is what "no publication, no barrier" means
    // observationally.
    assert_eq!(
        from_text.matches("dirty App.echo").count(),
        1,
        "{from_text}"
    );

    let from_api = traced(&built_gate_deferred_cancel(), &registry, script);

    assert_trace(
        "gate_deferred_cancel (constructed)",
        &from_api,
        &pair.golden,
    );
}

/// `gate_deferred_cancel.duir`, built through the `ductus-ir` API instead of
/// parsed.
fn built_gate_deferred_cancel() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(&["App", "open"], Prim::Bool, ValueLiteral::True)),
            Entry::Gate(gate(
                &["App", "g"],
                0xd0,
                &[&["App", "open"]],
                &[&["App", "echo"]],
                None,
            )),
            Entry::Cell(input(&["App", "x"], Prim::I64, int("1"))),
            Entry::Cell(gated(
                deferred(&["App", "echo"], Prim::I64, 0xd3, &[&["App", "x"]], 2),
                &["App", "g"],
            )),
        ],
    )])
}

/// A predicate whose INPUT changes but whose BOOLEAN does not: no transition, no
/// `gate` line, no snap.
///
/// The predicate here is `level > 0` rather than a passthrough, which is what
/// lets the script move the input twice and the answer once. C-50 has the engine
/// run every predicate every commit — a pure function over live values, so an
/// unchanged answer is unobservable — and this witness is the claim that
/// "unobservable" is literal: the extra runs leave nothing behind.
///
/// # Derivation
///
/// **Block 1 (startup).** `held = 2 + 1 = 3`. The predicate reads
/// `App.level = 5`, answers `true`, and `App.g` begins open.
///
/// **Block 2.** `App.level` dirties `5 -> 7`. The predicate has not seen it yet
/// (step 1a reads prior-commit values), and `App.held`'s own input `App.x` is
/// clean, so the block is one `dirty` line.
///
/// **Block 3.** Nothing is written. Step 1a now reads `7` and answers `true` —
/// the same answer as last commit. No transition, so no `gate` line and no snap,
/// and `App.held` does not evaluate. THE EMPTY BLOCK IS THE ASSERTION: a
/// predicate input changed, the predicate ran, and the graph did not move.
///
/// **Block 4.** `App.level` dirties `7 -> -1`. Step 1a read the live `7` and
/// still answered `true`.
///
/// **Block 5.** Step 1a reads `-1`, answers `false`, and the boolean finally
/// changes: `gate App.g close`. Nothing is staged, so the gate line stands
/// alone, and `App.held` freezes at `3`.
#[test]
fn predicate_stable() {
    let pair = witness("predicate_stable");
    let registry = registry_of(&[(0xd0, positive_i64()), (0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "level"], Value::I64(7));
        engine.commit();
        engine.commit();
        write(engine, &["App", "level"], Value::I64(-1));
        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        assert_eq!(read(&snapshot, &["App", "held"]), "3");
    };

    let from_text = traced(&parsed("predicate_stable", &pair.source), &registry, script);

    assert_trace("predicate_stable", &from_text, &pair.golden);

    // Two input changes and one predicate run per commit, against ONE line: the
    // trace records activation changes, never predicate runs.
    assert_eq!(from_text.matches("gate App.g").count(), 1, "{from_text}");
    assert!(!from_text.contains("eval App.held"), "{from_text}");

    let from_api = traced(&built_predicate_stable(), &registry, script);

    assert_trace("predicate_stable (constructed)", &from_api, &pair.golden);
}

/// `predicate_stable.duir`, built through the `ductus-ir` API instead of parsed.
fn built_predicate_stable() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(&["App", "level"], Prim::I64, int("5"))),
            Entry::Gate(gate(
                &["App", "g"],
                0xd0,
                &[&["App", "level"]],
                &[&["App", "held"]],
                None,
            )),
            Entry::Cell(input(&["App", "x"], Prim::I64, int("2"))),
            Entry::Cell(gated(
                derived(&["App", "held"], Prim::I64, 0xd1, &[&["App", "x"]]),
                &["App", "g"],
            )),
        ],
    )])
}

/// R76's whole claim in one commit block: a host write, a gate transition and a
/// deferred landing arrive at INTERLEAVED entry ranks, and the block's root
/// lines are ONE rank sequence.
///
/// Step 1 is one rank-ordered walk with every root source folded into it — host
/// writes, gate transitions, deferred landings — rather than three passes in a
/// fixed source order (R76). The module is built to make the difference visible:
/// the three roots are declared at ranks 0, 1 and 2 in an order no source-major
/// walk would produce, so a walk grouped by SOURCE would print them in some
/// other order and this golden would fail.
///
/// `App.g` guards nothing and `App.f` feeds nothing. That is deliberate — the
/// witness is about the SHAPE of the root sequence, so every downstream effect
/// is removed and the three lines stand alone.
///
/// What a reader CANNOT tell from the block is the other half of R75: the
/// landing renders as a `dirty` line indistinguishable from the host's write,
/// because a downstream consumer cannot distinguish them either, and the trace
/// is the consumer's view.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction evaluates the deferred SYNCHRONOUSLY,
/// like a derived (C-23): `App.f = App.b(0) + 1 = 1`. The predicate then reads
/// `App.k = 0`, answers `false`, and `App.g` begins CLOSED — a starting state,
/// not a transition, so no `gate` line (§13.2.6/016-144). Untraced; the first
/// commit finds nothing dirty.
///
/// **Block 2.** `App.b` and `App.k` are both written. Step 1a's predicate reads
/// PRIOR-COMMIT values, so it still sees `App.k = 0` and still answers `false`:
/// no transition this commit (R73). The two `dirty` lines come out in rank
/// order — `App.b` is rank 3 and `App.k` is rank 4. `App.f`'s provenance is
/// dirty (`App.b` moved), so step 4½ dispatches it over the newly-committed `7`,
/// holding `8` for commit index + 1 (C-28, R74a). A dispatch publishes nothing,
/// so it leaves no line.
///
/// **Block 3 — the claim.** Three roots, three sources, one rank sequence:
///
/// - `dirty App.a 0 -> 5` — the host write, rank 0;
/// - `gate App.g open` — step 1a now reads `App.k = 3` and the boolean flips,
///   rank 1;
/// - `dirty App.f 1 -> 8` — the held run lands at its due commit, rank 2.
///
/// Nothing evaluates: the gate guards no cell, and no cell reads `App.a` or
/// `App.f`. The block is the root sequence and `publish`, which is exactly what
/// makes the ordering the whole assertion.
#[test]
fn rank_interleaved_roots() {
    let pair = witness("rank_interleaved_roots");
    let registry = registry_of(&[(0xd0, positive_i64()), (0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "b"], Value::I64(7));
        write(engine, &["App", "k"], Value::I64(3));
        engine.commit();
        write(engine, &["App", "a"], Value::I64(5));
        engine.commit();
    };

    let from_text = traced(
        &parsed("rank_interleaved_roots", &pair.source),
        &registry,
        script,
    );

    assert_trace("rank_interleaved_roots", &from_text, &pair.golden);

    // The interleave on its own: the gate line sits BETWEEN two `dirty` lines
    // that no single source produced, which is what rules out a per-source walk.
    let write_line = from_text
        .find("dirty App.a 0 -> 5")
        .expect("the host write is a root");
    let transition = from_text
        .find("gate App.g open")
        .expect("the transition is a root");
    let landing = from_text
        .find("dirty App.f 1 -> 8")
        .expect("the landing is a root");

    assert!(write_line < transition, "{from_text}");
    assert!(transition < landing, "{from_text}");

    let from_api = traced(&built_rank_interleaved_roots(), &registry, script);

    assert_trace(
        "rank_interleaved_roots (constructed)",
        &from_api,
        &pair.golden,
    );
}

/// `rank_interleaved_roots.duir`, built through the `ductus-ir` API instead of
/// parsed.
fn built_rank_interleaved_roots() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(&["App", "a"], Prim::I64, int("0"))),
            Entry::Gate(gate(&["App", "g"], 0xd0, &[&["App", "k"]], &[], None)),
            Entry::Cell(deferred(
                &["App", "f"],
                Prim::I64,
                0xd1,
                &[&["App", "b"]],
                1,
            )),
            Entry::Cell(input(&["App", "b"], Prim::I64, int("0"))),
            Entry::Cell(input(&["App", "k"], Prim::I64, int("0"))),
        ],
    )])
}

/// §6.3's witness 12 — the SELF-CONDITIONAL gate: a gate whose predicate reads a
/// cell that gate guards. **RULED (a) by the owner, 2026-08-24.**
///
/// SPEC §13.9.6 and 022-36/37/38/39 permit the shape, and R73's rider — a
/// load-time rejection of it — was withdrawn: the check and its
/// `gate_predicate_reads_guarded` class are out of the tree, and NOTHING
/// replaces them, because step 1a already says what the predicate reads. The
/// pattern is legal, and this witness is what it does.
///
/// `App.g`'s predicate is `positive_i64` over `App.d`; `App.d` is
/// `App.n + 1` and is guarded by `App.g`. Two laws are visible in one trace: the
/// transition trails its own guarded change by exactly ONE COMMIT (step 1a reads
/// prior-commit values, R73), and a gate that decides its own subtree LATCHES —
/// once closed, the value its predicate reads is frozen, so nothing can reopen
/// it.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction evaluates `App.d = n(1) + 1 = 2`. The
/// predicate then runs over that constructed value — `2 > 0` — so `App.g` begins
/// OPEN (§13.2.6). Beginning open is a STATE, not a transition, so no `gate`
/// line is emitted, here or at step 4's first commit, which re-runs the same
/// predicate over the same buffer and finds nothing changed. The block is
/// `commit_begin`/`publish`/`commit_end`.
///
/// **Block 2 — the change.** `App.n` dirties `1 -> -1`. Step 1a runs FIRST and
/// reads the PRIOR-commit `App.d`, which is still `2`, so the gate holds open;
/// `App.d` is therefore effectively active when step 3 reaches it and evaluates
/// to `-1 + 1 = 0`. The value that will close the gate is produced UNDER the
/// open gate — that is the whole of the one-commit trail.
///
/// **Block 3 — the transition, one commit later.** Nothing is written. Step 1a
/// reads `App.d = 0`, `0 > 0` is false, and the gate CLOSES: `gate App.g close`
/// is the block's only root line. `App.d` is no longer effectively active, so
/// step 3 skips it — activation before staleness (RW-9) — and it emits nothing.
///
/// **Block 4 — the freeze, with a live input.** `App.n` dirties `-1 -> 5`: the
/// input is not guarded, so the write stages and the root line appears. `App.d`
/// is frozen and does not recompute, so no `eval` follows it (§4.7's Model B).
///
/// **Block 5 — the latch.** Another quiescent commit. The predicate reads
/// `App.d`, which is still the frozen `0`, answers false again, and nothing
/// transitions. The block is empty. A gate whose predicate depends only on cells
/// it guards can never reopen, because the values that would reopen it are the
/// ones it froze — that is a consequence of the ruling, and it is asserted here
/// rather than left to be discovered.
#[test]
fn self_conditional_gate() {
    let pair = witness("self_conditional_gate");
    let registry = registry_of(&[(0xd0, positive_i64()), (0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "n"], Value::I64(-1));
        engine.commit();
        engine.commit();
        write(engine, &["App", "n"], Value::I64(5));
        engine.commit();
        engine.commit();

        let snapshot = engine.acquire_snapshot();

        // The frozen guarded value, and the live input that could not reach it.
        assert_eq!(read(&snapshot, &["App", "d"]), "0");
        assert_eq!(read(&snapshot, &["App", "n"]), "5");
    };

    let from_text = traced(
        &parsed("self_conditional_gate", &pair.source),
        &registry,
        script,
    );

    assert_trace("self_conditional_gate", &from_text, &pair.golden);

    // The two halves on their own: the guarded cell evaluated exactly once —
    // in the commit BEFORE the transition its own value caused — and the gate
    // never came back.
    assert_eq!(from_text.matches("eval App.d").count(), 1, "{from_text}");
    assert!(!from_text.contains("gate App.g open"), "{from_text}");

    let from_api = traced(&built_self_conditional_gate(), &registry, script);

    assert_trace(
        "self_conditional_gate (constructed)",
        &from_api,
        &pair.golden,
    );
}

/// `self_conditional_gate.duir`, built through the `ductus-ir` API instead of
/// parsed.
fn built_self_conditional_gate() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            Entry::Cell(input(&["App", "n"], Prim::I64, int("1"))),
            Entry::Gate(gate(
                &["App", "g"],
                0xd0,
                &[&["App", "d"]],
                &[&["App", "d"]],
                None,
            )),
            Entry::Cell(gated(
                derived(&["App", "d"], Prim::I64, 0xd1, &[&["App", "n"]]),
                &["App", "g"],
            )),
        ],
    )])
}
