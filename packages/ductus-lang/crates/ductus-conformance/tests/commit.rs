//! Witness tests for the commit engine — the nine M0 commit witnesses
//! (corpus/README.md, R25, M0.md §6).
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
//!    (M0.md §6's dual-construction track).
//!
//! `trap_abort` is the one exception: past an abort there is no trace to
//! observe (D8, §13.13.1), so it asserts the panic instead of claims 4 and 5.
//!
//! # Every golden here is HAND-DERIVED
//!
//! Not one of these traces was captured from a run. Each is written out from the
//! semantics contract — SPEC §13.10.2's six steps as pinned in M0.md §4 — and
//! the derivation is the test fn's doc comment, block by block: which cells
//! dirty and why, which deriveds evaluate and in which order, and which do not.
//! A golden captured from the engine would assert only that the engine does what
//! the engine does.
//!
//! The three shape rules every derivation below applies:
//!
//! - A trace OPENS with the empty block `commit_begin`/`publish`/`commit_end` —
//!   §13.14.1 step 4's first commit, with construction (step 3) already done and
//!   untraced, so nothing is dirty and nothing evaluates.
//! - Inside a block: `dirty` lines for written INPUT cells whose staged value
//!   differs BITWISE from the committed one (R65), in RANK order (declaration
//!   order, R64 — never write order); then `eval` lines for the deriveds that
//!   actually evaluate — any direct dependency dirty — in static topological
//!   order with rank as the tiebreak. A skipped evaluation emits NOTHING (R24).
//! - Values render canonically (R34), and a commit with no dirt is the empty
//!   block.

use std::rc::Rc;

use ductus_conformance::runner::{RecordingSink, Witness};
use ductus_conformance::stub_library::{StubRegistry, add1_i64, double_i32, sum_i64};
use ductus_conformance::trace_compare::assert_trace;
use ductus_ir::diag::Span;
use ductus_ir::graph::{Cell, CellKind, Entry, Scope, ValueLiteral};
use ductus_ir::module::{BehaviorsSection, GraphSection, Module, TypesSection};
use ductus_ir::path::{CellPath, Segment};
use ductus_ir::text::emitter::emit;
use ductus_ir::text::lexer::lex;
use ductus_ir::text::parser::parse;
use ductus_ir::types::{Prim, TypeTag};
use ductus_runtime::engine::Engine;
use ductus_runtime::loader::load;
use ductus_runtime::provider::Behavior;
use ductus_runtime::value::Value;

/// Claim 1, and the parsed module for claims 2–4.
fn parsed(witness: &str, source: &str) -> Module {
    let tokens = lex(source).expect("the witness lexes");
    let module = parse(tokens).expect("the witness parses");

    assert_eq!(
        emit(&module),
        source,
        "witness `{witness}`: the corpus file is not canonical text"
    );

    module
}

/// Loads `module` under `registry`, drives `script` against a fresh engine and a
/// fresh sink, and hands back everything the sink heard.
///
/// The two construction routes of claim 5 differ ONLY in the module they pass
/// here: same registry, same script, same fresh engine — so a trace difference
/// can only be a module difference.
fn traced<S>(module: &Module, registry: &StubRegistry, script: S) -> String
where
    S: Fn(&mut Engine<'_>),
{
    let graph = load(module, registry).expect("the witness module loads");
    let mut sink = RecordingSink::new();
    let mut engine = Engine::start(graph, &mut sink);

    script(&mut engine);
    engine.teardown();

    sink.into_trace()
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

/// A plain dotted path — the only path shape the M0 slice admits (D7).
fn path(names: &[&str]) -> CellPath {
    CellPath::new(
        names
            .iter()
            .map(|text| Segment::Name {
                text: (*text).to_string(),
                ordinal: None,
            })
            .collect(),
    )
}

/// A cell with every clause absent; the two builders below fill exactly the
/// clauses their kind admits.
fn cell(kind: CellKind, names: &[&str], prim: Prim) -> Cell {
    Cell {
        kind,
        path: path(names),
        span: Span::new(0, 0),
        tag: TypeTag::Prim(prim),
        uses: None,
        inputs: Vec::new(),
        depth: None,
        reset_on_reopen: false,
        init: None,
        gate: None,
        combiner: None,
        else_value: None,
        members: Vec::new(),
    }
}

fn input(names: &[&str], prim: Prim, init: &str) -> Cell {
    let mut built = cell(CellKind::Input, names, prim);
    built.init = Some(ValueLiteral::int(init.to_string()));

    built
}

fn derived(names: &[&str], prim: Prim, uses: u32, inputs: &[&[&str]]) -> Cell {
    let mut built = cell(CellKind::Derived, names, prim);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();

    built
}

/// One scope holding `cells`, with the empty clause sets the M0 slice requires.
fn scope(names: &[&str], cells: Vec<Cell>) -> Scope {
    Scope {
        path: path(names),
        span: Span::new(0, 0),
        exposes: Vec::new(),
        effects: Vec::new(),
        reset_on_reopen: Vec::new(),
        entries: cells.into_iter().map(Entry::Cell).collect(),
    }
}

/// A module named `App` with an empty type table and an empty behavior table —
/// what every M0 witness carries (D5: the provider registry is the whole
/// resolvability answer).
fn module(scopes: Vec<Scope>) -> Module {
    Module {
        name: "App".to_string(),
        span: Span::new(0, 0),
        types: TypesSection {
            span: Span::new(0, 0),
            type_defs: Vec::new(),
        },
        graph: GraphSection {
            span: Span::new(0, 0),
            scopes,
        },
        behaviors: BehaviorsSection {
            span: Span::new(0, 0),
        },
    }
}

/// A registry holding one stub — the shape most witnesses need.
fn registry_of(bindings: &[(u32, Behavior)]) -> StubRegistry {
    let mut registry = StubRegistry::new();

    for (handle, behavior) in bindings {
        registry.register(*handle, Rc::clone(behavior));
    }

    registry
}

/// One input, one derived doubling it: `write count = 7` then `commit`.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction evaluates `App.doubled` once over the
/// init — `2 * 0 = 0` — untraced, because construction is not a commit. Step 4's
/// first commit then finds `App.count`'s staged value equal to its committed
/// one, so nothing dirties, nothing evaluates, and the block is
/// `commit_begin`/`publish`/`commit_end`.
///
/// **Block 2.** The write stages `7`; at step 1 that differs bitwise from the
/// committed `0`, so `App.count` dirties and its line reads `0 -> 7`.
/// `App.doubled` reads a dirty dependency, so step 3 evaluates it over the
/// PENDING value the dirty step just wrote — the staged `7`, never the stale
/// `0` — giving `14`.
#[test]
fn single_derived() {
    let witness = Witness::load("single_derived");
    let registry = registry_of(&[(0xd1, double_i32())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "count"], Value::I32(7));
        engine.commit();
    };

    let from_text = traced(
        &parsed("single_derived", &witness.source),
        &registry,
        script,
    );

    assert_trace("single_derived", &from_text, &witness.golden);

    let from_api = traced(&built_single_derived(), &registry, script);

    assert_trace("single_derived (constructed)", &from_api, &witness.golden);
}

/// `single_derived.duir`, built through the `ductus-ir` API instead of parsed.
fn built_single_derived() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            input(&["App", "count"], Prim::I32, "0"),
            derived(&["App", "doubled"], Prim::I32, 0xd1, &[&["App", "count"]]),
        ],
    )])
}

/// The same module, driven by a commit that stages nothing and a commit that
/// stages the value the cell already holds.
///
/// # Derivation
///
/// **Block 1 (startup).** As in `single_derived`: construction is untraced, and
/// step 4's commit finds nothing dirty.
///
/// **Block 2.** No write happened, so every input's staged value is its
/// committed value, step 1 dirties nothing, step 3 evaluates nothing — and step
/// 5 STILL publishes. That is §13.14.4's idempotence: a commit with no dirty
/// cells publishes a fresh snapshot whose values are identical. The empty block
/// is the observable form of "it ran and changed nothing", which is why a
/// no-dirt commit is not the same as no commit at all.
///
/// **Block 3.** The write stages `0` — the init, and therefore bitwise equal to
/// the committed value. Step 1 compares staged against the previous COMMIT
/// (§13.10.1), not against the write history, so an equal write dirties nothing
/// and `App.doubled` never evaluates. A third empty block.
#[test]
fn empty_commit() {
    let witness = Witness::load("empty_commit");
    let registry = registry_of(&[(0xd1, double_i32())]);
    let script = |engine: &mut Engine<'_>| {
        engine.commit();
        write(engine, &["App", "count"], Value::I32(0));
        engine.commit();
    };

    let from_text = traced(&parsed("empty_commit", &witness.source), &registry, script);

    assert_trace("empty_commit", &from_text, &witness.golden);

    let from_api = traced(&built_empty_commit(), &registry, script);

    assert_trace("empty_commit (constructed)", &from_api, &witness.golden);
}

/// `empty_commit.duir`, built through the `ductus-ir` API instead of parsed.
fn built_empty_commit() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            input(&["App", "count"], Prim::I32, "0"),
            derived(&["App", "doubled"], Prim::I32, 0xd1, &[&["App", "count"]]),
        ],
    )])
}

/// A three-link chain `a → b → c → d`: one write settles all three links inside
/// ONE commit, in topological order.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction walks the static order once — `b = 1`,
/// `c = 2`, `d = 3` off the init `0` — untraced, and step 4's commit finds
/// nothing dirty.
///
/// **Block 2.** `App.a` dirties `0 -> 5`. Step 3 walks the static order, which
/// for a chain is forced: `b` before `c` before `d`. `b` reads the pending `a`
/// (the staged `5`) → `6`; `c` reads `b`'s PENDING value — the `6` step 3 just
/// wrote, never the committed `2` — → `7`; `d` likewise → `8`. Each result also
/// differs from its committed value, so each link's dirty bit is set and the
/// next link evaluates. The arithmetic is what pins the order: a walk that
/// evaluated `c` before `b` would print `3`, not `7`.
#[test]
fn chain() {
    let witness = Witness::load("chain");
    let registry = registry_of(&[(0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "a"], Value::I64(5));
        engine.commit();
    };

    let from_text = traced(&parsed("chain", &witness.source), &registry, script);

    assert_trace("chain", &from_text, &witness.golden);

    let from_api = traced(&built_chain(), &registry, script);

    assert_trace("chain (constructed)", &from_api, &witness.golden);
}

/// `chain.duir`, built through the `ductus-ir` API instead of parsed.
fn built_chain() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            input(&["App", "a"], Prim::I64, "0"),
            derived(&["App", "b"], Prim::I64, 0xd1, &[&["App", "a"]]),
            derived(&["App", "c"], Prim::I64, 0xd1, &[&["App", "b"]]),
            derived(&["App", "d"], Prim::I64, 0xd1, &[&["App", "c"]]),
        ],
    )])
}

/// Two arms over one input, joined: glitch-freedom in one commit.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction settles the whole diamond off the init
/// `0` — `left = 1`, `right = 1`, `join = 2` — untraced; step 4 finds nothing
/// dirty.
///
/// **Block 2.** `App.a` dirties `0 -> 5`. Step 3's static order is `left`,
/// `right`, `join`: the arms are the same topological level and tie-break by
/// rank (R64), and `join` waits for both. `left` and `right` each read the
/// pending `5` → `6`. `join` reads BOTH arms out of the pending buffer, where
/// step 3 has already written this commit's `6` and `6`, so it sums `12`.
///
/// The two claims live in that one line. Glitch-freedom: `join` never sees one
/// new arm and one stale one — a half-updated read would print `7`
/// (`6 + 1`), not `12`. And `join` evaluates EXACTLY ONCE, though two of its
/// dependencies changed: step 3 is a single ordered walk, not a propagation
/// cascade, so there is one `eval App.join` line and there can never be two.
#[test]
fn diamond() {
    let witness = Witness::load("diamond");
    let registry = registry_of(&[(0xd1, add1_i64()), (0xd2, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "a"], Value::I64(5));
        engine.commit();
    };

    let from_text = traced(&parsed("diamond", &witness.source), &registry, script);

    assert_trace("diamond", &from_text, &witness.golden);
    assert_eq!(from_text.matches("eval App.join").count(), 1);

    let from_api = traced(&built_diamond(), &registry, script);

    assert_trace("diamond (constructed)", &from_api, &witness.golden);
}

/// `diamond.duir`, built through the `ductus-ir` API instead of parsed.
fn built_diamond() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            input(&["App", "a"], Prim::I64, "0"),
            derived(&["App", "left"], Prim::I64, 0xd1, &[&["App", "a"]]),
            derived(&["App", "right"], Prim::I64, 0xd1, &[&["App", "a"]]),
            derived(
                &["App", "join"],
                Prim::I64,
                0xd2,
                &[&["App", "left"], &["App", "right"]],
            ),
        ],
    )])
}

/// `|x|` over one `i32` — a SINGLE-USE stub, written here rather than in the
/// shared library so the reason `value_gating`'s golden holds sits beside the
/// golden.
///
/// It is the smallest behavior that maps two different inputs to one output,
/// which is what makes an equal-result gate observable at all.
///
/// # Panics
///
/// On any other argument shape — a harness-author bug, loud rather than papered
/// over. `wrapping_abs` keeps `i32::MIN` total, for the reason the library
/// stubs give: a fixture must not trap by accident.
fn abs_i32() -> Behavior {
    Rc::new(|inputs| match inputs {
        [Value::I32(payload)] => Value::I32(payload.wrapping_abs()),
        other => panic!("abs_i32 wants one i32 input, got {other:?}"),
    })
}

/// The R24 suppressed-evaluation law and the §13.2.4.4 output-gating law in one
/// file: a derived that re-evaluates to the value it already held stops the
/// propagation dead.
///
/// # Derivation
///
/// **Block 1 (startup).** Construction: `mag = |3| = 3`, `report = 2 * 3 = 6`.
/// Untraced; step 4 finds nothing dirty.
///
/// **Block 2.** The write stages `-3`, which differs bitwise from the committed
/// `3`, so `App.x` dirties `3 -> -3`. `App.mag` has a dirty dependency and
/// evaluates: `|-3| = 3` — ONE `eval` line. Step 3 then compares that result
/// against `mag`'s committed value, `3`; they are bit-identical, so `mag`'s
/// dirty bit is set to FALSE (§13.2.4.4's value-change gating on outputs).
/// `App.report` is walked next, finds its only dependency clean, and does not
/// evaluate — and a skipped evaluation emits NOTHING (R24), so the block holds
/// one `dirty` line and one `eval` line. The MISSING `eval App.report` is the
/// whole assertion.
///
/// **Block 3.** The second write stages `-3` again. Step 1 compares staged
/// against the previous COMMIT, which is now `-3`, so nothing dirties: the
/// empty block. Writing a value twice is not writing twice — dirtiness is a
/// property of the committed state, not of the write history.
#[test]
fn value_gating() {
    let witness = Witness::load("value_gating");
    let registry = registry_of(&[(0xd1, abs_i32()), (0xd2, double_i32())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "x"], Value::I32(-3));
        engine.commit();
        write(engine, &["App", "x"], Value::I32(-3));
        engine.commit();
    };

    let from_text = traced(&parsed("value_gating", &witness.source), &registry, script);

    assert_trace("value_gating", &from_text, &witness.golden);
    assert!(!from_text.contains("eval App.report"));

    let from_api = traced(&built_value_gating(), &registry, script);

    assert_trace("value_gating (constructed)", &from_api, &witness.golden);
}

/// `value_gating.duir`, built through the `ductus-ir` API instead of parsed.
fn built_value_gating() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            input(&["App", "x"], Prim::I32, "3"),
            derived(&["App", "mag"], Prim::I32, 0xd1, &[&["App", "x"]]),
            derived(&["App", "report"], Prim::I32, 0xd2, &[&["App", "mag"]]),
        ],
    )])
}

/// R64 twice over: `dirty` lines in rank order despite reversed writes, and
/// `eval` lines in DECLARATION order among same-level deriveds.
///
/// The module is built so neither claim can pass by accident. Each derived
/// reads its own input, and the deriveds are declared in the order OPPOSITE to
/// their inputs: `from_b` (which reads the second input) is declared before
/// `from_a` (which reads the first). So the eval order cannot be inherited from
/// the dirty order, from the input order, or from alphabetical order — only
/// declaration order produces it.
///
/// # Derivation
///
/// **Block 1 (startup).** Both deriveds construct off their init `0` → `1`
/// each. Untraced; step 4 finds nothing dirty.
///
/// **Block 2.** The script writes `b` FIRST, then `a`. Step 1 does not walk the
/// writes: it walks the cells in rank order — `a` is declared first, so
/// `dirty App.a 0 -> 10` precedes `dirty App.b 0 -> 20`. That is what makes a
/// trace invariant under permutation of same-commit writes to distinct cells.
///
/// Step 3 then walks the static order. Both deriveds are ready at once — each
/// depends only on an input — so the tiebreak decides, and it is rank, i.e.
/// declaration order: `from_b` (rank 2) before `from_a` (rank 3). `from_b`
/// reads the pending `20` → `21`; `from_a` reads the pending `10` → `11`. The
/// values make the order self-evidencing: a walk in input order would print
/// `11` first.
#[test]
fn tiebreak_order() {
    let witness = Witness::load("tiebreak_order");
    let registry = registry_of(&[(0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "b"], Value::I64(20));
        write(engine, &["App", "a"], Value::I64(10));
        engine.commit();
    };

    let from_text = traced(
        &parsed("tiebreak_order", &witness.source),
        &registry,
        script,
    );

    assert_trace("tiebreak_order", &from_text, &witness.golden);

    let from_api = traced(&built_tiebreak_order(), &registry, script);

    assert_trace("tiebreak_order (constructed)", &from_api, &witness.golden);
}

/// `tiebreak_order.duir`, built through the `ductus-ir` API instead of parsed.
fn built_tiebreak_order() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            input(&["App", "a"], Prim::I64, "0"),
            input(&["App", "b"], Prim::I64, "0"),
            derived(&["App", "from_b"], Prim::I64, 0xd1, &[&["App", "b"]]),
            derived(&["App", "from_a"], Prim::I64, 0xd1, &[&["App", "a"]]),
        ],
    )])
}

/// §13.10.4/§13.14.5: a transaction is grouping sugar over the staged buffer
/// (D3), and `abort()` is the only rollback path.
///
/// # Derivation
///
/// **Block 1 (startup).** `total = 0 + 0 = 0` at construction. Untraced; step 4
/// finds nothing dirty.
///
/// **Block 2.** The transaction applies both writes to the staged buffer at
/// close — nothing evaluated inside it, and nothing was visible until now. The
/// commit that follows sees exactly what two bare writes would have staged: `a`
/// dirties `0 -> 4` and `b` dirties `0 -> 6`, in rank order, and `App.total`
/// evaluates ONCE over both pending values → `10`. Two dirtied dependencies
/// still mean one evaluation, because step 3 walks cells, not edges.
///
/// **Block 3.** The second transaction stages `a = 100` into its own
/// accumulator and then aborts, so its pairs never reach the staged buffer —
/// `a`'s staged value is still the `4` the first transaction left. Step 1
/// compares that against the committed `4` and finds no difference: the empty
/// block. The abort is observable exactly as the ABSENCE of a dirty line.
#[test]
fn transaction_batch() {
    let witness = Witness::load("transaction_batch");
    let registry = registry_of(&[(0xd1, sum_i64())]);
    let script = |engine: &mut Engine<'_>| {
        engine
            .transaction(|transaction| {
                transaction.write_signal(&path(&["App", "a"]), Value::I64(4))?;
                transaction.write_signal(&path(&["App", "b"]), Value::I64(6))
            })
            .expect("both writes are legal");
        engine.commit();

        engine
            .transaction(|transaction| {
                transaction.write_signal(&path(&["App", "a"]), Value::I64(100))?;
                transaction.abort();

                Ok(())
            })
            .expect("an aborted transaction closes normally");
        engine.commit();
    };

    let from_text = traced(
        &parsed("transaction_batch", &witness.source),
        &registry,
        script,
    );

    assert_trace("transaction_batch", &from_text, &witness.golden);

    let from_api = traced(&built_transaction_batch(), &registry, script);

    assert_trace(
        "transaction_batch (constructed)",
        &from_api,
        &witness.golden,
    );
}

/// `transaction_batch.duir`, built through the `ductus-ir` API instead of
/// parsed.
fn built_transaction_batch() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            input(&["App", "a"], Prim::I64, "0"),
            input(&["App", "b"], Prim::I64, "0"),
            derived(
                &["App", "total"],
                Prim::I64,
                0xd1,
                &[&["App", "a"], &["App", "b"]],
            ),
        ],
    )])
}

/// Two subgraphs sharing nothing settle in ONE commit, each pass in rank order.
///
/// The four cells are INTERLEAVED — input, its derived, input, its derived — so
/// the two rank-ordered passes cannot be mistaken for one walk down the file:
/// the `dirty` pass visits ranks 0 and 2, the `eval` pass ranks 1 and 3.
///
/// # Derivation
///
/// **Block 1 (startup).** `da = 1` and `db = 1` at construction. Untraced; step
/// 4 finds nothing dirty.
///
/// **Block 2.** Both inputs are written, so both dirty, in rank order:
/// `App.a 0 -> 1` then `App.b 0 -> 2`. Step 3's static order holds both
/// deriveds — they are the same topological level (each reads an input only) and
/// tie-break by rank — so `App.da` evaluates to `2` and `App.db` to `3`, in that
/// order. One commit settles both islands; there is no second block, because
/// disjointness is a property of the graph, not a reason for a second pass.
#[test]
fn islands() {
    let witness = Witness::load("islands");
    let registry = registry_of(&[(0xd1, add1_i64())]);
    let script = |engine: &mut Engine<'_>| {
        write(engine, &["App", "a"], Value::I64(1));
        write(engine, &["App", "b"], Value::I64(2));
        engine.commit();
    };

    let from_text = traced(&parsed("islands", &witness.source), &registry, script);

    assert_trace("islands", &from_text, &witness.golden);

    let from_api = traced(&built_islands(), &registry, script);

    assert_trace("islands (constructed)", &from_api, &witness.golden);
}

/// `islands.duir`, built through the `ductus-ir` API instead of parsed.
fn built_islands() -> Module {
    module(vec![scope(
        &["App"],
        vec![
            input(&["App", "a"], Prim::I64, "0"),
            derived(&["App", "da"], Prim::I64, 0xd1, &[&["App", "a"]]),
            input(&["App", "b"], Prim::I64, "0"),
            derived(&["App", "db"], Prim::I64, 0xd1, &[&["App", "b"]]),
        ],
    )])
}

/// Passes its `i32` through, except on `13`, where it traps — the SINGLE-USE
/// stub of the trap witness, written here beside the claim it makes.
///
/// The condition is the point: an unconditional trap would abort during
/// CONSTRUCTION (§13.14.1 step 3), and the witness would then be about startup
/// rather than about a commit. Passing `0` through lets the graph start
/// normally, so the abort the test observes is the one the write causes.
///
/// # Panics
///
/// On `13` — that is the witness — and on any other argument shape, which is a
/// harness-author bug.
fn trap_on_thirteen() -> Behavior {
    Rc::new(|inputs| match inputs {
        [Value::I32(13)] => panic!("unlucky input"),
        [Value::I32(payload)] => Value::I32(*payload),
        other => panic!("trap_on_thirteen wants one i32 input, got {other:?}"),
    })
}

/// D8's trap shape: a behavior trap aborts the commit with a panic naming the
/// cell — and there is no trace to compare.
///
/// # Why this witness has no `.trace`
///
/// It is the one exception to corpus/README.md's pair convention, and the
/// exception is a LAW rather than an omission. §13.13.1 is explicit that the
/// runtime does not isolate traps: no errored-cell sentinel, no recovery, no
/// continuation. So the commit that traps never reaches `publish` or
/// `commit_end`, the run ends inside step 3, and what a golden would record is
/// a prefix of a block that the semantics say cannot complete. The observable
/// contract here is the PANIC (D8: harness-visible, carrying the cell path),
/// which is what this test asserts instead.
///
/// # Derivation
///
/// **The survival probe.** Construction evaluates `App.boom` over the init `0`,
/// which the stub passes through, so the graph starts and step 4's commit
/// publishes the empty block; a second bare commit publishes another. The stub's
/// totality at `0` is asserted directly first — under `should_panic` a probe
/// that trapped would look like a pass — and the two empty blocks then say the
/// startup path published normally.
///
/// **The abort.** `t` is written `13` and the commit begins: step 1 dirties
/// `App.t`, step 3 finds `App.boom` stale and invokes its behavior, which
/// panics. The engine re-panics with the trapping cell's path — nothing runs in
/// between, no value is invented, no state is repaired — so the harness sees
/// `behavior trap at App.boom: unlucky input`.
#[test]
#[should_panic(expected = "behavior trap at App.boom")]
fn trap_abort() {
    const STARTUP: &str = "commit_begin\npublish\ncommit_end\n";

    let source = Witness::source_only("trap_abort");
    let witness = parsed("trap_abort", &source);
    let registry = registry_of(&[(0xd1, trap_on_thirteen())]);

    // The stub is TOTAL at the init, asserted directly rather than through the
    // engine: a trap inside the probe below would carry the very message
    // `should_panic` expects, so this call is the only one that can tell
    // "startup survives" from "startup aborted".
    assert!(trap_on_thirteen()(&[Value::I32(0)]).bits_eq(&Value::I32(0)));

    let survives = traced(&witness, &registry, |engine: &mut Engine<'_>| {
        engine.commit();
    });

    assert_eq!(survives, format!("{STARTUP}{STARTUP}"));

    traced(&witness, &registry, |engine: &mut Engine<'_>| {
        write(engine, &["App", "t"], Value::I32(13));
        engine.commit();
    });
}
