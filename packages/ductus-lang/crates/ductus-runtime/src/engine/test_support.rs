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
use ductus_ir::graph::{Cell, CellKind, Entry, Scope, ValueLiteral};
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
        reset_on_reopen: false,
        init: None,
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

/// A scope holding cells in body order — the rank carrier (R64).
pub(super) fn scope(names: &[&str], cells: Vec<Cell>) -> Scope {
    Scope {
        path: path(names),
        span: Span::new(0, 0),
        exposes: Vec::new(),
        effects: Vec::new(),
        reset_on_reopen: Vec::new(),
        entries: cells.into_iter().map(Entry::Cell).collect(),
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
