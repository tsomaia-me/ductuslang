//! Semantic property tests (M0.md §6, track 4): randomized DAGs — drawn
//! dependency-safe, then EMITTED in shuffled order — and random write/commit
//! scripts, asserting the three laws a witness can only sample.
//!
//! A witness pins ONE trace of ONE module; these assert what must hold of every
//! module in the M0 slice:
//!
//! 1. **No double evaluation** — within one `commit_begin`..`commit_end` block,
//!    no cell evaluates twice (§13.10.2 step 3 walks a topological order once).
//! 2. **Oracle equivalence** — after every commit, every cell holds what a
//!    naive full recomputation from the current input values says it holds.
//!    That is glitch-freedom (no evaluation read a stale dependency) and
//!    completeness (no evaluation was wrongly skipped) in one claim.
//! 3. **Write-permutation invariance** — permuting a commit's writes to
//!    DISTINCT cells changes nothing in the trace, byte for byte (R64: the
//!    dirty walk is rank order, never write order).
//!
//! # Why the emission order is shuffled
//!
//! A generated graph is drawn dependency-safe — every derived reads cells
//! numbered strictly below it — because that is what makes acyclicity free. If
//! the entries were then WRITTEN in that numbering, entry order would already
//! be a topological order: no `inputs` clause would ever name a cell declared
//! later, and the loader's static order would equal declaration order for every
//! module the generator can draw. The three properties would then hold under a
//! scheduler that ignored dependencies entirely. Shuffling the emission order
//! separates the two: the numbering keeps the graph acyclic, while the entry
//! order the loader ranks by (R64) is a permutation of it, so forward references
//! are routine and the topological sort has to do the work.
//!
//! # Why these do not use the corpus
//!
//! A witness's `.duir`/`.trace` pair is hand-derived and byte-compared (R9);
//! there is nothing to hand-derive here, because the module is drawn at test
//! time. The generated modules are built through the `ductus-ir` Rust API —
//! the SAME route the dual-construction half of each witness takes — and every
//! claim is structural, asserted against the trace and against an oracle
//! rather than against a golden.
//!
//! # The oracle shares nothing with the engine
//!
//! [`oracle()`] is straight-line test code: it recomputes every derived from
//! scratch, in cell-number order, from the current input values, with no dirty
//! set, no static order, and no runtime type involved. If it agreed with the
//! engine by sharing the engine's code it would assert nothing.

use std::collections::HashSet;
use std::rc::Rc;

use ductus_conformance::runner::RecordingSink;
use ductus_conformance::stub_library::StubRegistry;
use ductus_ir::diag::Span;
use ductus_ir::graph::{Cell, CellKind, Entry, Scope, ValueLiteral};
use ductus_ir::module::{BehaviorsSection, GraphSection, Module, TypesSection};
use ductus_ir::path::{CellPath, Segment};
use ductus_ir::types::{Prim, TypeTag};
use ductus_runtime::engine::Engine;
use ductus_runtime::loader::load;
use ductus_runtime::value::Value;
use proptest::prelude::*;

/// The first behavior handle a generated module uses; derived `j` uses
/// `HANDLE_BASE + j`, so every derived has its OWN stub and two deriveds with
/// the same dependencies still differ (see [`registry()`]).
const HANDLE_BASE: u32 = 0xd0;

/// The widest plan the generator draws: three inputs plus five deriveds. The
/// emission shuffle draws this many raw bytes, so one fixed-length draw covers
/// every plan size.
const MAX_CELLS: usize = 8;

/// A generated graph and the script driven against it.
///
/// The cells are NUMBERED in dependency order: input `i` is number `i`, and
/// derived `j` is number `inputs + j`. Every derived depends on a nonempty set
/// of STRICTLY LOWER-NUMBERED cells, which is what makes the graph acyclic by
/// construction — no cycle is representable, so the generator never draws a
/// module the loader would reject.
///
/// A cell's number is NOT its rank. `emission` permutes the order the entries
/// are written into the graph section, and it is that entry order R64 ranks by,
/// so a dependency routinely lands after the derived that reads it.
#[derive(Debug, Clone)]
struct Plan {
    /// How many `i64` input cells, each `init 0`.
    inputs: usize,
    /// One entry per derived cell, holding its dependencies as cell numbers, in
    /// ascending order.
    deriveds: Vec<Vec<usize>>,
    /// One entry per commit, holding that commit's writes as
    /// `(input number, value)` pairs over DISTINCT inputs — the distinctness is
    /// what makes permutation a fair question (a second write to one cell
    /// overwrites the first, so reversing THAT batch changes the values, not
    /// just the order).
    commits: Vec<Vec<(usize, i64)>>,
    /// The order the cells are EMITTED into the graph section: a permutation of
    /// the cell numbers, so `emission[slot]` is the number of the cell the
    /// loader gives rank `slot`.
    emission: Vec<usize>,
}

impl Plan {
    /// Total cell count — inputs plus deriveds.
    fn cells(&self) -> usize {
        self.inputs + self.deriveds.len()
    }
}

/// The generator: 1..=3 inputs, 1..=5 deriveds, 1..=4 commits of 0..=3 writes,
/// and one shuffle of the emission order.
///
/// The random draws are RAW — a dependency mask per derived, an unbounded input
/// choice per write, a fixed-length byte vector for the shuffle — and the
/// mapping onto a legal plan happens here, in one place, deterministically.
/// Generating legality directly (a dependency subset whose size depends on the
/// derived's own index, a write index bounded by a count drawn in the same
/// tuple) would need nested dependent strategies for no extra coverage.
fn plan() -> impl Strategy<Value = Plan> {
    (
        1usize..=3,
        prop::collection::vec(any::<u8>(), 1..=5),
        prop::collection::vec(
            prop::collection::vec((any::<u8>(), -10i64..=10), 0..=3),
            1..=4,
        ),
        prop::collection::vec(any::<u8>(), MAX_CELLS),
    )
        .prop_map(|(inputs, masks, script, shuffle)| Plan {
            inputs,
            deriveds: dependencies(inputs, &masks),
            commits: script.iter().map(|batch| writes(inputs, batch)).collect(),
            emission: emission(inputs + masks.len(), &shuffle),
        })
}

/// One dependency set per derived, read off its mask bit by bit.
///
/// Bit `k` of derived `j`'s mask selects cell number `k`, over the
/// `inputs + j` cells numbered below it. A mask selecting nothing falls back to
/// the cell numbered just below: a derived with no dependencies would be a
/// constant, and a constant never re-evaluates, so it would silently drop out
/// of every claim below. A `u8` covers the widest case (3 inputs + 4 earlier
/// deriveds = 7 candidates).
fn dependencies(inputs: usize, masks: &[u8]) -> Vec<Vec<usize>> {
    let mut deriveds = Vec::new();

    for (derived, mask) in masks.iter().enumerate() {
        let available = inputs + derived;
        let mut deps: Vec<usize> = (0..available)
            .filter(|number| mask & (1u8 << number) != 0)
            .collect();

        if deps.is_empty() {
            deps.push(available - 1);
        }

        deriveds.push(deps);
    }

    deriveds
}

/// One commit's writes: raw draws mapped onto the inputs that exist, then
/// deduplicated so the batch touches each input at most once.
///
/// The FIRST draw of an input wins. Which one wins does not matter; that the
/// choice is made BEFORE any permutation does — the reversed run of property
/// (c) reverses this batch, so both runs must be permutations of one another
/// rather than of two different write sets.
fn writes(inputs: usize, batch: &[(u8, i64)]) -> Vec<(usize, i64)> {
    let mut chosen = Vec::new();
    let mut seen = HashSet::new();

    for (draw, value) in batch {
        let input = usize::from(*draw) % inputs;

        if seen.insert(input) {
            chosen.push((input, *value));
        }
    }

    chosen
}

/// The emission order: a permutation of the cell numbers `0..cells`.
///
/// One Fisher-Yates pass, taking each swap partner from `shuffle` — every
/// permutation is reachable, and the mapping from the raw draw is
/// deterministic, like the two above. This is the ONLY place the plan's
/// dependency numbering and the graph section's entry order come apart, and
/// coming apart is the point: entry order is what the loader ranks by (R64), so
/// a shuffled one forces the loader to link references it has not seen declared
/// yet and to sort the static order rather than inherit it.
fn emission(cells: usize, shuffle: &[u8]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..cells).collect();

    for slot in (1..cells).rev() {
        let partner = usize::from(shuffle[slot]) % (slot + 1);

        order.swap(slot, partner);
    }

    order
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

/// The path of the cell numbered `number` — `App.in<i>` for an input,
/// `App.d<j>` for a derived.
fn cell_path(plan: &Plan, number: usize) -> CellPath {
    if number < plan.inputs {
        path(&["App", &format!("in{number}")])
    } else {
        path(&["App", &format!("d{}", number - plan.inputs)])
    }
}

/// The behavior handle derived `j` uses.
fn handle(derived: usize) -> u32 {
    HANDLE_BASE + u32::try_from(derived).expect("a plan holds at most five deriveds")
}

/// A blank cell of the kind, with every clause absent.
fn cell(kind: CellKind, path: CellPath) -> Cell {
    Cell {
        kind,
        path,
        span: Span::new(0, 0),
        tag: TypeTag::Prim(Prim::I64),
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

/// The cell numbered `number`, with its clauses filled in: an input seeded `0`,
/// or a derived bound to its own stub handle and to its dependencies' PATHS.
///
/// Naming dependencies by path is what leaves the emission order free — a path
/// does not care whether the cell it names has been declared yet.
fn cell_of(plan: &Plan, number: usize) -> Cell {
    if number < plan.inputs {
        let mut built = cell(CellKind::Input, cell_path(plan, number));
        built.init = Some(ValueLiteral::int("0".to_string()));

        return built;
    }

    let derived = number - plan.inputs;
    let mut built = cell(CellKind::Derived, cell_path(plan, number));
    built.uses = Some(handle(derived));
    built.inputs = plan.deriveds[derived]
        .iter()
        .map(|&dep| cell_path(plan, dep))
        .collect();

    built
}

/// The plan as a `ductus-ir` module — the PROGRAMMATIC construction route, the
/// same one the dual-construction half of every witness takes (M0.md §6).
///
/// The cells go into one scope in the plan's EMISSION order, not its numbering,
/// so the section's entry order — R64's carrier, and therefore the loader's
/// rank — is a shuffle of the dependency order.
fn module_of(plan: &Plan) -> Module {
    let mut entries = Vec::new();

    for &number in &plan.emission {
        entries.push(Entry::Cell(cell_of(plan, number)));
    }

    Module {
        name: "App".to_string(),
        span: Span::new(0, 0),
        types: TypesSection {
            span: Span::new(0, 0),
            type_defs: Vec::new(),
        },
        graph: GraphSection {
            span: Span::new(0, 0),
            scopes: vec![Scope {
                path: path(&["App"]),
                span: Span::new(0, 0),
                exposes: Vec::new(),
                effects: Vec::new(),
                reset_on_reopen: Vec::new(),
                entries,
            }],
        },
        behaviors: BehaviorsSection {
            span: Span::new(0, 0),
        },
    }
}

/// One stub per derived: the sum of its arguments plus its OWN NUMBER.
///
/// The number addend is what makes the deriveds tell each other apart. Without
/// it, two deriveds over the same dependencies would compute the same number,
/// and a trace that attributed one's value to the other would still read
/// correct. Every stub is pure and total: the addition wraps (M0.md §5 — a
/// fixture must not trap by accident), and a non-`i64` argument is impossible
/// because every generated cell is `i64`.
fn registry(plan: &Plan) -> StubRegistry {
    let mut registry = StubRegistry::new();

    for derived in 0..plan.deriveds.len() {
        let bonus = i64::try_from(plan.inputs + derived).expect("a plan holds at most eight cells");

        registry.register(
            handle(derived),
            Rc::new(move |arguments: &[Value]| {
                let mut total = bonus;

                for argument in arguments {
                    match argument {
                        Value::I64(payload) => total = total.wrapping_add(*payload),
                        other => panic!("a generated graph is all i64; got {other:?}"),
                    }
                }

                Value::I64(total)
            }),
        );
    }

    registry
}

/// What one scripted run produced: the whole trace, and the committed state
/// after each commit.
struct Run {
    /// Every event of the run, rendered — the startup block included.
    trace: String,
    /// One entry per commit of the script, holding every cell's committed value
    /// in cell-number order, read through a snapshot taken right after that
    /// commit.
    states: Vec<Vec<Value>>,
}

/// Loads the plan's module, starts an engine on it, and drives the script.
///
/// `reversed` permutes each commit's write batch — the second run of property
/// (c). Nothing else differs between the two runs, which is what makes a trace
/// difference attributable to write order alone.
fn run(plan: &Plan, reversed: bool) -> Run {
    let module = module_of(plan);
    let provider = registry(plan);
    let graph = load(&module, &provider).expect("a generated module is in the M0 slice");
    let mut sink = RecordingSink::new();

    let states = {
        let mut engine = Engine::start(graph, &mut sink);
        let mut states = Vec::new();

        for batch in &plan.commits {
            let mut batch = batch.clone();

            if reversed {
                batch.reverse();
            }

            for (input, value) in batch {
                engine
                    .write_signal(&cell_path(plan, input), Value::I64(value))
                    .expect("an input cell of the generated module accepts an i64");
            }

            engine.commit();

            let snapshot = engine.acquire_snapshot();

            states.push(
                (0..plan.cells())
                    .map(|number| {
                        snapshot
                            .read(&cell_path(plan, number))
                            .expect("every generated cell is in the index")
                    })
                    .collect(),
            );
        }

        engine.teardown();

        states
    };

    Run {
        trace: sink.into_trace(),
        states,
    }
}

/// The naive oracle: every cell's value, recomputed from scratch in cell-number
/// order.
///
/// `inputs` holds the current value of each input cell. Every derived is then
/// its own number plus the sum of its dependencies — full recomputation, in the
/// one order the plan's numbering admits, with no dirty set and no skipping.
/// This is the whole of it: no engine type, no loader order, no emission order,
/// no trace.
fn oracle(plan: &Plan, inputs: &[i64]) -> Vec<i64> {
    let mut values = inputs.to_vec();

    for (derived, deps) in plan.deriveds.iter().enumerate() {
        let mut total =
            i64::try_from(plan.inputs + derived).expect("a plan holds at most eight cells");

        for &dep in deps {
            total = total.wrapping_add(values[dep]);
        }

        values.push(total);
    }

    values
}

/// The `eval` paths of each commit block, in emission order.
///
/// A block runs from `commit_begin` to `commit_end`; the startup block opens
/// every trace and contributes an empty entry.
fn evaluations(trace: &str) -> Vec<Vec<&str>> {
    let mut blocks: Vec<Vec<&str>> = Vec::new();

    for line in trace.lines() {
        if line == "commit_begin" {
            blocks.push(Vec::new());
        } else if let Some(rest) = line.strip_prefix("eval ") {
            let path = rest
                .split_once(" -> ")
                .expect("an `eval` line is `eval <path> -> <value>`")
                .0;

            blocks
                .last_mut()
                .expect("an `eval` line falls inside a commit block")
                .push(path);
        }
    }

    blocks
}

proptest! {
    /// (a) Within one commit block, no cell evaluates twice.
    ///
    /// §13.10.2 step 3 walks the static order ONCE and evaluates each dirty
    /// cell at most once — that is what makes a commit's cost linear in the
    /// dirty subgraph rather than exponential in its fan-in. A diamond is the
    /// shape that would break it: a join whose two shoulders both dirty would
    /// evaluate twice under naive push propagation, once per incoming edge, and
    /// the second `eval` line for one path inside one block is exactly what
    /// this reads.
    #[test]
    fn no_cell_evaluates_twice_in_one_commit(plan in plan()) {
        let observed = run(&plan, false);

        for (block, evaluated) in evaluations(&observed.trace).iter().enumerate() {
            let mut seen = HashSet::new();

            for path in evaluated {
                prop_assert!(
                    seen.insert(*path),
                    "block {block} evaluates `{path}` twice:\n{}",
                    observed.trace
                );
            }
        }
    }

    /// (b) After every commit, every cell holds what full recomputation says.
    ///
    /// Two failures are caught by the one comparison. A STALE read — a derived
    /// that evaluated before a dependency it reads — leaves a value mixing this
    /// commit's inputs with last commit's intermediate, which no full
    /// recomputation produces (glitch-freedom, §13.10.2 step 3's read law). A
    /// wrongly SKIPPED evaluation leaves last commit's value in a cell whose
    /// dependencies moved. The oracle knows neither dirt nor order, so it
    /// cannot make the same mistake in the same way.
    #[test]
    fn every_commit_settles_to_the_naive_recomputation(plan in plan()) {
        let observed = run(&plan, false);
        let mut inputs = vec![0i64; plan.inputs];

        for (index, batch) in plan.commits.iter().enumerate() {
            for &(input, value) in batch {
                inputs[input] = value;
            }

            let expected = oracle(&plan, &inputs);

            // The oracle covers every cell of the graph, so its length IS the
            // cell count — a shorter answer would mean a derived went
            // unrecomputed, which the assertion below could not see.
            prop_assert_eq!(expected.len(), plan.cells());

            for (number, value) in expected.iter().enumerate() {
                prop_assert!(
                    observed.states[index][number].bits_eq(&Value::I64(*value)),
                    "after commit {index}, cell `{}` holds {} where recomputation says {}:\n{}",
                    cell_path(&plan, number).render(),
                    observed.states[index][number].render(),
                    value,
                    observed.trace
                );
            }
        }
    }

    /// (c) Permuting a commit's writes to distinct cells changes no byte of the
    /// trace.
    ///
    /// The dirty walk is RANK order (R64), so write order is unobservable —
    /// including in the `dirty` lines, which is the only place it could show.
    /// A host that batches its writes differently between two runs of the same
    /// program must get the same run, or "the commit is the unit of change"
    /// would be a claim about the host's calling style rather than about the
    /// graph.
    #[test]
    fn reversing_each_commits_writes_changes_nothing(plan in plan()) {
        let forward = run(&plan, false);
        let reversed = run(&plan, true);

        prop_assert_eq!(forward.trace, reversed.trace);
    }
}

/// Does this plan emit some dependency AFTER the derived that reads it?
///
/// The forward reference is the whole point of shuffling the emission order: on
/// a plan without one, entry order is already a topological order, so the
/// loader's linking and its Kahn sort are never asked a question they could get
/// wrong.
fn emits_a_dependency_late(plan: &Plan) -> bool {
    let mut slot_of = vec![0usize; plan.cells()];

    for (slot, &number) in plan.emission.iter().enumerate() {
        slot_of[number] = slot;
    }

    plan.deriveds.iter().enumerate().any(|(derived, deps)| {
        deps.iter()
            .any(|&dep| slot_of[dep] > slot_of[plan.inputs + derived])
    })
}

/// The guard against a VACUOUS property run: the generator actually draws the
/// shapes the three properties are about.
///
/// Every property above passes trivially on a degenerate generator — a plan
/// whose batches hold one write each makes permutation invariance a tautology,
/// a plan whose deriveds have one dependency each never builds the join that
/// double evaluation would show, a plan that never evaluates anything agrees
/// with any oracle, and a plan whose entries land in dependency order never
/// exercises a forward reference. None of that would show as a failure, which
/// is why it is asserted here instead.
///
/// The floors are far below what the sample produces (a `deterministic()`
/// runner draws the same plans every run) — they are there to catch a
/// generator that COLLAPSES, not to pin the exact draw.
#[test]
fn the_generator_draws_the_shapes_the_properties_are_about() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    let mut runner = TestRunner::deterministic();
    let mut batched_writes = 0;
    let mut joins = 0;
    let mut forward_references = 0;
    let mut evaluated = 0;
    let mut suppressed_blocks = 0;
    let mut widest_block = 0;

    for _ in 0..256 {
        let plan = plan()
            .new_tree(&mut runner)
            .expect("the plan strategy draws")
            .current();

        if plan.commits.iter().any(|batch| batch.len() >= 2) {
            batched_writes += 1;
        }

        if plan.deriveds.iter().any(|deps| deps.len() >= 2) {
            joins += 1;
        }

        if emits_a_dependency_late(&plan) {
            forward_references += 1;
        }

        let observed = run(&plan, false);
        let blocks = evaluations(&observed.trace);

        // Block 0 is the startup block, which is empty by law (§13.14.1 step
        // 4); an empty block PAST it is a commit that evaluated nothing —
        // value-change gating (R24/§13.2.4.4) at work.
        for block in &blocks[1..] {
            if block.is_empty() {
                suppressed_blocks += 1;
            }

            evaluated += block.len();
            widest_block = widest_block.max(block.len());
        }
    }

    assert!(batched_writes >= 20, "batched writes: {batched_writes}");
    assert!(joins >= 20, "multi-dependency deriveds: {joins}");
    assert!(
        forward_references >= 20,
        "plans emitting a dependency late: {forward_references}"
    );
    assert!(evaluated >= 100, "evaluations: {evaluated}");
    assert!(
        suppressed_blocks >= 10,
        "quiet commits: {suppressed_blocks}"
    );
    assert!(widest_block >= 3, "widest commit block: {widest_block}");
}
