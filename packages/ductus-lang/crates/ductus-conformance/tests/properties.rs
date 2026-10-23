//! Semantic property tests (M0.md §6 track 4, extended by M1.md §6.4):
//! randomized graphs — drawn dependency-safe, then EMITTED in shuffled order —
//! and random write/commit scripts, asserting the six laws a witness can only
//! sample.
//!
//! A witness pins ONE trace of ONE module; these assert what must hold of every
//! module in the M1 slice — `input`, `derived`, `recurrent`, `deferred` and
//! `gate`:
//!
//! 1. **No double evaluation** — within one `commit_begin`..`commit_end` block,
//!    no cell evaluates twice (§13.10.2 step 3 walks a topological order once).
//! 2. **Oracle equivalence** — after every commit, every cell holds what an
//!    independent commit-indexed simulation says it holds, `has_staged_work`
//!    answers what that simulation's hold list says, and the block's `gate`
//!    lines are exactly its effective-activation changes (C-39/C-40) in
//!    entry-rank order beside the `dirty` roots (R76).
//! 3. **Write-permutation invariance** — permuting a commit's writes to
//!    DISTINCT cells changes nothing in the trace, byte for byte (R64: the
//!    dirty walk is rank order, never write order).
//! 4. **Lockstep** — no recurrent, and no derived, ever observes a recurrent's
//!    just-advanced value; a deferred's dispatch, which runs after step 4,
//!    observes exactly that value and no other (§13.2.4.1, R74a).
//! 5. **Advance cadence** — a history ring advances exactly once per
//!    evaluation and never otherwise, so `past(k)` at a cell's `m`-th
//!    evaluation is what its `m - k`-th evaluation produced (R72 + R77,
//!    fork 2(ii) + fork 9(a)).
//! 6. **Freeze idempotence** — with every gate closed, no write produces an
//!    `eval` line, while the input cells stay live and produce exactly the
//!    `dirty` lines they would produce ungated (§13.9.7).
//!
//! # Why the emission order is shuffled
//!
//! A generated graph is drawn dependency-safe — every computed cell reads cells
//! numbered strictly below it — because that is what makes acyclicity free. If
//! the entries were then WRITTEN in that numbering, entry order would already
//! be a topological order: no `inputs` clause would ever name a cell declared
//! later, and the loader's static order would equal declaration order for every
//! module the generator can draw. The properties would then hold under a
//! scheduler that ignored dependencies entirely. Shuffling the emission order
//! separates the two: the numbering keeps the graph acyclic, while the entry
//! order the loader ranks by (R64) is a permutation of it, so forward
//! references are routine and the topological sort has to do the work. `gate`
//! entries ride the same shuffle, because a gate takes a graph-section entry
//! rank exactly like a cell (C-45) and step 1's root walk interleaves the two
//! by that rank (R76).
//!
//! # The two shapes the generator once refused, and why it draws them now
//!
//! Wave 1 refused two shapes — a `recurrent` reading a `derived` or a
//! `deferred`, and any cell reading a `recurrent` emitted after it — for one
//! reason, and it was a finding rather than a convenience (FND-38): **whether a
//! reader of a `recurrent` ever saw that recurrent's change depended on
//! declaration order.** The advance's dirty bit was set in the middle of step 3,
//! so a reader ranked BEFORE the recurrent was visited while that bit was still
//! clear, skipped, and never got another chance, because step 6 cleared the bit.
//! The same module with the two entries swapped evaluated the reader every
//! commit: one dependency graph, two declaration orders, two settled values.
//!
//! §4.12's advance-root law removes the fork. A recurrent's evaluation sets NO
//! dirty bit inside the advancing commit, and the advance is a SILENT dirty root
//! at the next one — so under EITHER order the reader sits out commit N and
//! evaluates at N+1 over the value that was published. Both restrictions
//! therefore come off (M1.md §6.4), and lifting them is that law's natural
//! regression guard: the generator now draws readers on both sides of their
//! recurrent and recurrents that read computed cells, the vacuity guard carries
//! a floor for each of the three shapes, and a schedule that went back to
//! depending on rank would show up as two settled values that one
//! commit-indexed oracle cannot both match.
//!
//! Nothing else about the shuffle is constrained.
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
//! [`oracle()`] is straight-line test code: it walks the plan's OWN cell
//! numbering, which is a topological order by construction, and models the six
//! steps as a recurrence over commits — staged writes, per-advance rings, held
//! dispatches landing at `+depth`, and effective activation climbing the gate
//! parent chain. It holds no dirty bits (a cell is dirty exactly when its
//! committed value moved), no static order, no rank, and no runtime type. If it
//! agreed with the engine by sharing the engine's code it would assert nothing.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use ductus_conformance::runner::RecordingSink;
use ductus_conformance::stub_library::StubRegistry;
use ductus_ir::diag::Span;
use ductus_ir::graph::{
    Cell, CellKind, Entry, FallbackEntry, Gate, LookbackEntry, Scope, ValueLiteral,
};
use ductus_ir::module::{BehaviorsSection, GraphSection, Module, TypesSection};
use ductus_ir::path::{CellPath, Segment};
use ductus_ir::types::{Prim, TypeTag};
use ductus_runtime::engine::Engine;
use ductus_runtime::loader::load;
use ductus_runtime::value::Value;
use proptest::prelude::*;

/// The first behavior handle a generated module uses; the computed cell `j`
/// uses `HANDLE_BASE + j`, so every computed cell has its OWN stub and two
/// cells with the same dependencies still differ (see [`registry()`]).
const HANDLE_BASE: u32 = 0xd0;

/// The first PREDICATE handle: gate `g` uses `GATE_HANDLE_BASE + g`. A
/// predicate answers `bool` and a cell stub answers `i64`, so the two families
/// are kept apart by construction rather than by discipline.
const GATE_HANDLE_BASE: u32 = 0xe0;

/// The predicate of the all-closed variant (see [`frozen()`]): the one handle
/// that answers `false` unconditionally.
const FROZEN_HANDLE: u32 = 0xff;

/// The widest plan the generator draws: three inputs, five computed cells and
/// two gates. The emission shuffle draws this many raw bytes, so one
/// fixed-length draw covers every plan size.
const MAX_ENTRIES: usize = 10;

/// A generated graph and the script driven against it.
///
/// The cells are NUMBERED in dependency order: input `i` is number `i`, and
/// computed cell `j` is number `inputs + j`. Every computed cell depends on a
/// nonempty set of STRICTLY LOWER-NUMBERED cells, which is what makes the graph
/// acyclic by construction — no cycle is representable, so the generator never
/// draws a module the loader would reject.
///
/// A cell's number is NOT its rank. `emission` permutes the order the entries
/// are written into the graph section, and it is that entry order R64 ranks by,
/// so a dependency routinely lands after the cell that reads it. Gates share
/// the numbering: entry `cells() + g` is gate `g`.
#[derive(Debug, Clone)]
struct Plan {
    /// How many `i64` input cells, each `init 0`.
    inputs: usize,
    /// One entry per computed cell, in dependency order.
    computed: Vec<Computed>,
    /// One entry per `gate` entry, in gate rank order.
    gates: Vec<GateSpec>,
    /// One entry per commit, holding that commit's writes as
    /// `(input number, value)` pairs over DISTINCT inputs — the distinctness is
    /// what makes permutation a fair question (a second write to one cell
    /// overwrites the first, so reversing THAT batch changes the values, not
    /// just the order).
    commits: Vec<Vec<(usize, i64)>>,
    /// The order the entries are EMITTED into the graph section: a permutation
    /// of the entry numbers, so `emission[slot]` is the number of the entry the
    /// loader gives rank `slot`.
    emission: Vec<usize>,
}

impl Plan {
    /// Total cell count — inputs plus computed cells.
    fn cells(&self) -> usize {
        self.inputs + self.computed.len()
    }

    /// Total entry count — every cell plus every gate.
    fn entries(&self) -> usize {
        self.cells() + self.gates.len()
    }

    /// The computed cell numbered `number`, or `None` when it is an input.
    fn computed(&self, number: usize) -> Option<&Computed> {
        self.computed.get(number.checked_sub(self.inputs)?)
    }

    /// Whether the cell numbered `number` is a `recurrent` — the one kind whose
    /// reads answer with a PREVIOUS-committed value (§13.11.3).
    fn is_recurrent(&self, number: usize) -> bool {
        matches!(
            self.computed(number).map(|computed| &computed.kind),
            Some(Kind::Recurrent { .. })
        )
    }
}

/// One computed cell: what it reads, what kind it is, and which gate guards it.
#[derive(Debug, Clone)]
struct Computed {
    /// Its dependencies as cell numbers, in ascending order — the `inputs`
    /// clause, and the first block of the behavior ABI (R71).
    deps: Vec<usize>,
    /// Which of the three computed kinds it is, with that kind's clauses.
    kind: Kind,
    /// The gate whose `guards` list names it, or `None` for an ungated cell.
    gate: Option<usize>,
}

/// The three computed kinds the M1 slice admits, each carrying its own clauses.
#[derive(Debug, Clone)]
enum Kind {
    /// `derived` — a pure function of its `inputs`, recomputed on demand.
    Derived,
    /// `recurrent` — a derived cell that also reads its own history and, per
    /// `lookback` entry, an input's history.
    Recurrent {
        /// `depth INT` — the self-history depth, 1..=3.
        depth: usize,
        /// Whether the seeds are spelled `init v` (every depth falls back to
        /// one value) or `fallbacks [k: v, …]` (one per depth). They are ONE
        /// clause in two spellings and carrying both is a load error (033-268),
        /// so this is a choice of spelling, not of content.
        uniform: bool,
        /// `reset_on_reopen` — the bare flag.
        resets: bool,
        /// `lookback lookback_map` — per-input history depths.
        lookback: Vec<Lookback>,
    },
    /// `deferred` — published `depth` commits after it is dispatched.
    Deferred {
        /// `depth INT` — the latency in commits, 1..=2.
        depth: usize,
    },
}

/// One `lookback` entry, keyed by POSITION in the reading cell's `deps` rather
/// than by cell number.
///
/// The position is the same number twice over: it is where the input's current
/// value sits in the ABI's first block, and it is what orders the ring
/// arguments in the third (R71, §4.9). Keying by it makes both readable off one
/// field.
#[derive(Debug, Clone, Copy)]
struct Lookback {
    /// Which of the reading cell's `deps` this entry looks back on.
    position: usize,
    /// The deepest `.past(k, …)` of it, 1..=2.
    k: usize,
}

/// One `gate` entry: the input its predicate reads, its enclosing gate, and the
/// threshold that decides it.
#[derive(Debug, Clone, Copy)]
struct GateSpec {
    /// The input cell number the predicate reads. An INPUT, always — a
    /// GENERATOR choice, not a law: a predicate reading a cell its own gate
    /// guards is LEGAL (the owner's 2026-08-24 ruling (a) on R73's rider, which
    /// deleted the `gate_predicate_reads_guarded` check and its class). Drawing
    /// from the inputs keeps the predicate a function of the writes alone, and
    /// no gate ever guards an input.
    input: usize,
    /// `gate_parent PATH` — an EARLIER gate, or `None` at the root of a chain.
    /// Drawing the parent from the gates below this one is what keeps the chain
    /// acyclic without a check.
    parent: Option<usize>,
    /// The predicate is `value >= threshold`, so the flip is a function of one
    /// input's committed value and nothing else.
    threshold: i64,
}

/// The generator: 1..=3 inputs, 1..=5 computed cells, 0..=2 gates, 1..=4
/// commits of 0..=3 writes, and one shuffle of the emission order.
///
/// The random draws are RAW — four bytes per computed cell, a byte and a
/// threshold per gate, an unbounded input choice per write, a fixed-length byte
/// vector for the shuffle — and the mapping onto a legal plan happens here, in
/// one place, deterministically. Generating legality directly (a dependency
/// subset whose size depends on the cell's own index, a write index bounded by
/// a count drawn in the same tuple) would need nested dependent strategies for
/// no extra coverage.
///
/// The dependency draw no longer consults the emission order at all: with the
/// two wave-1 restrictions off (see the module doc), a cell's admissible
/// dependencies are every cell numbered below it, whatever slot either one is
/// emitted into.
fn plan() -> impl Strategy<Value = Plan> {
    (
        1usize..=3,
        prop::collection::vec((any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>()), 1..=5),
        prop::collection::vec((any::<u8>(), -5i64..=5), 0..=2),
        prop::collection::vec(
            prop::collection::vec((any::<u8>(), -10i64..=10), 0..=3),
            1..=4,
        ),
        prop::collection::vec(any::<u8>(), MAX_ENTRIES),
    )
        .prop_map(|(inputs, draws, gate_draws, script, shuffle)| {
            let gates = gate_specs(inputs, &gate_draws);
            let emission = emission(inputs + draws.len() + gates.len(), &shuffle);

            Plan {
                inputs,
                computed: computed(inputs, &draws, gates.len()),
                gates,
                commits: script.iter().map(|batch| writes(inputs, batch)).collect(),
                emission,
            }
        })
}

/// One `GateSpec` per gate draw, read off its byte.
///
/// The low nibble picks the input the predicate reads, the high nibble picks
/// the parent from the gates ALREADY drawn (or none), which is what makes the
/// `gate_parent` chain acyclic by construction rather than by a check.
fn gate_specs(inputs: usize, draws: &[(u8, i64)]) -> Vec<GateSpec> {
    draws
        .iter()
        .enumerate()
        .map(|(rank, &(draw, threshold))| {
            let parent = usize::from(draw >> 4) % (rank + 1);

            GateSpec {
                input: usize::from(draw) % inputs,
                parent: (parent < rank).then_some(parent),
                threshold,
            }
        })
        .collect()
}

/// One `Computed` per draw, in dependency order.
///
/// The cells are built in ascending number order because each one's dependency
/// choice is over the cells below it — that ordering is the whole acyclicity
/// argument, and since the two wave-1 restrictions came off (module doc) it is
/// also the only thing the choice consults.
fn computed(inputs: usize, draws: &[(u8, u8, u8, u8)], gates: usize) -> Vec<Computed> {
    let mut built: Vec<Computed> = Vec::new();

    for (index, &(mask, shape, extra, choice)) in draws.iter().enumerate() {
        let deps = dependencies(inputs + index, mask);

        built.push(Computed {
            kind: kind(&deps, shape, extra),
            deps,
            gate: guard(gates, choice),
        });
    }

    built
}

/// One computed cell's dependency set, read off its mask bit by bit.
///
/// Bit `k` of the mask selects cell number `k`, over every cell numbered below
/// this one — the kind of the candidate does not matter and neither does the
/// slot it is emitted into, which is what lifting the wave-1 restrictions comes
/// to in code. A mask selecting nothing falls back to the highest-numbered
/// candidate: a cell with no dependencies would be a constant, and a constant
/// never re-evaluates, so it would silently drop out of every claim below. Cell
/// 0 is a candidate for every computed cell, so a fallback always exists.
fn dependencies(number: usize, mask: u8) -> Vec<usize> {
    let mut deps: Vec<usize> = (0..number)
        .filter(|&candidate| mask & (1u8 << candidate) != 0)
        .collect();

    if deps.is_empty() {
        deps.extend(number.checked_sub(1));
    }

    deps
}

/// One computed cell's kind and that kind's clauses, read off its shape byte.
///
/// The three kinds are drawn in even thirds. The remaining bits carry the
/// depth, the seed SPELLING (`init` versus `fallbacks`) and the
/// `reset_on_reopen` flag — every clause R71 added to the `cell` production.
fn kind(deps: &[usize], shape: u8, extra: u8) -> Kind {
    match shape % 3 {
        0 => Kind::Derived,
        1 => Kind::Recurrent {
            depth: 1 + (usize::from(shape / 3) % 3),
            uniform: shape & 0b0100_0000 != 0,
            resets: shape & 0b1000_0000 != 0,
            lookback: lookback(deps, extra),
        },
        _ => Kind::Deferred {
            depth: 1 + (usize::from(shape / 3) % 2),
        },
    }
}

/// One recurrent's `lookback` map: the low nibble of `extra` selects which of
/// the first four dependencies are looked back on, the high nibble gives each
/// selected one its depth.
///
/// The keys are DEPENDENCIES by construction, which is the loader's own law —
/// a `lookback` key outside `inputs` is `lookback_not_an_input`, because R71's
/// ABI order is defined over the `inputs` list.
fn lookback(deps: &[usize], extra: u8) -> Vec<Lookback> {
    (0..deps.len().min(4))
        .filter(|position| extra & (1u8 << position) != 0)
        .map(|position| Lookback {
            position,
            k: 1 + usize::from((extra >> (4 + position)) & 1),
        })
        .collect()
}

/// Which gate guards a computed cell, drawn uniformly over the gates and
/// "none".
///
/// At most ONE gate ever names a cell, which is what keeps the two spellings of
/// the binding from disagreeing: a cell claimed by two `guards` lists is
/// `guards_inverse_mismatch` (C-43, R80 item 5 — the class the strict inverse
/// renamed `conflicting_gate_binding` to).
fn guard(gates: usize, choice: u8) -> Option<usize> {
    let picked = usize::from(choice) % (gates + 1);

    (picked < gates).then_some(picked)
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

/// The emission order: a permutation of the entry numbers `0..entries`.
///
/// One Fisher-Yates pass, taking each swap partner from `shuffle` — every
/// permutation is reachable, and the mapping from the raw draw is
/// deterministic, like the ones above. This is the ONLY place the plan's
/// dependency numbering and the graph section's entry order come apart, and
/// coming apart is the point: entry order is what the loader ranks by (R64), so
/// a shuffled one forces the loader to link references it has not seen declared
/// yet and to sort the static order rather than inherit it.
fn emission(entries: usize, shuffle: &[u8]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..entries).collect();

    for slot in (1..entries).rev() {
        let partner = usize::from(shuffle[slot]) % (slot + 1);

        order.swap(slot, partner);
    }

    order
}

/// A plain dotted path — the only path shape the implemented slice admits (D7).
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

/// The path of the cell numbered `number` — `App.in<i>` for an input, and one
/// letter per computed kind: `d` derived, `r` recurrent, `p` deferred
/// (postponed).
///
/// The kind letter is not decoration: a trace line names a path and nothing
/// else, so reading one means knowing which mechanism produced it.
fn cell_path(plan: &Plan, number: usize) -> CellPath {
    let Some(computed) = plan.computed(number) else {
        return path(&["App", &format!("in{number}")]);
    };
    let index = number - plan.inputs;
    let name = match computed.kind {
        Kind::Derived => format!("d{index}"),
        Kind::Recurrent { .. } => format!("r{index}"),
        Kind::Deferred { .. } => format!("p{index}"),
    };

    path(&["App", &name])
}

/// The path of gate `rank`.
fn gate_path(rank: usize) -> CellPath {
    path(&["App", &format!("g{rank}")])
}

/// The behavior handle the computed cell `index` uses.
fn handle(index: usize) -> u32 {
    HANDLE_BASE + u32::try_from(index).expect("a plan holds at most five computed cells")
}

/// The predicate handle gate `rank` uses.
fn gate_handle(rank: usize) -> u32 {
    GATE_HANDLE_BASE + u32::try_from(rank).expect("a plan holds at most two gates")
}

/// A blank cell of the kind, with every clause absent.
fn cell(kind: CellKind, at: CellPath) -> Cell {
    Cell {
        kind,
        path: at,
        span: Span::new(0, 0),
        tag: TypeTag::Prim(Prim::I64),
        uses: None,
        inputs: Vec::new(),
        depth: None,
        lookback: Vec::new(),
        reset_on_reopen: false,
        init: None,
        fallbacks: Vec::new(),
        gate: None,
        combiner: None,
        else_value: None,
        members: Vec::new(),
    }
}

/// The history seed for depth `k` of the cell numbered `number`.
///
/// Under the `fallbacks` spelling the seeds DIFFER per depth, so a ring that
/// answered one depth's seed for another would show as a wrong number rather
/// than as a coincidence; under the `init` spelling they are one value, which
/// is what that spelling means (033-268).
fn fallback(number: usize, k: usize, uniform: bool) -> i64 {
    let base = number as i64 * 10;

    if uniform { base } else { base + k as i64 }
}

/// The cell numbered `number`, with its clauses filled in: an input seeded `0`,
/// or a computed cell bound to its own stub handle, to its dependencies' PATHS,
/// to its gate, and to whatever clauses its kind carries.
///
/// Naming dependencies by path is what leaves the emission order free — a path
/// does not care whether the cell it names has been declared yet.
///
/// The `gate` clause is the BINDING and the gate's `guards` list is its
/// materialized inverse (C-43), so both are written and the loader checks the
/// two sets for equality (`guards_inverse_mismatch`).
fn cell_of(plan: &Plan, number: usize) -> Cell {
    let Some(computed) = plan.computed(number) else {
        let mut built = cell(CellKind::Input, cell_path(plan, number));
        built.init = Some(ValueLiteral::int("0".to_string()));

        return built;
    };
    let index = number - plan.inputs;
    let word = match computed.kind {
        Kind::Derived => CellKind::Derived,
        Kind::Recurrent { .. } => CellKind::Recurrent,
        Kind::Deferred { .. } => CellKind::Deferred,
    };
    let mut built = cell(word, cell_path(plan, number));

    built.uses = Some(handle(index));
    built.gate = computed.gate.map(gate_path);
    built.inputs = computed
        .deps
        .iter()
        .map(|&dep| cell_path(plan, dep))
        .collect();

    match &computed.kind {
        Kind::Derived => {}
        Kind::Deferred { depth } => built.depth = Some(*depth as u64),
        Kind::Recurrent {
            depth,
            uniform,
            resets,
            lookback,
        } => {
            built.depth = Some(*depth as u64);
            built.reset_on_reopen = *resets;
            built.lookback = lookback
                .iter()
                .map(|entry| LookbackEntry {
                    cell: cell_path(plan, computed.deps[entry.position]),
                    k: entry.k as u64,
                })
                .collect();

            if *uniform {
                built.init = Some(ValueLiteral::int(fallback(number, 1, true).to_string()));
            } else {
                built.fallbacks = (1..=*depth)
                    .map(|k| FallbackEntry {
                        k: k as u64,
                        value: ValueLiteral::int(fallback(number, k, false).to_string()),
                    })
                    .collect();
            }
        }
    }

    built
}

/// The `gate` entry of gate `rank`.
///
/// Its `guards` list is the exact inverse of what the constructs below it
/// declared — the `gate PATH` clauses [`cell_of`] wrote AND the `gate_parent`
/// clauses of the gates nested under it. §15.4.1's "materialized inverse" is
/// checked for equality at load (C-43), so both directions are generated from
/// one source of truth.
fn gate_of(plan: &Plan, rank: usize) -> Gate {
    let spec = plan.gates[rank];
    let cells = (0..plan.computed.len())
        .filter(|&index| plan.computed[index].gate == Some(rank))
        .map(|index| cell_path(plan, plan.inputs + index));
    let nested = (0..plan.gates.len())
        .filter(|&inner| plan.gates[inner].parent == Some(rank))
        .map(gate_path);

    Gate {
        path: gate_path(rank),
        span: Span::new(0, 0),
        pred: gate_handle(rank),
        inputs: vec![cell_path(plan, spec.input)],
        guards: cells.chain(nested).collect(),
        gate_parent: spec.parent.map(gate_path),
    }
}

/// One scope's worth of entries, wrapped as a module.
fn scoped(entries: Vec<Entry>) -> Module {
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

/// The plan as a `ductus-ir` module — the PROGRAMMATIC construction route, the
/// same one the dual-construction half of every witness takes (M0.md §6).
///
/// The entries go into one scope in the plan's EMISSION order, not its
/// numbering, so the section's entry order — R64's carrier, and therefore the
/// loader's rank — is a shuffle of the dependency order, with the `gate`
/// entries interleaved among the cells exactly as C-45 has them.
fn module_of(plan: &Plan) -> Module {
    let mut entries = Vec::new();

    for &number in &plan.emission {
        if number < plan.cells() {
            entries.push(Entry::Cell(cell_of(plan, number)));
        } else {
            entries.push(Entry::Gate(gate_of(plan, number - plan.cells())));
        }
    }

    scoped(entries)
}

/// The same cells under ONE gate that never opens — the all-closed variant
/// property (f) drives.
///
/// The plan's own gates are dropped rather than forced shut, because a cell
/// answers to exactly one gate and the `guards` inverse is checked for equality
/// (`guards_inverse_mismatch`, C-43) — so every computed cell's own `gate`
/// clause is REWRITTEN to `App.frozen` here, matching the one `guards` list.
///
/// The predicate reads input 0 — which every plan has, and which this gate does
/// not guard. It could read a guarded cell instead and still load (R73's rider
/// was withdrawn: a predicate reading a cell its own gate guards is LEGAL, and
/// `self_conditional_gate` is that witness); input 0 is chosen because it needs
/// no entry in the `guards` list, and the answer is `false` whatever it reads.
fn frozen_module(plan: &Plan) -> Module {
    let frozen = path(&["App", "frozen"]);
    let mut entries = vec![Entry::Gate(Gate {
        path: frozen.clone(),
        span: Span::new(0, 0),
        pred: FROZEN_HANDLE,
        inputs: vec![cell_path(plan, 0)],
        guards: (0..plan.computed.len())
            .map(|index| cell_path(plan, plan.inputs + index))
            .collect(),
        gate_parent: None,
    })];

    for &number in &plan.emission {
        if number < plan.cells() {
            let mut built = cell_of(plan, number);

            built.gate = plan.computed(number).map(|_| frozen.clone());

            entries.push(Entry::Cell(built));
        }
    }

    scoped(entries)
}

/// One behavior call, as the stub that served it saw it.
///
/// The arguments are what makes the ABI observable at all: R71's order —
/// `inputs`, then `past1 … pastDepth`, then each `lookback` entry's
/// `input_past1 … input_pastK` — is a claim about a `&[Value]` nobody can read
/// from outside the behavior, and properties (d) and (e) are the two claims
/// that need to read it.
#[derive(Debug, Clone)]
struct Call {
    /// The cell number whose stub ran.
    cell: usize,
    /// Every argument, in ABI order.
    arguments: Vec<i64>,
    /// What the stub answered.
    value: i64,
}

/// The call log the stubs append to — shared, because every stub writes into
/// one list and the test reads it after the engine is gone.
type Log = Rc<RefCell<Vec<Call>>>;

/// The value the cell numbered `number` computes from `arguments`.
///
/// The cell number is the addend and the argument POSITION is the weight, and
/// both are load-bearing. Without the addend, two cells over the same
/// dependencies would compute the same number, and a trace that attributed
/// one's value to the other would still read correct. Without the weights the
/// stub would be a plain sum, which is order-insensitive — and R71's whole
/// content is an ORDER, so a gather that shuffled the history arguments would
/// produce the right answer for the wrong reason.
///
/// Every stub is pure and total: the arithmetic wraps (M0.md §5 — a fixture
/// must not trap by accident), and a non-`i64` argument is impossible because
/// every generated cell is `i64`.
fn combine(number: usize, arguments: &[i64]) -> i64 {
    let mut total = number as i64;

    for (position, argument) in arguments.iter().enumerate() {
        let weight = position as i64 + 1;

        total = total.wrapping_add(weight.wrapping_mul(*argument));
    }

    total
}

/// A behavior's arguments as plain numbers.
///
/// # Panics
///
/// On a non-`i64` argument — a harness-author bug, since every generated cell
/// is `i64`.
fn raw_arguments(arguments: &[Value]) -> Vec<i64> {
    arguments
        .iter()
        .map(|argument| match argument {
            Value::I64(payload) => *payload,
            other => panic!("a generated graph is all i64; got {other:?}"),
        })
        .collect()
}

/// One stub per computed cell and one predicate per gate, all appending to
/// `log`.
///
/// # The log is observation, not state
///
/// M0.md §5 makes purity the witness author's contract, and these stubs keep
/// it: the value a stub answers is a function of its arguments alone
/// ([`combine`]), and the log it appends to is never read back by any stub. It
/// is a side channel the TEST owns, which is the only way the ABI order and the
/// ring cadence become assertable — neither is visible in the trace, and R78
/// (fork 8) rules out host history reads.
fn registry(plan: &Plan, log: &Log) -> StubRegistry {
    let mut registry = StubRegistry::new();

    for index in 0..plan.computed.len() {
        let number = plan.inputs + index;
        let recorder = Rc::clone(log);

        registry.register(
            handle(index),
            Rc::new(move |arguments: &[Value]| {
                let raw = raw_arguments(arguments);
                let value = combine(number, &raw);

                recorder.borrow_mut().push(Call {
                    cell: number,
                    arguments: raw,
                    value,
                });

                Value::I64(value)
            }),
        );
    }

    for (rank, spec) in plan.gates.iter().enumerate() {
        let threshold = spec.threshold;

        registry.register(
            gate_handle(rank),
            Rc::new(move |arguments: &[Value]| match arguments {
                [Value::I64(payload)] => Value::Bool(*payload >= threshold),
                other => panic!("a generated predicate reads one i64; got {other:?}"),
            }),
        );
    }

    registry
}

/// What one scripted run produced: the whole trace, the committed state after
/// each commit, the readiness answer after each commit, and every behavior call
/// in order.
struct Run {
    /// Every event of the run, rendered — the startup block included.
    trace: String,
    /// Every cell's value in cell-number order right after startup, which is
    /// what the first commit's step 1a and step 3 read.
    initial: Vec<i64>,
    /// One entry per commit of the script, holding every cell's committed value
    /// in cell-number order, read through a snapshot taken right after that
    /// commit.
    states: Vec<Vec<i64>>,
    /// [`Engine::has_staged_work`] after each commit (R74b).
    staged: Vec<bool>,
    /// Every behavior call of the whole run, construction's included.
    calls: Vec<Call>,
    /// The call-log length at each commit boundary: `bounds[0]` is what
    /// construction logged, and `bounds[i + 1]` what had been logged by the end
    /// of script commit `i` — so commit `i`'s calls are
    /// `calls[bounds[i]..bounds[i + 1]]` and construction's are
    /// `calls[..bounds[0]]`.
    bounds: Vec<usize>,
}

/// Every cell's committed value, in cell-number order.
fn committed(plan: &Plan, engine: &Engine<'_>) -> Vec<i64> {
    let snapshot = engine.acquire_snapshot();

    (0..plan.cells())
        .map(|number| {
            match snapshot
                .read(&cell_path(plan, number))
                .expect("every generated cell is in the index")
            {
                Value::I64(payload) => payload,
                other => panic!("a generated graph is all i64; got {other:?}"),
            }
        })
        .collect()
}

/// Loads the plan's module, starts an engine on it, and drives the script.
///
/// `reversed` permutes each commit's write batch — the second run of property
/// (c). Nothing else differs between the two runs, which is what makes a trace
/// difference attributable to write order alone.
fn run(plan: &Plan, reversed: bool) -> Run {
    let module = module_of(plan);
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let provider = registry(plan, &log);
    let graph = load(&module, &provider).expect("a generated module is in the M1 slice");
    let mut sink = RecordingSink::new();

    let (initial, states, staged, bounds) = {
        let mut engine = Engine::start(graph, &mut sink);
        let initial = committed(plan, &engine);
        let mut states = Vec::new();
        let mut staged = Vec::new();
        let mut bounds = vec![log.borrow().len()];

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

            states.push(committed(plan, &engine));
            staged.push(engine.has_staged_work());
            bounds.push(log.borrow().len());
        }

        engine.teardown();

        (initial, states, staged, bounds)
    };
    let calls = log.borrow().clone();

    Run {
        trace: sink.into_trace(),
        initial,
        states,
        staged,
        calls,
        bounds,
    }
}

/// The same script against the all-closed variant, returning its trace.
///
/// The gate is closed from startup, so §13.14.1's construction still evaluates
/// every cell — gates gate propagation, never existence (016-144) — and every
/// commit after it finds the whole graph ineligible.
fn frozen(plan: &Plan) -> String {
    let module = frozen_module(plan);
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let mut provider = registry(plan, &log);

    provider.register(FROZEN_HANDLE, Rc::new(|_| Value::Bool(false)));

    let graph = load(&module, &provider).expect("a generated module is in the M1 slice");
    let mut sink = RecordingSink::new();

    {
        let mut engine = Engine::start(graph, &mut sink);

        for batch in &plan.commits {
            for &(input, value) in batch {
                engine
                    .write_signal(&cell_path(plan, input), Value::I64(value))
                    .expect("an input cell of the generated module accepts an i64");
            }

            engine.commit();
        }

        engine.teardown();
    }

    sink.into_trace()
}

/// One history ring, as the oracle models it: the per-depth seeds, and the
/// values pushed so far, newest first.
///
/// The seeds are consulted per DEPTH rather than pre-loaded into the ring,
/// which is the read law §13.2.4.3 states: after one advance, `past(1)` is the
/// advanced value while `past(2)` is still its OWN seed.
#[derive(Debug, Clone)]
struct Ring {
    /// One seed per depth; `seeds.len()` IS the ring's depth.
    seeds: Vec<i64>,
    /// The values advanced into it, newest first, never longer than the depth.
    pushed: Vec<i64>,
}

impl Ring {
    /// A ring of the given per-depth seeds, unadvanced.
    fn new(seeds: Vec<i64>) -> Ring {
        Ring {
            seeds,
            pushed: Vec::new(),
        }
    }

    /// What `.past(k, …)` yields: the `k`-th newest advance, or `k`'s seed when
    /// the ring has advanced fewer than `k` times.
    fn past(&self, k: usize) -> i64 {
        match self.pushed.get(k - 1) {
            Some(value) => *value,
            None => self.seeds[k - 1],
        }
    }

    /// One advance — R72's "once per evaluation, never otherwise", counted here
    /// by being called exactly where the engine's step 4 calls it.
    fn advance(&mut self, value: i64) {
        self.pushed.insert(0, value);
        self.pushed.truncate(self.seeds.len());
    }

    /// Empties the ring, so every read answers from the seeds again — C-13's
    /// `reset_on_reopen`.
    fn clear(&mut self) {
        self.pushed.clear();
    }
}

/// One cell's whole history: its own ring, plus one per `lookback` entry in
/// list order.
#[derive(Debug, Clone)]
struct History {
    /// The self-history ring; a depth-zero ring for every non-recurrent cell,
    /// which is never read.
    own: Ring,
    /// One ring per `lookback` entry, in list order (R71's ABI order).
    inputs: Vec<Ring>,
}

/// The history a cell starts with: its declared seeds, and one ring per
/// `lookback` entry.
///
/// The `lookback` rings open on zero and are re-seeded during construction with
/// the input's CONSTRUCTED value, because an input's pre-history has no
/// spelling in the IR (033-267) and that value is the only one of the input's
/// type the runtime holds before the first advance.
fn history_of(plan: &Plan, number: usize) -> History {
    let Some(Kind::Recurrent {
        depth,
        uniform,
        lookback,
        ..
    }) = plan.computed(number).map(|computed| &computed.kind)
    else {
        return History {
            own: Ring::new(Vec::new()),
            inputs: Vec::new(),
        };
    };

    History {
        own: Ring::new(
            (1..=*depth)
                .map(|k| fallback(number, k, *uniform))
                .collect(),
        ),
        inputs: lookback
            .iter()
            .map(|entry| Ring::new(vec![0; entry.k]))
            .collect(),
    }
}

/// One deferred run in flight, as the oracle models it (R74a).
#[derive(Debug, Clone, Copy)]
struct Held {
    /// The deferred cell the run belongs to.
    cell: usize,
    /// The value its behavior produced at dispatch.
    value: i64,
    /// The commit index at which it lands — `dispatch + depth`.
    due: u64,
}

/// What the oracle says of a whole script.
struct Simulation {
    /// Every cell's value after each commit, in cell-number order.
    states: Vec<Vec<i64>>,
    /// Whether a run was still held after each commit (R74b).
    staged: Vec<bool>,
    /// The EFFECTIVE activation changes of each commit, as `(gate, opened)`.
    transitions: Vec<Vec<(usize, bool)>>,
}

/// Whether the cell numbered `number` is effectively active — its own gate AND
/// every ancestor (§13.9.7), already composed into `effective`.
fn active(plan: &Plan, effective: &[bool], number: usize) -> bool {
    match plan.computed(number).and_then(|computed| computed.gate) {
        Some(rank) => effective[rank],
        None => true,
    }
}

/// Whether the gate guarding `gate` effectively OPENED this commit — the
/// gate-open snap's trigger (022-59).
fn opened(changed: &[(usize, bool)], gate: Option<usize>) -> bool {
    match gate {
        Some(rank) => changed.contains(&(rank, true)),
        None => false,
    }
}

/// One gate's effective activation, composed from the own values by climbing
/// the parent chain (§13.9.7).
fn effective_of(plan: &Plan, own: &[bool], rank: usize) -> bool {
    let mut at = Some(rank);

    while let Some(current) = at {
        if !own[current] {
            return false;
        }

        at = plan.gates[current].parent;
    }

    true
}

/// One computed cell's arguments in R71's ABI order: `inputs` in list order,
/// then its own `past1 … pastDepth`, then each `lookback` entry's
/// `input_past1 … input_pastK`.
///
/// Every value read comes from the one in-flight buffer, and a read of a
/// RECURRENT needs no special case even though §13.11.3 gives it one. The
/// reason is the hold: a recurrent's result does not enter the buffer until
/// step 4's second pass, so while step 3 is running the buffer still carries
/// its previous-committed value — the law falls out of the model instead of
/// being asserted by it. Property (d) is where the engine is made to agree.
fn arguments_of(plan: &Plan, index: usize, current: &[i64], history: &History) -> Vec<i64> {
    let computed = &plan.computed[index];
    let mut arguments: Vec<i64> = computed.deps.iter().map(|&dep| current[dep]).collect();

    if let Kind::Recurrent {
        depth, lookback, ..
    } = &computed.kind
    {
        for k in 1..=*depth {
            arguments.push(history.own.past(k));
        }

        for (slot, entry) in lookback.iter().enumerate() {
            for k in 1..=entry.k {
                arguments.push(history.inputs[slot].past(k));
            }
        }
    }

    arguments
}

/// The oracle: an independent commit-indexed simulation of the script.
///
/// It walks the plan's OWN cell numbering, which is a topological order because
/// every cell reads cells numbered below it, and models §13.10.2's steps as a
/// recurrence. Nothing here is the engine's: there is no static order, no rank,
/// no path index, no behavior seam and no runtime type.
///
/// **Dirtiness is a value comparison, not a propagation marker.** The `dirty`
/// vector below holds one question's answer — did this cell's committed value
/// MOVE this commit — and every entry is written as exactly that comparison,
/// including the two root kinds. Nothing marks a cell dirty because something
/// upstream was; transitivity is not modelled at all, because a downstream cell
/// re-evaluates and then compares its own result like everyone else.
///
/// **Construction comes first** (§13.14.1 step 3): every computed cell
/// evaluates once, a deferred's included (C-23), a recurrent reads its seeds and
/// then advances, and the gates take their initial state from the constructed
/// values. The startup commit that follows changes nothing — nothing is staged,
/// nothing has transitioned — so it contributes no state and only moves the
/// commit index to 1, which is the number the first script commit's landings are
/// measured against.
fn oracle(plan: &Plan) -> Simulation {
    let mut values = vec![0i64; plan.cells()];
    let mut history: Vec<History> = (0..plan.cells())
        .map(|number| history_of(plan, number))
        .collect();

    for index in 0..plan.computed.len() {
        let number = plan.inputs + index;

        if let Kind::Recurrent { lookback, .. } = &plan.computed[index].kind {
            for (slot, entry) in lookback.iter().enumerate() {
                let seen = values[plan.computed[index].deps[entry.position]];

                history[number].inputs[slot] = Ring::new(vec![seen; entry.k]);
            }
        }

        let arguments = arguments_of(plan, index, &values, &history[number]);

        values[number] = combine(number, &arguments);

        if let Kind::Recurrent { lookback, .. } = &plan.computed[index].kind {
            history[number].own.advance(values[number]);

            for (slot, entry) in lookback.iter().enumerate() {
                let seen = values[plan.computed[index].deps[entry.position]];

                history[number].inputs[slot].advance(seen);
            }
        }
    }

    let mut effective: Vec<bool> = {
        let own: Vec<bool> = plan
            .gates
            .iter()
            .map(|spec| values[spec.input] >= spec.threshold)
            .collect();

        (0..plan.gates.len())
            .map(|rank| effective_of(plan, &own, rank))
            .collect()
    };
    let mut staged_inputs: Vec<i64> = values[..plan.inputs].to_vec();
    let mut runs: Vec<Held> = Vec::new();
    // The advance roots pending for the NEXT commit (§4.12). Construction's
    // advance is deliberately absent: it publishes as the initial snapshot, so
    // there is nothing for a consumer to catch up to.
    let mut advanced = vec![false; plan.cells()];
    let mut simulation = Simulation {
        states: Vec::new(),
        staged: Vec::new(),
        transitions: Vec::new(),
    };

    for (commit, batch) in plan.commits.iter().enumerate() {
        let commit_index = commit as u64 + 1;
        let previous = values.clone();

        // Step 1a — every predicate, against PRIOR-COMMIT values (R73).
        let own: Vec<bool> = plan
            .gates
            .iter()
            .map(|spec| previous[spec.input] >= spec.threshold)
            .collect();
        let fresh: Vec<bool> = (0..plan.gates.len())
            .map(|rank| effective_of(plan, &own, rank))
            .collect();
        let changed: Vec<(usize, bool)> = (0..plan.gates.len())
            .filter(|&rank| fresh[rank] != effective[rank])
            .map(|rank| (rank, fresh[rank]))
            .collect();

        effective = fresh;

        // Still step 1a, and before any read: a reopened `reset_on_reopen`
        // history clears (C-13), and a frozen deferred's held run is cancelled
        // rather than postponed (§13.9.7).
        for index in 0..plan.computed.len() {
            let resets = matches!(
                plan.computed[index].kind,
                Kind::Recurrent { resets: true, .. }
            );

            if resets && opened(&changed, plan.computed[index].gate) {
                let number = plan.inputs + index;

                history[number].own.clear();

                for ring in &mut history[number].inputs {
                    ring.clear();
                }
            }
        }

        runs.retain(|held| active(plan, &effective, held.cell));

        // Step 1b — the roots: staged writes and landings, each dirty only if
        // its value MOVED.
        for &(input, value) in batch {
            staged_inputs[input] = value;
        }

        let mut current = previous.clone();
        let mut dirty = vec![false; plan.cells()];

        for input in 0..plan.inputs {
            if staged_inputs[input] != previous[input] {
                dirty[input] = true;
                current[input] = staged_inputs[input];
            }
        }

        for index in 0..plan.computed.len() {
            let number = plan.inputs + index;

            if !matches!(plan.computed[index].kind, Kind::Deferred { .. }) {
                continue;
            }

            // A landing happens exactly once, whether or not its value
            // propagates — so the run is consumed before the comparison.
            let Some(at) = runs
                .iter()
                .position(|held| held.cell == number && held.due == commit_index)
            else {
                continue;
            };
            let landed = runs.remove(at).value;

            if landed != previous[number] {
                dirty[number] = true;
                current[number] = landed;
            }
        }

        // Still step 1b — the advance roots (§4.12). A recurrent that advanced
        // to a NEW value at the previous commit is a dirty root here, carrying
        // no value (its advanced value was published then, so `current` already
        // holds it) and no trace line.
        for number in 0..plan.cells() {
            if advanced[number] {
                advanced[number] = false;
                dirty[number] = true;
            }
        }

        // Step 3 — evaluate, activation checked BEFORE staleness (RW-9).
        let mut held: Vec<(usize, i64)> = Vec::new();

        for index in 0..plan.computed.len() {
            let number = plan.inputs + index;
            let computed = &plan.computed[index];

            if matches!(computed.kind, Kind::Deferred { .. }) || !active(plan, &effective, number) {
                continue;
            }

            let snap = matches!(computed.kind, Kind::Derived) && opened(&changed, computed.gate);

            if !snap && !computed.deps.iter().any(|&dep| dirty[dep]) {
                continue;
            }

            let value = combine(
                number,
                &arguments_of(plan, index, &current, &history[number]),
            );

            // A recurrent's result is HELD, so nothing later in this pass can
            // see it — that is the whole of lockstep (§13.2.4.1) — and it sets
            // NO dirty bit here: its advance is a root at the NEXT commit
            // instead (§4.12), which is what makes the schedule independent of
            // declaration order.
            match computed.kind {
                Kind::Recurrent { .. } => held.push((number, value)),
                Kind::Derived | Kind::Deferred { .. } => {
                    dirty[number] = value != previous[number];
                    current[number] = value;
                }
            }
        }

        // Step 4 — advance, then publish, in two passes: every ring records the
        // value its cell SAW, whatever order the drain took (§4.4). One pass
        // would not do — a `lookback` on another recurrent would read a
        // just-advanced value whenever that cell happened to drain first.
        for &(number, value) in &held {
            let computed = &plan.computed[number - plan.inputs];

            history[number].own.advance(value);

            if let Kind::Recurrent { lookback, .. } = &computed.kind {
                for (slot, entry) in lookback.iter().enumerate() {
                    let seen = current[computed.deps[entry.position]];

                    history[number].inputs[slot].advance(seen);
                }
            }
        }

        for &(number, value) in &held {
            advanced[number] = value != previous[number];
            current[number] = value;
        }

        // Step 4½ — dispatch, over the values this commit is ABOUT to publish,
        // the recurrent advances included (R74a).
        for index in 0..plan.computed.len() {
            let number = plan.inputs + index;
            let computed = &plan.computed[index];
            let Kind::Deferred { depth } = computed.kind else {
                continue;
            };

            if !active(plan, &effective, number) {
                continue;
            }

            // A RECURRENT dep answers with its `advanced` bit, not its dirty
            // one: §4.12 sets the dirty bit at the commit AFTER the advance,
            // while the arguments gathered here are THIS commit's, so the two
            // would disagree — an advance would be dispatched a commit late over
            // the next advance's value, and a rooted commit that did not advance
            // would dispatch again over a value that had not moved. 023-73's
            // words are the test: the run snapshots the values the commit is
            // about to publish, recurrent advances included.
            let provenance = opened(&changed, computed.gate)
                || computed.deps.iter().any(|&dep| {
                    if plan.is_recurrent(dep) {
                        advanced[dep]
                    } else {
                        dirty[dep]
                    }
                });

            if !provenance {
                continue;
            }

            let arguments: Vec<i64> = computed.deps.iter().map(|&dep| current[dep]).collect();

            runs.push(Held {
                cell: number,
                value: combine(number, &arguments),
                due: commit_index + depth as u64,
            });
        }

        // Step 5 — publish.
        values = current;

        simulation.states.push(values.clone());
        simulation.staged.push(!runs.is_empty());
        simulation.transitions.push(changed);
    }

    simulation
}

/// One root line of a commit block — step 1's two observable shapes (R76).
#[derive(Debug, Clone, Copy)]
enum Root<'t> {
    /// `dirty <path> <old> -> <new>` — a written input or a landed deferred.
    Dirty(&'t str),
    /// `gate <path> open|close` — an effective activation change.
    Gate(&'t str, bool),
}

/// One `commit_begin`..`commit_end` block, split into what each property reads.
#[derive(Debug, Default)]
struct Block<'t> {
    /// Every `eval` line, as `(path, rendered value)`.
    evaluations: Vec<(&'t str, &'t str)>,
    /// Every root line, in the order the trace carries them.
    roots: Vec<Root<'t>>,
}

/// The commit blocks of a trace, in order.
///
/// A block runs from `commit_begin` to `commit_end`; the startup block opens
/// every trace and is empty by law (§13.14.1 step 4).
///
/// # Panics
///
/// On a line that does not parse as its own event — a rendering change that no
/// property below would otherwise notice.
fn blocks(trace: &str) -> Vec<Block<'_>> {
    let mut parsed: Vec<Block<'_>> = Vec::new();

    for line in trace.lines() {
        if line == "commit_begin" {
            parsed.push(Block::default());

            continue;
        }

        let Some(block) = parsed.last_mut() else {
            continue;
        };

        if let Some(rest) = line.strip_prefix("eval ") {
            let split = rest
                .split_once(" -> ")
                .expect("an `eval` line is `eval <path> -> <value>`");

            block.evaluations.push(split);
        } else if let Some(rest) = line.strip_prefix("dirty ") {
            let split = rest
                .split_once(' ')
                .expect("a `dirty` line is `dirty <path> <old> -> <new>`");

            block.roots.push(Root::Dirty(split.0));
        } else if let Some(rest) = line.strip_prefix("gate ") {
            let (at, direction) = rest
                .rsplit_once(' ')
                .expect("a `gate` line is `gate <path> open|close`");
            let open = match direction {
                "open" => true,
                "close" => false,
                other => panic!("a `gate` line reads `open` or `close`, not `{other}`"),
            };

            block.roots.push(Root::Gate(at, open));
        }
    }

    parsed
}

/// Every entry's rendered path against its entry NUMBER — the scale step 1's
/// interleaved root walk is ordered by (C-45, R76).
///
/// The number is the plan's, and the emission order is what turns it into the
/// loader's rank, so the check below compares the trace's order against the
/// emission's rather than against the numbering's.
fn entry_numbers(plan: &Plan) -> Vec<(String, usize)> {
    let mut named: Vec<(String, usize)> = (0..plan.cells())
        .map(|number| (cell_path(plan, number).render(), number))
        .collect();

    for rank in 0..plan.gates.len() {
        named.push((gate_path(rank).render(), plan.cells() + rank));
    }

    named
}

/// Where `at` sits in the EMISSION order — its rank, which is what R76 orders
/// the root lines by.
fn slot_of(plan: &Plan, named: &[(String, usize)], at: &str) -> usize {
    let entry = named
        .iter()
        .find(|(rendered, _)| rendered == at)
        .map(|(_, number)| *number)
        .expect("every traced path names an entry of the plan");

    plan.emission
        .iter()
        .position(|&number| number == entry)
        .expect("every entry is emitted exactly once")
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
    ///
    /// It extends to M1 unchanged, and the two new ways to be evaluated are why
    /// that is worth re-asserting rather than assuming: a gate-open snap forces
    /// a guarded derived stale (022-59) on a commit its dependencies may ALSO
    /// have dirtied, and a recurrent takes a place in the same static order
    /// while reading values no edge in it describes.
    #[test]
    fn no_cell_evaluates_twice_in_one_commit(plan in plan()) {
        let observed = run(&plan, false);

        for (index, block) in blocks(&observed.trace).iter().enumerate() {
            let mut seen = HashSet::new();

            for (at, _) in &block.evaluations {
                prop_assert!(
                    seen.insert(*at),
                    "block {index} evaluates `{at}` twice:\n{}",
                    observed.trace
                );
            }
        }
    }

    /// (b) After every commit, every cell holds what the independent simulation
    /// says, `has_staged_work` answers what its hold list says, and the block's
    /// root lines are its transitions and its dirt in rank order.
    ///
    /// Three failures are caught by the one comparison. A STALE read — a cell
    /// that evaluated before a dependency it reads — leaves a value mixing this
    /// commit's inputs with last commit's intermediate, which the simulation
    /// never produces (glitch-freedom, §13.10.2 step 3's read law). A wrongly
    /// SKIPPED evaluation leaves last commit's value in a cell whose
    /// dependencies moved. And a ring advanced at the wrong cadence, a landing
    /// at the wrong commit, or a freeze applied to the wrong cell all show as a
    /// number that no correct sequence produces.
    ///
    /// The gate lines are asserted as a SET against the simulation's
    /// transitions, because the trace orders them by entry rank while the
    /// simulation knows only gate order; their ORDER is then asserted
    /// separately, against the emission — a root line is a root line whatever
    /// produced it, and R76 says the whole sequence ascends by rank.
    #[test]
    fn every_commit_settles_to_the_naive_recomputation(plan in plan()) {
        let observed = run(&plan, false);
        let expected = oracle(&plan);
        let parsed = blocks(&observed.trace);
        let named = entry_numbers(&plan);

        prop_assert_eq!(expected.states.len(), plan.commits.len());
        prop_assert_eq!(parsed.len(), plan.commits.len() + 1);

        for (index, state) in expected.states.iter().enumerate() {
            // The simulation covers every cell of the graph, so its length IS
            // the cell count — a shorter answer would mean a cell went
            // unmodelled, which the assertion below could not see.
            prop_assert_eq!(state.len(), plan.cells());

            for (number, value) in state.iter().enumerate() {
                prop_assert_eq!(
                    observed.states[index][number],
                    *value,
                    "after commit {}, cell `{}` holds {} where the simulation says {}:\n{}",
                    index,
                    cell_path(&plan, number).render(),
                    observed.states[index][number],
                    value,
                    observed.trace
                );
            }

            prop_assert_eq!(
                observed.staged[index],
                expected.staged[index],
                "after commit {}, `has_staged_work` says {} where the simulation holds {} runs:\n{}",
                index,
                observed.staged[index],
                usize::from(expected.staged[index]),
                observed.trace
            );

            let mut spelled: Vec<String> = parsed[index + 1]
                .roots
                .iter()
                .filter_map(|root| match root {
                    Root::Gate(at, open) => Some(format!("{at} {open}")),
                    Root::Dirty(_) => None,
                })
                .collect();
            let mut rendered: Vec<String> = expected.transitions[index]
                .iter()
                .map(|&(rank, open)| format!("{} {open}", gate_path(rank).render()))
                .collect();

            spelled.sort();
            rendered.sort();

            prop_assert_eq!(
                &spelled,
                &rendered,
                "commit {} traces gate lines {:?} where the simulation transitions {:?}:\n{}",
                index,
                spelled,
                rendered,
                observed.trace
            );

            let mut previous: Option<usize> = None;

            for root in &parsed[index + 1].roots {
                let at = match root {
                    Root::Dirty(at) | Root::Gate(at, _) => at,
                };
                let slot = slot_of(&plan, &named, at);

                if let Some(earlier) = previous {
                    prop_assert!(
                        earlier < slot,
                        "commit {index} traces `{at}` (rank {slot}) after rank {earlier}:\n{}",
                        observed.trace
                    );
                }

                previous = Some(slot);
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
    ///
    /// It is also the cheapest guard that step 1's generalization did not break
    /// that determinism: the walk now interleaves three root sources (R76), and
    /// two of them — a landing, a transition — arrive from engine state rather
    /// than from the host's calls.
    #[test]
    fn reversing_each_commits_writes_changes_nothing(plan in plan()) {
        let forward = run(&plan, false);
        let reversed = run(&plan, true);

        prop_assert_eq!(forward.trace, reversed.trace);
    }

    /// (d) LOCKSTEP — no step-3 read ever sees a recurrent's just-advanced
    /// value, and the step-4½ dispatch sees nothing else.
    ///
    /// §13.2.4.1's lockstep law says the drain order of step 4 is unobservable,
    /// and the engine makes it so by HOLDING every recurrent's result until
    /// every recurrent has evaluated. This asserts the law where it is decided
    /// rather than where it is visible: the arguments a stub was actually
    /// handed. A recurrent reading another recurrent, and a derived reading
    /// one, must both see the value that stood at commit START (§13.11.3) — so
    /// permuting which one drains first cannot change a single argument, and
    /// therefore cannot change a value or a line.
    ///
    /// The deferred is the counterpart and the reason this is not a tautology:
    /// its dispatch runs AFTER step 4 (R74a), so it reads the very value the
    /// step-3 readers were forbidden to see. One commit, one cell, two reads,
    /// two different answers — both required.
    #[test]
    fn no_read_observes_a_recurrents_advance_before_step_four(plan in plan()) {
        let observed = run(&plan, false);

        for commit in 0..plan.commits.len() {
            let before = if commit == 0 {
                &observed.initial
            } else {
                &observed.states[commit - 1]
            };
            let after = &observed.states[commit];

            for call in &observed.calls[observed.bounds[commit]..observed.bounds[commit + 1]] {
                let computed = &plan.computed[call.cell - plan.inputs];
                let dispatched = matches!(computed.kind, Kind::Deferred { .. });

                for (position, &dep) in computed.deps.iter().enumerate() {
                    if !plan.is_recurrent(dep) {
                        continue;
                    }

                    let expected = if dispatched { after[dep] } else { before[dep] };

                    prop_assert_eq!(
                        call.arguments[position],
                        expected,
                        "at commit {}, `{}` read `{}` as {} where the law says {}:\n{}",
                        commit,
                        cell_path(&plan, call.cell).render(),
                        cell_path(&plan, dep).render(),
                        call.arguments[position],
                        expected,
                        observed.trace
                    );
                }
            }
        }
    }

    /// (e) ADVANCE CADENCE — a ring advances exactly once per evaluation and
    /// never otherwise.
    ///
    /// R72 makes `past(k)` count ADVANCES rather than commits, and R77 makes an
    /// evaluation advance the ring even when its result is bit-identical. Put
    /// together they pin one arithmetic: at a cell's `m`-th evaluation,
    /// `past(k)` is what its `m - k`-th evaluation produced, and its seed until
    /// `m` reaches `k`. Both forks are encoded by that one equation — a commit
    /// the cell sat out consumes no slot (fork 2(ii)), and an equal-valued
    /// evaluation consumes one (fork 9(a)) — and neither is visible in the
    /// trace, since a suppressed evaluation traces nothing and a ring is not
    /// readable by path (R78, fork 8).
    ///
    /// The `lookback` rings answer the same equation against the READER's
    /// advances (C-10): the input's value at the reader's `m - k`-th
    /// evaluation is the value the reader itself was handed then, and its seed
    /// is the value it was handed at construction.
    ///
    /// A `reset_on_reopen` recurrent under a gate is left out: its ring clears
    /// on a reopen (C-13), which is a discontinuity in exactly this sequence
    /// and is asserted by (b) instead. Ungated ones stay in — nothing can snap
    /// them.
    #[test]
    fn a_ring_advances_exactly_once_per_evaluation(plan in plan()) {
        let observed = run(&plan, false);

        for (index, computed) in plan.computed.iter().enumerate() {
            let number = plan.inputs + index;
            let Kind::Recurrent { depth, uniform, resets, lookback } = &computed.kind else {
                continue;
            };

            if *resets && computed.gate.is_some() {
                continue;
            }

            let calls: Vec<&Call> = observed
                .calls
                .iter()
                .filter(|call| call.cell == number)
                .collect();

            for (turn, call) in calls.iter().enumerate() {
                for k in 1..=*depth {
                    let expected = match turn.checked_sub(k) {
                        Some(earlier) => calls[earlier].value,
                        None => fallback(number, k, *uniform),
                    };

                    prop_assert_eq!(
                        call.arguments[computed.deps.len() + k - 1],
                        expected,
                        "`{}` read past({}) as {} at evaluation {}, where the ring says {}:\n{}",
                        cell_path(&plan, number).render(),
                        k,
                        call.arguments[computed.deps.len() + k - 1],
                        turn,
                        expected,
                        observed.trace
                    );
                }

                let mut offset = computed.deps.len() + *depth;

                for entry in lookback {
                    for k in 1..=entry.k {
                        let expected = match turn.checked_sub(k) {
                            Some(earlier) => calls[earlier].arguments[entry.position],
                            None => calls[0].arguments[entry.position],
                        };

                        prop_assert_eq!(
                            call.arguments[offset + k - 1],
                            expected,
                            "`{}` read `{}`.past({}) as {} at evaluation {}, where the ring says {}:\n{}",
                            cell_path(&plan, number).render(),
                            cell_path(&plan, computed.deps[entry.position]).render(),
                            k,
                            call.arguments[offset + k - 1],
                            turn,
                            expected,
                            observed.trace
                        );
                    }

                    offset += entry.k;
                }
            }
        }
    }

    /// (f) FREEZE IDEMPOTENCE — with every gate closed, no write evaluates
    /// anything, and the inputs still dirty exactly as they would ungated.
    ///
    /// §13.9.7's freeze law is TWO claims and this asserts both at once. The
    /// guarded cells are ineligible — activation is checked before staleness
    /// (RW-9) — so nothing recomputes, no ring advances, no deferred dispatches
    /// and therefore none ever lands. And the input cells stay LIVE: a write
    /// under a closed gate stages, publishes and traces its `dirty` line, which
    /// is what makes "a gate freezes propagation, never existence" a statement
    /// about the graph rather than about which cells happen to be named.
    ///
    /// The gate is closed from startup, so it never TRANSITIONS either: a trace
    /// records changes, and a gate that has always been shut has none to record.
    #[test]
    fn a_graph_under_closed_gates_evaluates_nothing(plan in plan()) {
        let trace = frozen(&plan);
        let parsed = blocks(&trace);
        let mut committed = vec![0i64; plan.inputs];
        let mut expected = 0;

        for batch in &plan.commits {
            for &(input, value) in batch {
                if value != committed[input] {
                    expected += 1;
                    committed[input] = value;
                }
            }
        }

        let mut dirtied = 0;

        for (index, block) in parsed.iter().enumerate() {
            prop_assert!(
                block.evaluations.is_empty(),
                "block {index} evaluated under closed gates:\n{trace}"
            );

            for root in &block.roots {
                match root {
                    Root::Dirty(_) => dirtied += 1,
                    Root::Gate(at, open) => prop_assert!(
                        false,
                        "block {index} traced `gate {at} {open}` for a gate shut since startup:\n{trace}"
                    ),
                }
            }
        }

        prop_assert_eq!(
            dirtied,
            expected,
            "a frozen graph traced {} dirty lines where the writes move {} values:\n{}",
            dirtied,
            expected,
            trace
        );
    }
}

/// Does this plan emit some dependency AFTER the cell that reads it?
///
/// The forward reference is the whole point of shuffling the emission order: on
/// a plan without one, entry order is already a topological order, so the
/// loader's linking and its Kahn sort are never asked a question they could get
/// wrong.
fn emits_a_dependency_late(plan: &Plan) -> bool {
    let mut slot_of = vec![0usize; plan.entries()];

    for (slot, &entry) in plan.emission.iter().enumerate() {
        slot_of[entry] = slot;
    }

    plan.computed.iter().enumerate().any(|(index, computed)| {
        computed
            .deps
            .iter()
            .any(|&dep| slot_of[dep] > slot_of[plan.inputs + index])
    })
}

/// Does this plan hold a cell that reads a `recurrent` emitted on the given
/// side of it — `Late` for the arrangement wave 1 refused, `Early` for the one
/// it allowed?
///
/// Both sides matter now and neither did before. §4.12 says the reader sits out
/// the advancing commit and evaluates at the next one WHATEVER the entry order,
/// so the claim is only tested when the sample carries both arrangements; a
/// generator that quietly drew one of them would assert the law on half its
/// domain and pass.
fn reads_a_recurrent_emitted(plan: &Plan, side: Side) -> bool {
    let mut slot_of = vec![0usize; plan.entries()];

    for (slot, &entry) in plan.emission.iter().enumerate() {
        slot_of[entry] = slot;
    }

    plan.computed.iter().enumerate().any(|(index, computed)| {
        let reader = slot_of[plan.inputs + index];

        computed.deps.iter().any(|&dep| {
            plan.is_recurrent(dep)
                && match side {
                    Side::Early => slot_of[dep] < reader,
                    Side::Late => slot_of[dep] > reader,
                }
        })
    })
}

/// Which side of its reader a `recurrent` is emitted on.
#[derive(Debug, Clone, Copy)]
enum Side {
    /// Before the reader — the arrangement wave 1's generator guaranteed.
    Early,
    /// After the reader — the arrangement it refused to draw, legal and
    /// indistinguishable under §4.12.
    Late,
}

/// Does this plan hold a `recurrent` whose own `inputs` name a `derived` or a
/// `deferred`?
///
/// Wave 1's other restriction: a recurrent read only inputs and other
/// recurrents, so it had in-degree zero and every Kahn pass reached it first.
/// The shape it excluded is `self_cycle`'s — a recurrent fed by a computed cell
/// — and nothing about §4.12 leaves it observable, so it is drawn and counted.
fn feeds_a_recurrent_from_a_computed_cell(plan: &Plan) -> bool {
    plan.computed.iter().any(|computed| {
        matches!(computed.kind, Kind::Recurrent { .. })
            && computed
                .deps
                .iter()
                .any(|&dep| dep >= plan.inputs && !plan.is_recurrent(dep))
    })
}

/// The guard against a VACUOUS property run: the generator actually draws the
/// shapes the six properties are about.
///
/// Every property above passes trivially on a degenerate generator — a plan
/// whose batches hold one write each makes permutation invariance a tautology,
/// a plan whose cells have one dependency each never builds the join that
/// double evaluation would show, a plan that never evaluates anything agrees
/// with any oracle, a plan whose gates never flip asserts nothing about freeze
/// or snap, a plan whose deferreds never land asserts nothing about latency,
/// and a plan whose recurrents evaluate every commit would pass an advance
/// cadence that counted COMMITS. None of that would show as a failure, which is
/// why it is asserted here instead.
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
    let mut readers_after_a_recurrent = 0;
    let mut readers_before_a_recurrent = 0;
    let mut recurrents_fed_by_a_computed_cell = 0;
    let mut evaluated = 0;
    let mut suppressed_blocks = 0;
    let mut widest_block = 0;
    let mut recurrents = 0;
    let mut deep_rings = 0;
    let mut per_depth_seeds = 0;
    let mut lookbacks = 0;
    let mut deferreds = 0;
    let mut landings = 0;
    let mut nested_gates = 0;
    let mut opens = 0;
    let mut closes = 0;
    let mut skipped_advances = 0;

    for _ in 0..256 {
        let plan = plan()
            .new_tree(&mut runner)
            .expect("the plan strategy draws")
            .current();

        if plan.commits.iter().any(|batch| batch.len() >= 2) {
            batched_writes += 1;
        }

        if plan
            .computed
            .iter()
            .any(|computed| computed.deps.len() >= 2)
        {
            joins += 1;
        }

        if emits_a_dependency_late(&plan) {
            forward_references += 1;
        }

        if reads_a_recurrent_emitted(&plan, Side::Early) {
            readers_after_a_recurrent += 1;
        }

        if reads_a_recurrent_emitted(&plan, Side::Late) {
            readers_before_a_recurrent += 1;
        }

        if feeds_a_recurrent_from_a_computed_cell(&plan) {
            recurrents_fed_by_a_computed_cell += 1;
        }

        if plan.gates.iter().any(|spec| spec.parent.is_some()) {
            nested_gates += 1;
        }

        for computed in &plan.computed {
            match &computed.kind {
                Kind::Derived => {}
                Kind::Deferred { .. } => deferreds += 1,
                Kind::Recurrent {
                    depth,
                    uniform,
                    lookback,
                    ..
                } => {
                    recurrents += 1;
                    lookbacks += lookback.len();

                    if *depth >= 2 {
                        deep_rings += 1;
                    }

                    if !*uniform {
                        per_depth_seeds += 1;
                    }
                }
            }
        }

        let observed = run(&plan, false);
        let parsed = blocks(&observed.trace);
        let deferred_paths: Vec<String> = (0..plan.computed.len())
            .filter(|&index| matches!(plan.computed[index].kind, Kind::Deferred { .. }))
            .map(|index| cell_path(&plan, plan.inputs + index).render())
            .collect();
        let recurrent_paths: Vec<String> = (0..plan.computed.len())
            .filter(|&index| matches!(plan.computed[index].kind, Kind::Recurrent { .. }))
            .map(|index| cell_path(&plan, plan.inputs + index).render())
            .collect();

        // Block 0 is the startup block, which is empty by law (§13.14.1 step
        // 4); an empty block PAST it is a commit that evaluated nothing —
        // value-change gating (R24/§13.2.4.4) at work.
        for block in &parsed[1..] {
            if block.evaluations.is_empty() {
                suppressed_blocks += 1;
            }

            evaluated += block.evaluations.len();
            widest_block = widest_block.max(block.evaluations.len());

            for root in &block.roots {
                match root {
                    Root::Dirty(at) => {
                        if deferred_paths.iter().any(|path| path == at) {
                            landings += 1;
                        }
                    }
                    Root::Gate(_, true) => opens += 1,
                    Root::Gate(_, false) => closes += 1,
                }
            }

            // A commit that evaluated SOMETHING while a recurrent sat it out is
            // the shape fork 2(ii) is about: the ring that did not shift.
            if !block.evaluations.is_empty() {
                skipped_advances += recurrent_paths
                    .iter()
                    .filter(|path| {
                        !block
                            .evaluations
                            .iter()
                            .any(|(evaluated, _)| evaluated == path)
                    })
                    .count();
            }
        }
    }

    assert!(batched_writes >= 20, "batched writes: {batched_writes}");
    assert!(joins >= 20, "multi-dependency cells: {joins}");
    assert!(
        forward_references >= 20,
        "plans emitting a dependency late: {forward_references}"
    );
    assert!(
        readers_after_a_recurrent >= 20,
        "plans reading a recurrent emitted before the reader: {readers_after_a_recurrent}"
    );
    assert!(
        readers_before_a_recurrent >= 20,
        "plans reading a recurrent emitted after the reader: {readers_before_a_recurrent}"
    );
    assert!(
        recurrents_fed_by_a_computed_cell >= 20,
        "plans feeding a recurrent from a computed cell: {recurrents_fed_by_a_computed_cell}"
    );
    assert!(evaluated >= 100, "evaluations: {evaluated}");
    assert!(
        suppressed_blocks >= 10,
        "quiet commits: {suppressed_blocks}"
    );
    assert!(widest_block >= 3, "widest commit block: {widest_block}");
    assert!(recurrents >= 50, "recurrent cells: {recurrents}");
    assert!(deep_rings >= 20, "rings deeper than one: {deep_rings}");
    assert!(
        per_depth_seeds >= 20,
        "recurrents spelling per-depth seeds: {per_depth_seeds}"
    );
    assert!(lookbacks >= 20, "lookback entries: {lookbacks}");
    assert!(deferreds >= 50, "deferred cells: {deferreds}");
    assert!(landings >= 20, "deferred landings: {landings}");
    assert!(nested_gates >= 10, "plans nesting a gate: {nested_gates}");
    assert!(opens >= 10, "effective gate opens: {opens}");
    assert!(closes >= 10, "effective gate closes: {closes}");
    assert!(
        skipped_advances >= 20,
        "commits a recurrent sat out: {skipped_advances}"
    );
}
