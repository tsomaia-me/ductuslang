//! Folder-private test scaffolding shared by the engine's test modules (S1's
//! support-file exception): the graph builders, the stub provider, and the
//! recording sink every engine test drives its witness through.
//!
//! Nothing here is a fixture in disguise — graphs are built through the REAL
//! pipeline (`ductus-ir`'s Rust API → [`load`] → [`Engine::start`]), so a test
//! exercises the loader's rank assignment and static order rather than a
//! hand-written stand-in of them.

use std::collections::HashMap;
use std::rc::Rc;

use ductus_ir::diag::Span;
use ductus_ir::graph::{Cell, CellKind, Entry, Gate, Scope, ValueLiteral};
use ductus_ir::module::{BehaviorsSection, GraphSection, Module, TypesSection};
use ductus_ir::path::{CellPath, Segment};
use ductus_ir::types::{Prim, TypeTag};

use crate::loader::{LoadedGraph, load};
use crate::provider::{Behavior, BehaviorProvider};
use crate::trace_sink::{TraceEvent, TraceSink};
use crate::value::Value;

/// The block every trace opens with — §13.14.1 step 4's first commit, which
/// finds nothing dirty (construction already ran) and publishes the initial
/// snapshot.
pub(super) const STARTUP: &str = "commit_begin\npublish\ncommit_end\n";

/// A plain dotted path — the only path shape the M0 slice admits (D7).
pub(super) fn path(names: &[&str]) -> CellPath {
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

/// A blank cell of the kind: every clause absent, filled by the two builders
/// below.
fn cell(kind: CellKind, names: &[&str], prim: Prim) -> Cell {
    Cell {
        kind,
        path: path(names),
        span: Span::new(0, 0),
        tag: TypeTag::Prim(prim),
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

/// An `input` cell with its `init` literal.
pub(super) fn input(names: &[&str], prim: Prim, init: ValueLiteral) -> Cell {
    let mut built = cell(CellKind::Input, names, prim);
    built.init = Some(init);

    built
}

/// A `derived` cell with its behavior handle and its input paths.
pub(super) fn derived(names: &[&str], prim: Prim, uses: u32, inputs: &[&[&str]]) -> Cell {
    let mut built = cell(CellKind::Derived, names, prim);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();

    built
}

/// A `recurrent` cell with its behavior handle, input paths, self-history
/// depth and the UNIFORM `init` fallback — every depth falls back to one value
/// (R71's shorthand), which is what most witnesses want.
///
/// A witness needing per-depth seeds or `lookback` rings fills the fields
/// itself: they are the same clauses, spelled one by one.
pub(super) fn recurrent(
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

/// A `deferred` cell with its behavior handle, input paths and latency in
/// commits — the three clauses 033-245 renders, and a deferred admits no
/// others.
pub(super) fn deferred(
    names: &[&str],
    prim: Prim,
    uses: u32,
    inputs: &[&[&str]],
    depth: u64,
) -> Cell {
    let mut built = cell(CellKind::Deferred, names, prim);
    built.uses = Some(uses);
    built.inputs = inputs.iter().map(|names| path(names)).collect();
    built.depth = Some(depth);

    built
}

/// A `gate` entry with its predicate handle, the cells the predicate reads and
/// the constructs it guards.
///
/// `gate_parent` is left absent — [`nested`] adds it. Both ends of every binding
/// have to be spelled: `guards` is the materialized inverse of the guarded
/// construct's own clause and the loader checks the two sets for EQUALITY
/// (`guards_inverse_mismatch`, C-43), so a witness naming one end and not the
/// other no longer loads.
pub(super) fn gate(names: &[&str], pred: u32, inputs: &[&[&str]], guards: &[&[&str]]) -> Gate {
    Gate {
        path: path(names),
        span: Span::new(0, 0),
        pred,
        inputs: inputs.iter().map(|names| path(names)).collect(),
        guards: guards.iter().map(|names| path(names)).collect(),
        gate_parent: None,
    }
}

/// The same gate, carrying a `gate_parent PATH` — the inside half of nesting,
/// whose outside half is the enclosing gate's `guards` entry.
pub(super) fn nested(mut gate: Gate, parent: &[&str]) -> Gate {
    gate.gate_parent = Some(path(parent));

    gate
}

/// The same cell, carrying a `gate PATH` back-reference —
/// `gated(derived(…), &["App", "g"])`.
pub(super) fn gated(mut cell: Cell, gate: &[&str]) -> Cell {
    cell.gate = Some(path(gate));

    cell
}

/// A scope holding cells in body order — the rank carrier (R64).
pub(super) fn scope(names: &[&str], cells: Vec<Cell>) -> Scope {
    entries(names, cells.into_iter().map(Entry::Cell).collect())
}

/// A scope holding entries of ANY kind in body order.
///
/// Gates and cells share ONE entry-rank sequence (C-45), so a witness about
/// rank interleaving has to be able to put a `gate` between two cells — which
/// is what this builder is for and what [`scope`] cannot express.
pub(super) fn entries(names: &[&str], entries: Vec<Entry>) -> Scope {
    Scope {
        path: path(names),
        span: Span::new(0, 0),
        exposes: Vec::new(),
        effects: Vec::new(),
        reset_on_reopen: Vec::new(),
        entries,
    }
}

/// A module whose graph section holds `scopes` in section order, with the
/// empty types and behaviors sections every M0 module carries.
pub(super) fn module(scopes: Vec<Scope>) -> Module {
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

/// The resolvability seam's test double (D5): a plain map, resolving what was
/// registered and refusing what was not.
pub(super) struct StubProvider {
    behaviors: HashMap<u32, Behavior>,
}

impl StubProvider {
    pub(super) fn new() -> StubProvider {
        StubProvider {
            behaviors: HashMap::new(),
        }
    }

    /// Registers one handle, chaining — `StubProvider::new().with(1, …)`.
    #[must_use]
    pub(super) fn with(mut self, handle: u32, behavior: Behavior) -> StubProvider {
        self.behaviors.insert(handle, behavior);

        self
    }
}

impl BehaviorProvider for StubProvider {
    fn resolve(&self, handle: u32) -> Option<Behavior> {
        self.behaviors.get(&handle).cloned()
    }
}

/// A provider resolving each handle to the same inert `i64` stub — for tests
/// that never evaluate anything (validation, load shape).
pub(super) fn provider(handles: &[u32]) -> StubProvider {
    let mut built = StubProvider::new();

    for &handle in handles {
        built = built.with(handle, Rc::new(|_| Value::I64(0)));
    }

    built
}

/// Loads a module through the real loader.
pub(super) fn loaded(module: &Module, provider: &StubProvider) -> LoadedGraph {
    load(module, provider).expect("the module loads")
}

/// The recording sink: renders every event it hears into one accumulating
/// buffer, so a whole session's trace compares as TEXT (R24's rendering is the
/// definition of "same trace").
pub(super) struct RecordingSink {
    trace: String,
}

impl RecordingSink {
    pub(super) fn new() -> RecordingSink {
        RecordingSink {
            trace: String::new(),
        }
    }

    /// Everything heard so far, in emission order.
    pub(super) fn trace(&self) -> &str {
        &self.trace
    }
}

impl TraceSink for RecordingSink {
    fn event(&mut self, event: &TraceEvent) {
        event.render_into(&mut self.trace);
    }
}
