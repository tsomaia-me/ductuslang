//! [`load()`] — Module → [`LoadedGraph`]: the loader as the sole legality
//! authority (R21, F4).

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use ductus_ir::diag::{DiagClass, Diagnostic, Span};
use ductus_ir::graph::{Cell, CellKind, Entry, Gate, Scope, ValueLiteral};
use ductus_ir::module::Module;
use ductus_ir::path::{CellPath, Segment};
use ductus_ir::types::TypeTag;

use super::cell_index::CellIndex;
use super::cell_role::CellRole;
use super::gate_index::GateIndex;
use super::loaded_cell::LoadedCell;
use super::loaded_gate::LoadedGate;
use super::loaded_graph::LoadedGraph;
use super::lookback_link::LookbackLink;
use crate::provider::{Behavior, BehaviorProvider};
use crate::value::{Value, ValueType};

/// Load a parsed [`Module`] into a validated, linked, ordered [`LoadedGraph`]
/// (M0.md §2's `load` verb; §13.14.2).
///
/// The parser was a pure grammar machine that over-admitted on purpose; this
/// function owns ALL legality (R21, F4's over-admit/legality split). It is
/// fail-fast — the FIRST error wins, in a deterministic order — and the order
/// is the check order below, which walks the file's own order:
///
/// **Pass A** — per scope in section order, then per entry in body order:
///
/// 1. Scope path shape: keyed or ordinal segments → `outside_m0_slice` (D7:
///    M0 graphs are plain dotted paths).
/// 2. Scope clauses: a non-empty `exposes`, `effects` or `reset_on_reopen` →
///    `outside_m0_slice` (placement/effect/stream machinery is M2/M5).
/// 3. Scope identity, in this order: a first segment that is not the MODULE's
///    name → `unanchored_scope_path` (every path is anchored at the module
///    root, §15.4.1.1); a path some CELL or GATE already claims →
///    `scope_cell_collision`; a path some SCOPE already claims →
///    `duplicate_scope_path`.
/// 4. Entry kind: `wire`/`effect`/`stream` → `outside_m0_slice`. `gate`
///    entries are admitted (R70's cut B) and take their place in the ONE
///    entry-rank sequence beside the cells (C-45): path shape as in 1;
///    CONTAINMENT as in 5; then the same namespace claim a cell makes, with a
///    repeat spelling `duplicate_gate_path`.
/// 5. Per cell: path shape as in 1; kind `fold` → `outside_m0_slice`; a type
///    outside the word tier → `outside_m0_slice`; the kind's clause law
///    (`input` requires `init`, the three computed kinds require `uses`,
///    everything else forbidden — with `init` and `fallbacks` on a
///    `recurrent` mutually exclusive, `conflicting_history_fallbacks`, and a
///    cell naming itself among its `inputs` `deferred_self_reference` /
///    `recurrent_self_reference`); CONTAINMENT — the enclosing scope's
///    path must be a proper prefix of the cell's → `cell_outside_scope`; a
///    path some SCOPE claims → `scope_cell_collision`; a duplicate path (any
///    prior cell, any scope) → `duplicate_cell_path`; an `input`'s `init`
///    conversion under the declared type (R57) → `type_mismatched_init` on
///    failure; a `recurrent`'s HISTORY law — `depth` positive and
///    `lookback` values positive (`invalid_history_depth`), a fallback for
///    every depth `1 ..= depth` and no depth twice (`missing_fallback_key`,
///    `duplicate_fallback_key`, `invalid_history_depth` out of range), each
///    converting under the declared type exactly as `init` does; and a
///    `deferred`'s LATENCY law — an omitted `depth` is 1 and a WRITTEN one is
///    at least 1 (`invalid_history_depth`), since a zero-commit latency is not
///    a delay (016-301).
///
/// Checks 3 and 5's identity rules are ONE namespace seen twice: a path names
/// a scope or a cell, never both, and the SECOND declaration is always the one
/// blamed.
///
/// Containment is what makes two further rules unnecessary. A cell path EQUAL
/// to its scope's is already rejected, because a proper prefix is strictly
/// shorter — so a cell can never collide with its OWN scope, only with another
/// scope's path. And a cell needs no anchoring check: its scope is anchored at
/// the module root and the cell extends its scope, so the cell is anchored
/// too. Depth, by contrast, is FREE — an intermediate segment needs no
/// declared scope, so `scope App { input App.print.text }` is legal with no
/// `App.print` scope anywhere.
///
/// Rank assignment rides this walk (FND-22/R64): each cell takes the next
/// dense [`CellIndex`], so rank = declaration order, scope-major — the
/// normative carrier of §13.10.3's same-level tiebreak.
///
/// **Pass B** — after all cells are collected, because forward references are
/// LEGAL: the graph section is a serialization, not a scoped program text, so
/// a derived at rank 0 may read a cell declared later.
///
/// 6. Every computed cell's `inputs` path, rank order then list order: unknown
///    → `unknown_input_path` carrying the MISSING path. Then, for a
///    `recurrent`, its `lookback` keys in list order: unknown → the same
///    `unknown_input_path`; known but absent from this cell's own `inputs` →
///    `lookback_not_an_input` (R71's ABI order is defined over the `inputs`
///    list, so a key outside it names history for a cell the recurrent does
///    not read).
/// 7. Every computed cell's `uses` handle, rank order: unresolvable through
///    the provider → `unresolvable_behavior` (D5: module table ∪ provider
///    registry — and the module table is empty throughout M0, so the registry
///    is the whole answer).
/// 8. The GATE laws, in gate rank order and then in this order per check:
///    `inputs` paths known → `unknown_input_path`; `guards` paths naming
///    neither a cell nor a gate → `unknown_guard_path`; `gate_parent` naming
///    no gate → `unknown_gate_parent`; a cell's own `gate PATH` naming no gate
///    → `unknown_gate_reference`. The binding is the construct's OWN clause and
///    a gate's `guards` list is its materialized inverse (§15.4.1), so the two
///    sets must be EQUAL — a mismatch in either direction →
///    `guards_inverse_mismatch`; the `gate_parent` chain must
///    terminate → `gate_parent_cycle`; and each `pred` must resolve →
///    `unresolvable_behavior`. A predicate reading a cell its OWN gate guards
///    is LEGAL and needs no check — the self-conditional gate of SPEC §13.9.6,
///    ruled by the owner on 2026-08-24; the predicate runs in step 1a over
///    prior-commit values, so the read is already defined.
/// 9. The ONE topological sort, in two passes over one node set — every
///    computed cell. (a) CYCLE detection: a read of a `derived` or a `deferred`
///    is a cycle edge, and leftovers → `derived_cycle` with the members in the
///    message; a recurrent read is NOT one (§13.11.3, §13.11.4, RW-4), so a
///    loop closed through a recurrent loads, while a loop closed through a
///    `deferred` does NOT (§13.11.4). A recurrent naming ITSELF among `inputs`
///    never reaches here — it is `recurrent_self_reference` at check 5. (b) The
///    ORDER, which §13.14.1 step 3 and §13.10.2 step 2 share: every `inputs`
///    read orders, recurrent reads included (R80 item 4), with a recurrent read
///    dropped only where honouring it would deadlock — which is §13.11.3's
///    previous-committed reading applied to the schedule.
///
/// **What load() does NOT do:** no store allocation, no evaluation, no first
/// commit — those are the engine's (§13.14.1 steps 2–4). This is Module →
/// validated graph with resolved behaviors and the static order, nothing more.
pub fn load(module: &Module, provider: &dyn BehaviorProvider) -> Result<LoadedGraph, Diagnostic> {
    let mut pending: Vec<PendingCell> = Vec::new();
    let mut pending_gates: Vec<PendingGate> = Vec::new();
    let mut index: HashMap<CellPath, CellIndex> = HashMap::new();
    let mut gate_index: HashMap<CellPath, GateIndex> = HashMap::new();
    let mut scopes: HashSet<CellPath> = HashSet::new();
    let mut entry_rank = 0usize;

    // Pass A — file order: scopes in section order, entries in body order,
    // checks 1–5 per construct, rank assigned as each entry survives. Cells and
    // gates take dense indices of their own AND a shared entry rank: the first
    // is where the construct lives, the second is 033-260's declaration order
    // over every entry, which step 1's root walk is sequenced by (C-45).
    for scope in &module.graph.scopes {
        admitted_scope(scope)?;
        anchored(scope, &module.name)?;
        claim_scope(&mut scopes, &index, &gate_index, scope)?;

        for entry in &scope.entries {
            match admitted_entry(entry)? {
                AdmittedEntry::Cell(cell) => {
                    plain_path(&cell.path, cell.span)?;
                    let kind = admitted_kind(cell)?;
                    let value_type = word_tier(cell)?;
                    admitted_clauses(cell, kind)?;
                    contained(scope, &cell.path, cell.span)?;
                    claim(
                        &mut index,
                        &scopes,
                        &gate_index,
                        cell,
                        CellIndex(pending.len()),
                    )?;
                    let form = pending_form(cell, kind, value_type)?;

                    pending.push(PendingCell {
                        path: cell.path.clone(),
                        span: cell.span,
                        entry_rank,
                        value_type,
                        gate: cell.gate.clone(),
                        form,
                    });
                }
                AdmittedEntry::Gate(gate) => {
                    plain_path(&gate.path, gate.span)?;
                    contained(scope, &gate.path, gate.span)?;
                    claim_gate(
                        &mut gate_index,
                        &scopes,
                        &index,
                        gate,
                        GateIndex(pending_gates.len()),
                    )?;

                    pending_gates.push(PendingGate {
                        path: gate.path.clone(),
                        span: gate.span,
                        entry_rank,
                        pred: gate.pred,
                        inputs: gate.inputs.clone(),
                        guards: gate.guards.clone(),
                        gate_parent: gate.gate_parent.clone(),
                    });
                }
            }

            entry_rank += 1;
        }
    }

    // Pass B — checks 6–9 over the collected entries, each check a full
    // rank-order loop of its own so the error order stays deterministic.
    let links = linked_cells(&pending, &index)?;
    let resolved = resolved_cells(pending, links, provider)?;
    let bound = gate_bindings(&pending_gates, &resolved, &index, &gate_index, provider)?;
    acyclic_reads(&resolved)?;

    let static_order = static_order(&resolved);

    let cells = resolved
        .into_iter()
        .zip(bound.cell_gate)
        .map(|(cell, gate)| LoadedCell {
            path: cell.path,
            entry_rank: cell.entry_rank,
            value_type: cell.value_type,
            gate,
            role: cell.role,
        })
        .collect();

    Ok(LoadedGraph {
        cells,
        index,
        static_order,
        gates: bound.gates,
    })
}

/// A cell that passed Pass A: `init` already converted, links not yet made.
///
/// The span rides along so Pass B's diagnostics can still point into the text;
/// it is dropped only when the [`LoadedCell`]s are built, after the last check
/// that could need it.
struct PendingCell {
    path: CellPath,
    span: Span,
    entry_rank: usize,
    value_type: ValueType,
    /// The cell's own `gate PATH` back-reference, still as a path: gates are
    /// collected by the same Pass-A walk, so a forward reference to a gate
    /// declared later resolves only once that walk is done.
    gate: Option<CellPath>,
    form: PendingForm,
}

/// A gate that passed Pass A: claimed its path, took its entry rank, and
/// carries every clause as written. Nothing about it is linked yet — `guards`
/// may name a cell declared in a later scope, exactly as `inputs` may.
struct PendingGate {
    path: CellPath,
    span: Span,
    entry_rank: usize,
    pred: u32,
    inputs: Vec<CellPath>,
    guards: Vec<CellPath>,
    gate_parent: Option<CellPath>,
}

/// The kind-specific half of a [`PendingCell`].
enum PendingForm {
    /// An `input`: its `init`, converted under the declared type (R57).
    Input { init: Value },
    /// A `derived`: its handle and its input PATHS — indices are check 6's.
    Derived { uses: u32, inputs: Vec<CellPath> },
    /// A `recurrent`: the same, plus its history — the ring depth, the
    /// `lookback` keys as PATHS, and the per-`k` fallbacks already converted
    /// under the declared type.
    Recurrent {
        uses: u32,
        inputs: Vec<CellPath>,
        depth: usize,
        lookback: Vec<PendingLookback>,
        fallbacks: Vec<Value>,
        reset_on_reopen: bool,
    },
    /// A `deferred`: its handle, its input PATHS and its latency in commits.
    Deferred {
        uses: u32,
        inputs: Vec<CellPath>,
        depth: usize,
    },
}

/// One `lookback` pair before linking: the key as written, and its already
/// range-checked depth.
struct PendingLookback {
    cell: CellPath,
    k: usize,
}

/// One cell's linked edges — check 6's product.
struct CellLinks {
    /// The `inputs` list as dense indices, in list order.
    inputs: Vec<CellIndex>,
    /// The `lookback` list as linked pairs, in list order; empty for every
    /// kind but `recurrent`.
    lookback: Vec<LookbackLink>,
}

/// A cell past checks 6 and 7: the role is built, the span still at hand for
/// check 8's cycle report.
struct ResolvedCell {
    path: CellPath,
    span: Span,
    entry_rank: usize,
    value_type: ValueType,
    gate: Option<CellPath>,
    role: CellRole,
}

/// Check 8's product: the loaded gates, and each cell's own gate after the two
/// binding spellings are folded together.
///
/// The two travel as one value because they are one answer seen from two ends —
/// the `guards` list that produced a cell's gate is the same list that produced
/// a gate's parent, and neither is derivable from the other afterwards.
struct GateBindings {
    gates: Vec<LoadedGate>,
    /// One entry per cell, in rank order.
    cell_gate: Vec<Option<GateIndex>>,
}

/// The cell kinds inside the implemented slice, as their own type — downstream
/// checks match on this and structurally cannot meet the one kind left out.
#[derive(Clone, Copy)]
enum AdmittedKind {
    Input,
    Derived,
    Recurrent,
    Deferred,
}

// The shared `outside_m0_slice` constructor: every slice check points at the
// offending construct's span and path (R28's class, D7/M0.md §2's law).
fn slice_error(span: Span, path: &CellPath, message: &str) -> Diagnostic {
    Diagnostic {
        class: DiagClass::OutsideM0Slice,
        span: Some(span),
        path: Some(path.clone()),
        message: message.to_string(),
    }
}

// Checks 1 and 5a — M0 graphs are plain dotted paths (D7): every segment a
// bare NAME, no keyed segments, no `:N` ordinals. The parser admitted the full
// path grammar; the slice does not.
fn plain_path(path: &CellPath, span: Span) -> Result<(), Diagnostic> {
    for segment in path.segments() {
        let plain = matches!(segment, Segment::Name { ordinal: None, .. });

        if !plain {
            return Err(slice_error(
                span,
                path,
                "keyed or ordinal path segments are outside the M0 slice \
                 (M0 graphs are plain dotted paths)",
            ));
        }
    }

    Ok(())
}

// Checks 1 and 2 — the scope line: a plain path, and no placement/effect/
// stream machinery (`exposes`/`effects` non-empty, or any `reset_on_reopen`
// pair — M2/M5 matter, all of it).
fn admitted_scope(scope: &Scope) -> Result<(), Diagnostic> {
    plain_path(&scope.path, scope.span)?;

    if !scope.exposes.is_empty() {
        return Err(slice_error(
            scope.span,
            &scope.path,
            "a non-empty `exposes` is outside the M0 slice",
        ));
    }

    if !scope.effects.is_empty() {
        return Err(slice_error(
            scope.span,
            &scope.path,
            "a non-empty `effects` is outside the M0 slice",
        ));
    }

    if !scope.reset_on_reopen.is_empty() {
        return Err(slice_error(
            scope.span,
            &scope.path,
            "a `reset_on_reopen` set is outside the M0 slice",
        ));
    }

    Ok(())
}

// Check 3a — the module-root anchor (§15.4.1.1): a scope path's FIRST segment
// is the module's name. `module App` holds `App`, `App.panel`,
// `App.panel.row` — never `Foo`, and never an `App`-prefixed spelling like
// `Application`, because the segment must BE the name, not begin with it.
//
// The non-name arm is unreachable after check 1 — every segment is a plain
// name by then, and a path has at least one segment — and answers with the
// same diagnostic rather than panicking: a loader answers in diagnostics.
fn anchored(scope: &Scope, module_name: &str) -> Result<(), Diagnostic> {
    let anchor = match scope.path.segments().first() {
        Some(Segment::Name { text, .. }) => Some(text.as_str()),
        Some(Segment::Key(_)) | None => None,
    };

    if anchor == Some(module_name) {
        return Ok(());
    }

    Err(Diagnostic {
        class: DiagClass::UnanchoredScopePath,
        span: Some(scope.span),
        path: Some(scope.path.clone()),
        message: format!(
            "scope `{}` is not anchored at module `{module_name}`",
            scope.path.render()
        ),
    })
}

// The shared `scope_cell_collision` constructor (checks 3b and 5f): scope
// paths and cell paths are ONE namespace, so a path names a region or a cell
// and never both. The construct making the SECOND claim is blamed — its span,
// the contested path.
fn collision(span: Span, path: &CellPath, message: &str) -> Diagnostic {
    Diagnostic {
        class: DiagClass::ScopeCellCollision,
        span: Some(span),
        path: Some(path.clone()),
        message: message.to_string(),
    }
}

// Checks 3b and 3c — the scope's claim on that one namespace: a path some CELL
// already took is a collision, a path another SCOPE already took is a
// duplicate, and the scope surviving both records itself for the cells and
// scopes still to come. Both maps hold only what the walk has already passed,
// so the offender is always this scope and the other declaration is always
// above it.
//
// A `HashSet` rather than a map to spans: nothing ever reports the FIRST
// declaration, so the earlier span would be dead weight.
fn claim_scope(
    scopes: &mut HashSet<CellPath>,
    index: &HashMap<CellPath, CellIndex>,
    gate_index: &HashMap<CellPath, GateIndex>,
    scope: &Scope,
) -> Result<(), Diagnostic> {
    if index.contains_key(&scope.path) {
        return Err(collision(
            scope.span,
            &scope.path,
            &format!(
                "`{}` is declared as a scope here and as a cell above",
                scope.path.render()
            ),
        ));
    }

    if gate_index.contains_key(&scope.path) {
        return Err(collision(
            scope.span,
            &scope.path,
            &format!(
                "`{}` is declared as a scope here and as a gate above",
                scope.path.render()
            ),
        ));
    }

    if !scopes.insert(scope.path.clone()) {
        return Err(Diagnostic {
            class: DiagClass::DuplicateScopePath,
            span: Some(scope.span),
            path: Some(scope.path.clone()),
            message: format!("scope `{}` is declared twice", scope.path.render()),
        });
    }

    Ok(())
}

// The entry kinds inside the implemented slice, as their own type — Pass A
// matches on this and structurally cannot meet the three kinds left out.
enum AdmittedEntry<'e> {
    Cell(&'e Cell),
    Gate(&'e Gate),
}

// Check 4 — the implemented slice holds `cell` and `gate` entries (R70's cut
// B). The three still out are rejected at the entry's own span and path, and
// two of them are out for a reason no milestone boundary describes: `wire` and
// `stream` are PARKED (M1.md §2.3) because the corpus does not define what a
// wire propagates and contradicts itself on streams in four places — a corpus
// boundary, not a slice one, so the message promises nothing.
fn admitted_entry(entry: &Entry) -> Result<AdmittedEntry<'_>, Diagnostic> {
    match entry {
        Entry::Cell(cell) => Ok(AdmittedEntry::Cell(cell)),
        Entry::Gate(gate) => Ok(AdmittedEntry::Gate(gate)),
        Entry::Wire(wire) => Err(slice_error(
            wire.span,
            &wire.path,
            "`wire` entries are outside the M0 slice",
        )),
        Entry::Effect(effect) => Err(slice_error(
            effect.span,
            &effect.path,
            "`effect` entries are outside the M0 slice",
        )),
        Entry::Stream(stream) => Err(slice_error(
            stream.span,
            &stream.path,
            "`stream` entries are outside the M0 slice",
        )),
    }
}

// Check 5b — the implemented cell kinds are `input`, `derived`, `recurrent`
// and `deferred` (M0.md §2 as widened by M1's cut B, R70); `fold` is
// M5-adjacent and the only kind still out.
fn admitted_kind(cell: &Cell) -> Result<AdmittedKind, Diagnostic> {
    match cell.kind {
        CellKind::Input => Ok(AdmittedKind::Input),
        CellKind::Derived => Ok(AdmittedKind::Derived),
        CellKind::Recurrent => Ok(AdmittedKind::Recurrent),
        CellKind::Deferred => Ok(AdmittedKind::Deferred),
        CellKind::Fold => Err(slice_error(
            cell.span,
            &cell.path,
            &format!("`{}` cells are outside the M0 slice", cell.kind.as_str()),
        )),
    }
}

// Check 5c — the M0 value space is the word tier (D2): a `TypeTag::Prim` that
// `ValueType::from_prim` maps. Everything else — `str`, the narrow and wide
// integers, and every non-primitive tag — is outside the slice.
fn word_tier(cell: &Cell) -> Result<ValueType, Diagnostic> {
    let TypeTag::Prim(prim) = &cell.tag else {
        return Err(slice_error(
            cell.span,
            &cell.path,
            "a non-primitive cell type is outside the M0 slice",
        ));
    };

    match ValueType::from_prim(prim) {
        Some(value_type) => Ok(value_type),
        None => Err(slice_error(
            cell.span,
            &cell.path,
            &format!(
                "cell type `{}` is outside the M0 word tier (i64, i32, f64, f32, bool)",
                prim.as_str()
            ),
        )),
    }
}

// Check 5d — the kind's clause law. The parser admitted any clause combination
// in grammar order (F4); which clauses a KIND admits is decided here.
//
// An absent set clause and an empty one are ONE state in this AST — an empty
// vector — so emptiness IS the checkable condition for `inputs` and `members`.
fn admitted_clauses(cell: &Cell, kind: AdmittedKind) -> Result<(), Diagnostic> {
    match kind {
        AdmittedKind::Input => input_clauses(cell),
        AdmittedKind::Derived => derived_clauses(cell),
        AdmittedKind::Recurrent => recurrent_clauses(cell),
        AdmittedKind::Deferred => deferred_clauses(cell),
    }
}

// The `input` row of the clause law — M0.md §2's "input (with init)": `init`
// is REQUIRED, and every other clause belongs to another kind or another
// milestone.
//
// `gate` is admitted on EVERY kind, this one included, and an input's is not
// vacuous even though §13.9.7 keeps input cells live under a closed gate: the
// clause is what says the cell belongs to the guarded subtree, and belonging is
// what makes "the write stages and dirties but nothing downstream evaluates"
// the observable shape rather than an accident of which cells happen to be
// named in a `guards` list.
fn input_clauses(cell: &Cell) -> Result<(), Diagnostic> {
    if cell.init.is_none() {
        return Err(slice_error(
            cell.span,
            &cell.path,
            "an `input` cell without `init` is outside the M0 slice",
        ));
    }

    let forbidden = [
        (cell.uses.is_some(), "uses"),
        (!cell.inputs.is_empty(), "inputs"),
        (cell.depth.is_some(), "depth"),
        (!cell.lookback.is_empty(), "lookback"),
        (cell.reset_on_reopen, "reset_on_reopen"),
        (!cell.fallbacks.is_empty(), "fallbacks"),
        (cell.combiner.is_some(), "combiner"),
        (cell.else_value.is_some(), "else"),
        (!cell.members.is_empty(), "members"),
    ];

    admitted_clause_set(cell, "an `input` cell", &forbidden)
}

// The `derived` row — `uses` is REQUIRED; `inputs` MAY be empty, because a
// constant derived is legal and simply evaluates once at startup; everything
// else is forbidden.
fn derived_clauses(cell: &Cell) -> Result<(), Diagnostic> {
    if cell.uses.is_none() {
        return Err(slice_error(
            cell.span,
            &cell.path,
            "a `derived` cell without `uses` is outside the M0 slice",
        ));
    }

    let forbidden = [
        (cell.depth.is_some(), "depth"),
        (!cell.lookback.is_empty(), "lookback"),
        (cell.reset_on_reopen, "reset_on_reopen"),
        (cell.init.is_some(), "init"),
        (!cell.fallbacks.is_empty(), "fallbacks"),
        (cell.combiner.is_some(), "combiner"),
        (cell.else_value.is_some(), "else"),
        (!cell.members.is_empty(), "members"),
    ];

    admitted_clause_set(cell, "a `derived` cell", &forbidden)
}

// The `recurrent` row — `uses` is REQUIRED, and `inputs`, `depth`, `lookback`,
// the history seeds (`init` OR `fallbacks`) and `reset_on_reopen` are its own.
// What stays forbidden is a fold's machinery: `combiner`, `else`, `members`.
//
// `reset_on_reopen` is a RECURRENT'S clause and no other kind's (C-42). The
// production admits the bare flag on any `cell_kind`, but its whole content is
// "clear the history when the gate reopens", and only a recurrent has history —
// on any other kind the flag would name an act with nothing to act on.
//
// `init` and `fallbacks` are ONE clause in two spellings — `init v` says every
// depth falls back to `v` — so carrying both is a load error of its own class
// rather than a slice complaint (R71: one meaning, one spelling). Which
// spelling a module used stops mattering here; §4.9's law is applied in
// `history_fallbacks`, where the values convert.
//
// The self-reference law rides here too, exactly as it does on the `deferred`
// row: `inputs` carries VALUE reads, and 016-51 puts a recurrent's own past
// behind `.previous`/`.past` — `depth` and `fallbacks` in the IR — so its own
// path has no business in that list.
fn recurrent_clauses(cell: &Cell) -> Result<(), Diagnostic> {
    if cell.uses.is_none() {
        return Err(slice_error(
            cell.span,
            &cell.path,
            "a `recurrent` cell without `uses` is outside the M0 slice",
        ));
    }

    let forbidden = [
        (cell.combiner.is_some(), "combiner"),
        (cell.else_value.is_some(), "else"),
        (!cell.members.is_empty(), "members"),
    ];

    admitted_clause_set(cell, "a `recurrent` cell", &forbidden)?;

    // The self-reference law, the `deferred` row's sibling and the sharper of
    // the two: 016-51 makes prior self-values readable only through
    // `.previous`/`.past`, which ride `depth` and `fallbacks`. An `inputs` entry
    // is a VALUE read, so a recurrent naming itself there asks for the value it
    // is at that moment computing — and, left standing, becomes its own
    // staleness trigger, advancing once per commit forever with no host write.
    if cell.inputs.contains(&cell.path) {
        return Err(Diagnostic {
            class: DiagClass::RecurrentSelfReference,
            span: Some(cell.span),
            path: Some(cell.path.clone()),
            message: format!(
                "`{}` reads its own value; a recurrent's own past rides `depth` \
                 and `fallbacks`, never `inputs`",
                cell.path.render()
            ),
        });
    }

    if cell.init.is_some() && !cell.fallbacks.is_empty() {
        return Err(Diagnostic {
            class: DiagClass::ConflictingHistoryFallbacks,
            span: Some(cell.span),
            path: Some(cell.path.clone()),
            message: format!(
                "`{}` carries both `init` and `fallbacks`; `init` IS the uniform \
                 `fallbacks` spelling",
                cell.path.render()
            ),
        });
    }

    Ok(())
}

// The `deferred` row — `uses` is REQUIRED; `depth` is OPTIONAL and defaults to
// 1 (016-301), and it is not a history depth but a LATENCY in commits
// (033-245). `inputs` MAY be empty, on the same reading `derived` gets: an
// absent clause and an empty one are one state in this AST, and a deferred with
// no provenance simply never dispatches after construction.
//
// What is forbidden is everything a deferred has no use for. The history
// clauses are the interesting half: `init`, `fallbacks` and `lookback` all
// answer "what does a history read yield before the ring has advanced that
// far", and a deferred has no ring — a latency is not a memory. Its startup
// value comes from the same place a derived's does, a synchronous construction
// evaluation (C-23, 016-298), so there is nothing left for `init` to say.
//
// The self-reference law rides here too, because it is the deferred's own law
// rather than a clause question: §13.2.12's "its expression may not read the
// cell it declares".
fn deferred_clauses(cell: &Cell) -> Result<(), Diagnostic> {
    if cell.uses.is_none() {
        return Err(slice_error(
            cell.span,
            &cell.path,
            "a `deferred` cell without `uses` is outside the M0 slice",
        ));
    }

    let forbidden = [
        (!cell.lookback.is_empty(), "lookback"),
        (cell.reset_on_reopen, "reset_on_reopen"),
        (cell.init.is_some(), "init"),
        (!cell.fallbacks.is_empty(), "fallbacks"),
        (cell.combiner.is_some(), "combiner"),
        (cell.else_value.is_some(), "else"),
        (!cell.members.is_empty(), "members"),
    ];

    admitted_clause_set(cell, "a `deferred` cell", &forbidden)?;

    if cell.inputs.contains(&cell.path) {
        return Err(Diagnostic {
            class: DiagClass::DeferredSelfReference,
            span: Some(cell.span),
            path: Some(cell.path.clone()),
            message: format!(
                "`{}` reads itself; a deferred is a latency, not a cycle device",
                cell.path.render()
            ),
        });
    }

    Ok(())
}

// The shared tail of the four clause rows: the FIRST present forbidden clause,
// in the production's clause order, is the one reported.
fn admitted_clause_set(
    cell: &Cell,
    kind: &str,
    forbidden: &[(bool, &str)],
) -> Result<(), Diagnostic> {
    for (present, clause) in forbidden {
        if *present {
            return Err(slice_error(
                cell.span,
                &cell.path,
                &format!("`{clause}` on {kind} is outside the M0 slice"),
            ));
        }
    }

    Ok(())
}

// Check 5e — containment: the enclosing scope's path is a PROPER prefix of the
// entry's, segment by segment. The braces and the dots carry ONE hierarchy, so
// a cell written inside `scope App.panel` lives at `App.panel.…` and nowhere
// else. A `gate` entry answers to the same law and for the same reason: its
// path joins the one namespace, so it names a place in the same tree.
//
// Two consequences are load-bearing elsewhere. A cell path EQUAL to its
// scope's is rejected HERE, because a proper prefix is strictly shorter — so a
// cell can never collide with its own scope, only with another scope's path.
// And DEPTH IS FREE: only the prefix is law, so `scope App { input
// App.print.text }` is legal with no `App.print` scope declared anywhere.
//
// The comparison is structural — `Segment`'s own equality, segment by segment,
// the same authority `CellPath` compares by — never rendered text.
fn contained(scope: &Scope, path: &CellPath, span: Span) -> Result<(), Diagnostic> {
    let enclosing = scope.path.segments();
    let inner = path.segments();

    if inner.len() > enclosing.len() && inner.starts_with(enclosing) {
        return Ok(());
    }

    Err(Diagnostic {
        class: DiagClass::CellOutsideScope,
        span: Some(span),
        path: Some(path.clone()),
        message: format!(
            "`{}` does not extend its scope `{}`",
            path.render(),
            scope.path.render()
        ),
    })
}

// Checks 5f and 5g — the cell's claim on the namespace check 3 governs: a path
// some SCOPE took is a collision, a path some earlier CELL took is a
// duplicate. Cell identity is BY PATH, across every scope (M0.md §2), so the
// map that will serve the engine's lookups is also the duplicate check. The
// SECOND declaration is the one blamed: its span, the shared path.
fn claim(
    index: &mut HashMap<CellPath, CellIndex>,
    scopes: &HashSet<CellPath>,
    gate_index: &HashMap<CellPath, GateIndex>,
    cell: &Cell,
    rank: CellIndex,
) -> Result<(), Diagnostic> {
    if scopes.contains(&cell.path) {
        return Err(collision(
            cell.span,
            &cell.path,
            &format!(
                "`{}` is declared as a cell here and as a scope above",
                cell.path.render()
            ),
        ));
    }

    if gate_index.contains_key(&cell.path) {
        return Err(collision(
            cell.span,
            &cell.path,
            &format!(
                "`{}` is declared as a cell here and as a gate above",
                cell.path.render()
            ),
        ));
    }

    if index.insert(cell.path.clone(), rank).is_some() {
        return Err(Diagnostic {
            class: DiagClass::DuplicateCellPath,
            span: Some(cell.span),
            path: Some(cell.path.clone()),
            message: format!("cell `{}` is declared twice", cell.path.render()),
        });
    }

    Ok(())
}

// Check 4's identity half — the GATE's claim on the same one namespace. A gate
// is not a cell and occupies no store slot, but its path is still a place in
// the tree (§15.4.1), so the three claim kinds are exclusive in every pairing:
// a path already taken by a scope or a cell is a collision, and a path a
// previous GATE took is `duplicate_gate_path` — the gate-side sibling of
// `duplicate_scope_path` and `duplicate_cell_path`.
fn claim_gate(
    gate_index: &mut HashMap<CellPath, GateIndex>,
    scopes: &HashSet<CellPath>,
    index: &HashMap<CellPath, CellIndex>,
    gate: &Gate,
    rank: GateIndex,
) -> Result<(), Diagnostic> {
    if scopes.contains(&gate.path) {
        return Err(collision(
            gate.span,
            &gate.path,
            &format!(
                "`{}` is declared as a gate here and as a scope above",
                gate.path.render()
            ),
        ));
    }

    if index.contains_key(&gate.path) {
        return Err(collision(
            gate.span,
            &gate.path,
            &format!(
                "`{}` is declared as a gate here and as a cell above",
                gate.path.render()
            ),
        ));
    }

    if gate_index.insert(gate.path.clone(), rank).is_some() {
        return Err(Diagnostic {
            class: DiagClass::DuplicateGatePath,
            span: Some(gate.span),
            path: Some(gate.path.clone()),
            message: format!("gate `{}` is declared twice", gate.path.render()),
        });
    }

    Ok(())
}

// Check 5h and the collected form. An `input`'s `init` converts HERE, under
// the declared type — R57's other half: the parser converted structural
// numbers where the position's type was known, and a typed value waited for
// this exact place. A `derived` carries its handle and its input paths into
// Pass B untouched.
//
// The two required-clause `else` branches repeat check 5d's law as a safety
// net: this function's contract does not depend on being called after
// `admitted_clauses`, and the diagnostic is the same either way.
fn pending_form(
    cell: &Cell,
    kind: AdmittedKind,
    value_type: ValueType,
) -> Result<PendingForm, Diagnostic> {
    match kind {
        AdmittedKind::Input => {
            let Some(literal) = &cell.init else {
                return Err(slice_error(
                    cell.span,
                    &cell.path,
                    "an `input` cell without `init` is outside the M0 slice",
                ));
            };

            let init = converted_value(cell, literal, value_type, "init")?;

            Ok(PendingForm::Input { init })
        }
        AdmittedKind::Derived => {
            let Some(uses) = cell.uses else {
                return Err(slice_error(
                    cell.span,
                    &cell.path,
                    "a `derived` cell without `uses` is outside the M0 slice",
                ));
            };

            Ok(PendingForm::Derived {
                uses,
                inputs: cell.inputs.clone(),
            })
        }
        AdmittedKind::Recurrent => {
            let Some(uses) = cell.uses else {
                return Err(slice_error(
                    cell.span,
                    &cell.path,
                    "a `recurrent` cell without `uses` is outside the M0 slice",
                ));
            };

            let depth = history_depth(cell)?;
            let lookback = history_lookback(cell)?;
            let fallbacks = history_fallbacks(cell, depth, value_type)?;

            Ok(PendingForm::Recurrent {
                uses,
                inputs: cell.inputs.clone(),
                depth,
                lookback,
                fallbacks,
                reset_on_reopen: cell.reset_on_reopen,
            })
        }
        AdmittedKind::Deferred => {
            let Some(uses) = cell.uses else {
                return Err(slice_error(
                    cell.span,
                    &cell.path,
                    "a `deferred` cell without `uses` is outside the M0 slice",
                ));
            };

            Ok(PendingForm::Deferred {
                uses,
                inputs: cell.inputs.clone(),
                depth: latency_depth(cell)?,
            })
        }
    }
}

// The `depth` law on a `deferred` (C-24) — the same shape as the recurrent's,
// because 016-301 gives them the same default in one sentence: *"an omitted
// depth on a recurrent is 1, an omitted depth on a `deferred` is 1, and a
// written depth of 0 on either kind is rejected"*. Only a WRITTEN zero is the
// error: a zero-commit latency is a `derived` under another name, while an
// omitted clause is the corpus's own default and says one commit.
fn latency_depth(cell: &Cell) -> Result<usize, Diagnostic> {
    let declared = cell.depth.unwrap_or(1);

    match usize::try_from(declared) {
        Ok(depth) if depth >= 1 => Ok(depth),
        _ => Err(invalid_depth(
            cell,
            &format!(
                "`{}` declares `depth {declared}`; a deferred's latency is at least 1 commit",
                cell.path.render()
            ),
        )),
    }
}

// The `depth` law (C-7/C-8): an omitted clause means ONE (016-48's language
// default applied at the text form), and a zero is a load error — a `.duir`
// line that means nothing at runtime, since a zero-slot ring names no ABI
// parameter and allocates nothing.
//
// The `usize` conversion is the same check seen from the machine's end: a depth
// no ring could be allocated for is rejected here rather than at the
// allocation. The corpus states no upper bound (C-25 leaves it OPEN), so this
// is the machine's bound and nothing more.
fn history_depth(cell: &Cell) -> Result<usize, Diagnostic> {
    let declared = cell.depth.unwrap_or(1);

    match usize::try_from(declared) {
        Ok(depth) if depth >= 1 => Ok(depth),
        _ => Err(invalid_depth(
            cell,
            &format!(
                "`{}` declares `depth {declared}`; a recurrent's history depth is at least 1",
                cell.path.render()
            ),
        )),
    }
}

// The `lookback` values' law — the same positivity, per entry, in list order.
// The KEYS are check 6's: they name cells, and a path is only resolvable once
// every cell is collected.
fn history_lookback(cell: &Cell) -> Result<Vec<PendingLookback>, Diagnostic> {
    let mut pending = Vec::with_capacity(cell.lookback.len());

    for entry in &cell.lookback {
        let k = match usize::try_from(entry.k) {
            Ok(k) if k >= 1 => k,
            _ => {
                return Err(invalid_depth(
                    cell,
                    &format!(
                        "`{}` looks back {} on `{}`; a lookback depth is at least 1",
                        cell.path.render(),
                        entry.k,
                        entry.cell.render()
                    ),
                ));
            }
        };

        pending.push(PendingLookback {
            cell: entry.cell.clone(),
            k,
        });
    }

    Ok(pending)
}

// The history seeds, as the engine needs them: one value per depth `1 ..=
// depth`, index `k - 1` (R71's §4.9).
//
// `init v` is the UNIFORM spelling — every depth falls back to `v` — and
// `fallbacks` is the general form; `recurrent_clauses` has already refused a
// cell carrying both. What this function adds is TOTALITY: every depth the
// behavior's `pastK` parameter can read needs a seed, because the first
// evaluation reads all of them before a single advance exists. A map with a
// hole, and a recurrent carrying neither clause, are the same failure —
// `missing_fallback_key` — and the runtime invents nothing to fill it.
//
// Per entry the order is range, then duplicate, then conversion: structural
// questions before typed ones, which is R57's split.
fn history_fallbacks(
    cell: &Cell,
    depth: usize,
    value_type: ValueType,
) -> Result<Vec<Value>, Diagnostic> {
    if let Some(literal) = &cell.init {
        let uniform = converted_value(cell, literal, value_type, "init")?;

        return Ok(vec![uniform; depth]);
    }

    let mut seeds: Vec<Option<Value>> = vec![None; depth];

    for entry in &cell.fallbacks {
        let k = match usize::try_from(entry.k) {
            Ok(k) if k >= 1 && k <= depth => k,
            _ => {
                return Err(invalid_depth(
                    cell,
                    &format!(
                        "`{}` seeds depth {} of a `depth {depth}` history",
                        cell.path.render(),
                        entry.k
                    ),
                ));
            }
        };

        if seeds[k - 1].is_some() {
            return Err(Diagnostic {
                class: DiagClass::DuplicateFallbackKey,
                span: Some(cell.span),
                path: Some(cell.path.clone()),
                message: format!("`{}` seeds depth {k} twice", cell.path.render()),
            });
        }

        seeds[k - 1] = Some(converted_value(
            cell,
            &entry.value,
            value_type,
            &format!("fallbacks [{k}]"),
        )?);
    }

    let mut fallbacks = Vec::with_capacity(depth);

    for (index, seed) in seeds.into_iter().enumerate() {
        let Some(value) = seed else {
            return Err(Diagnostic {
                class: DiagClass::MissingFallbackKey,
                span: Some(cell.span),
                path: Some(cell.path.clone()),
                message: format!(
                    "`{}` has no fallback for depth {}; every depth of the history is \
                     read before it has advanced that far",
                    cell.path.render(),
                    index + 1
                ),
            });
        };

        fallbacks.push(value);
    }

    Ok(fallbacks)
}

// The shared `invalid_history_depth` constructor: a depth that means nothing at
// runtime, blamed at the cell that declared it.
fn invalid_depth(cell: &Cell, message: &str) -> Diagnostic {
    Diagnostic {
        class: DiagClass::InvalidHistoryDepth,
        span: Some(cell.span),
        path: Some(cell.path.clone()),
        message: message.to_string(),
    }
}

// The conversion table (R57), STRICT by lexeme family: each literal family
// converts under exactly the types its lexeme names, and every other
// (family, type) pair is `type_mismatched_init` — an `Int` under a float type
// does not helpfully widen, because the canonical text form has exactly one
// spelling per value and `init 1` under `f64` is not it.
//
// Both dimensions are matched exhaustively, so a new literal family or a new
// word-tier type must decide its rows here before the crate compiles.
//
// `clause` names the position for the diagnostic — `init`, or `fallbacks [k]`
// for a recurrent's per-depth seed, which converts under exactly the same law
// (M1.md §5.1: "fallback values convert under the cell's type … as `init`
// does").
fn converted_value(
    cell: &Cell,
    literal: &ValueLiteral,
    value_type: ValueType,
    clause: &str,
) -> Result<Value, Diagnostic> {
    let converted = match literal {
        // INT text parses at the declared integer width; overflow and
        // underflow are conversion failures, not saturations.
        ValueLiteral::Int(text) => match value_type {
            ValueType::I64 => text.parse::<i64>().ok().map(Value::I64),
            ValueType::I32 => text.parse::<i32>().ok().map(Value::I32),
            ValueType::F64 | ValueType::F32 | ValueType::Bool => None,
        },
        // FLOAT text parses DIRECTLY at the target width, never via f64 — one
        // rounding, at the type the cell declared. Out-of-range text rounds
        // per IEEE — to ±inf at the extreme — which is float semantics,
        // accepted; canonical FLOAT text never fails to parse, so the `None`
        // here covers only hand-built junk literals.
        ValueLiteral::Float(text) => match value_type {
            ValueType::F64 => text.parse::<f64>().ok().map(Value::F64),
            ValueType::F32 => text.parse::<f32>().ok().map(Value::F32),
            ValueType::I64 | ValueType::I32 | ValueType::Bool => None,
        },
        // The three float words are TYPE-GATED (033-248): legal exactly under
        // the float types, at the declared width's own constants.
        ValueLiteral::Nan => match value_type {
            ValueType::F64 => Some(Value::F64(f64::NAN)),
            ValueType::F32 => Some(Value::F32(f32::NAN)),
            ValueType::I64 | ValueType::I32 | ValueType::Bool => None,
        },
        ValueLiteral::Inf => match value_type {
            ValueType::F64 => Some(Value::F64(f64::INFINITY)),
            ValueType::F32 => Some(Value::F32(f32::INFINITY)),
            ValueType::I64 | ValueType::I32 | ValueType::Bool => None,
        },
        ValueLiteral::NegInf => match value_type {
            ValueType::F64 => Some(Value::F64(f64::NEG_INFINITY)),
            ValueType::F32 => Some(Value::F32(f32::NEG_INFINITY)),
            ValueType::I64 | ValueType::I32 | ValueType::Bool => None,
        },
        ValueLiteral::True => match value_type {
            ValueType::Bool => Some(Value::Bool(true)),
            ValueType::I64 | ValueType::I32 | ValueType::F64 | ValueType::F32 => None,
        },
        ValueLiteral::False => match value_type {
            ValueType::Bool => Some(Value::Bool(false)),
            ValueType::I64 | ValueType::I32 | ValueType::F64 | ValueType::F32 => None,
        },
        // No M0 type holds a string — a `str`-typed cell was already rejected
        // at check 5c — so a STRING literal mismatches every type it could
        // stand under.
        ValueLiteral::Str(_) => None,
    };

    match converted {
        Some(value) => Ok(value),
        None => Err(Diagnostic {
            class: DiagClass::TypeMismatchedInit,
            span: Some(cell.span),
            path: Some(cell.path.clone()),
            message: format!(
                "`{clause}` of `{}` does not convert under `{}`",
                cell.path.render(),
                value_type.as_str()
            ),
        }),
    }
}

// Check 6 — link every computed cell's `inputs` paths to dense indices: rank
// order, then list order, then (for a recurrent) its `lookback` keys. Forward
// references resolve like any other — the whole point of linking after Pass A —
// and the diagnostic carries the MISSING path, with the reading cell's span.
//
// A `lookback` key answers to TWO laws, and they report differently on purpose:
// a path no cell declares is the same `unknown_input_path` any unknown path
// gets, while a path that names a real cell the recurrent does not READ is
// `lookback_not_an_input` — R71's ABI order is defined over the `inputs` list,
// so history for a cell outside it has no argument position to arrive in.
fn linked_cells(
    pending: &[PendingCell],
    index: &HashMap<CellPath, CellIndex>,
) -> Result<Vec<CellLinks>, Diagnostic> {
    let mut links = Vec::with_capacity(pending.len());

    for cell in pending {
        let inputs = match &cell.form {
            PendingForm::Input { .. } => Vec::new(),
            PendingForm::Derived { inputs, .. }
            | PendingForm::Recurrent { inputs, .. }
            | PendingForm::Deferred { inputs, .. } => linked_paths(cell, inputs, index)?,
        };

        let mut lookback = Vec::new();

        if let PendingForm::Recurrent { lookback: keys, .. } = &cell.form {
            for key in keys {
                let input = linked_path(cell, &key.cell, index)?;

                if !inputs.contains(&input) {
                    return Err(Diagnostic {
                        class: DiagClass::LookbackNotAnInput,
                        span: Some(cell.span),
                        path: Some(key.cell.clone()),
                        message: format!(
                            "`{}` looks back on `{}`, which is not among its `inputs`",
                            cell.path.render(),
                            key.cell.render()
                        ),
                    });
                }

                lookback.push(LookbackLink { input, k: key.k });
            }
        }

        links.push(CellLinks { inputs, lookback });
    }

    Ok(links)
}

// One cell's path list, linked in list order.
fn linked_paths(
    cell: &PendingCell,
    paths: &[CellPath],
    index: &HashMap<CellPath, CellIndex>,
) -> Result<Vec<CellIndex>, Diagnostic> {
    let mut linked = Vec::with_capacity(paths.len());

    for path in paths {
        linked.push(linked_path(cell, path, index)?);
    }

    Ok(linked)
}

// One path, linked — the `unknown_input_path` site, carrying the MISSING path
// with the reading cell's span.
fn linked_path(
    cell: &PendingCell,
    path: &CellPath,
    index: &HashMap<CellPath, CellIndex>,
) -> Result<CellIndex, Diagnostic> {
    match index.get(path) {
        Some(&target) => Ok(target),
        None => Err(Diagnostic {
            class: DiagClass::UnknownInputPath,
            span: Some(cell.span),
            path: Some(path.clone()),
            message: format!(
                "`{}` reads `{}`, which no cell declares",
                cell.path.render(),
                path.render()
            ),
        }),
    }
}

// Check 7 — resolve every computed cell's handle through the provider, rank
// order. D5: resolvability = module table ∪ provider registry, and the module's
// behavior table is empty throughout M0, so the registry is the whole answer.
// The resolved `Rc` is kept in the role — a clone shares the executable.
//
// The behavior's ARITY is not checked, and cannot be: [`Behavior`] is an
// `Rc<dyn Fn(&[Value]) -> Value>`, so the number of parameters a stub expects
// is not knowable through the provider seam. A recurrent's expected arity is
// computable — `inputs.len() + depth + sum(lookback k)` (R71) — but the
// counterpart to compare it against does not exist. M1.md §5.1's
// `behavior_arity_mismatch` row therefore waits on a seam change (D5), and a
// mismatched stub still fails where every wrong stub fails: inside the
// behavior, as a trap naming its cell (D8).
fn resolved_cells(
    pending: Vec<PendingCell>,
    links: Vec<CellLinks>,
    provider: &dyn BehaviorProvider,
) -> Result<Vec<ResolvedCell>, Diagnostic> {
    let mut resolved = Vec::with_capacity(pending.len());

    for (cell, linked) in pending.into_iter().zip(links) {
        let PendingCell {
            path,
            span,
            entry_rank,
            value_type,
            gate,
            form,
        } = cell;

        let role = match form {
            PendingForm::Input { init } => CellRole::Input { init },
            PendingForm::Derived { uses, .. } => CellRole::Derived {
                behavior: behavior(provider, &path, span, uses)?,
                inputs: linked.inputs,
            },
            PendingForm::Recurrent {
                uses,
                depth,
                fallbacks,
                reset_on_reopen,
                ..
            } => CellRole::Recurrent {
                behavior: behavior(provider, &path, span, uses)?,
                inputs: linked.inputs,
                depth,
                lookback: linked.lookback,
                fallbacks,
                reset_on_reopen,
            },
            PendingForm::Deferred { uses, depth, .. } => CellRole::Deferred {
                behavior: behavior(provider, &path, span, uses)?,
                inputs: linked.inputs,
                depth,
            },
        };

        resolved.push(ResolvedCell {
            path,
            span,
            entry_rank,
            value_type,
            gate,
            role,
        });
    }

    Ok(resolved)
}

// One handle, resolved — the `unresolvable_behavior` site.
fn behavior(
    provider: &dyn BehaviorProvider,
    path: &CellPath,
    span: Span,
    uses: u32,
) -> Result<Behavior, Diagnostic> {
    provider.resolve(uses).ok_or_else(|| Diagnostic {
        class: DiagClass::UnresolvableBehavior,
        span: Some(span),
        path: Some(path.clone()),
        message: format!(
            "`{}` uses B@{uses:x}, which neither the module table nor the provider defines",
            path.render()
        ),
    })
}

// What one `guards` entry turned out to name. A gate guards CONSTRUCTS, and in
// the implemented slice a construct is a cell or another gate — the second is
// how gates nest from the outside, which is the same nesting `gate_parent`
// spells from the inside.
#[derive(Clone, Copy)]
enum GuardTarget {
    Cell(CellIndex),
    Gate(GateIndex),
}

// Check 8 — the gate laws, as ONE function, because they are one composition
// rather than a list of independent tests: the construct's own clause IS the
// binding and the `guards` lists must restate it exactly (R80 item 5), and the
// acyclicity that follows is a question about that one binding.
//
// The sub-checks run as full gate-rank loops, one per law, so the error order is
// deterministic and does not depend on which gate happens to be malformed in two
// ways at once.
fn gate_bindings(
    pending: &[PendingGate],
    cells: &[ResolvedCell],
    index: &HashMap<CellPath, CellIndex>,
    gate_index: &HashMap<CellPath, GateIndex>,
    provider: &dyn BehaviorProvider,
) -> Result<GateBindings, Diagnostic> {
    let mut inputs: Vec<Vec<CellIndex>> = Vec::with_capacity(pending.len());

    for gate in pending {
        inputs.push(gate_inputs(gate, index)?);
    }

    let mut guards: Vec<Vec<GuardTarget>> = Vec::with_capacity(pending.len());

    for gate in pending {
        guards.push(gate_guards(gate, index, gate_index)?);
    }

    let mut parent: Vec<Option<GateIndex>> = Vec::with_capacity(pending.len());

    for gate in pending {
        parent.push(declared_parent(gate, gate_index)?);
    }

    let mut cell_gate: Vec<Option<GateIndex>> = Vec::with_capacity(cells.len());

    for cell in cells {
        cell_gate.push(declared_gate(cell, gate_index)?);
    }

    guards_inverse(pending, cells, &guards, &parent, &cell_gate)?;
    acyclic_parents(pending, &parent)?;

    let mut predicates = Vec::with_capacity(pending.len());

    for gate in pending {
        predicates.push(behavior(provider, &gate.path, gate.span, gate.pred)?);
    }

    let gates = pending
        .iter()
        .zip(inputs)
        .zip(predicates)
        .enumerate()
        .map(|(rank, ((gate, inputs), predicate))| LoadedGate {
            path: gate.path.clone(),
            entry_rank: gate.entry_rank,
            predicate,
            inputs,
            parent: parent[rank],
        })
        .collect();

    Ok(GateBindings { gates, cell_gate })
}

// One gate's `inputs`, linked in list order. A predicate reads ordinary graph
// cells, so an unknown path is the same `unknown_input_path` a derived's is —
// the reading construct is a gate rather than a cell, and nothing else differs.
fn gate_inputs(
    gate: &PendingGate,
    index: &HashMap<CellPath, CellIndex>,
) -> Result<Vec<CellIndex>, Diagnostic> {
    let mut linked = Vec::with_capacity(gate.inputs.len());

    for path in &gate.inputs {
        let Some(&cell) = index.get(path) else {
            return Err(Diagnostic {
                class: DiagClass::UnknownInputPath,
                span: Some(gate.span),
                path: Some(path.clone()),
                message: format!(
                    "gate `{}` reads `{}`, which no cell declares",
                    gate.path.render(),
                    path.render()
                ),
            });
        };

        linked.push(cell);
    }

    Ok(linked)
}

// One gate's `guards`, linked in list order. A cell is looked up first because
// cells are the ordinary case; a path naming neither a cell nor a gate guards
// nothing, and gets its own class rather than `unknown_input_path` — a guard is
// not a read.
fn gate_guards(
    gate: &PendingGate,
    index: &HashMap<CellPath, CellIndex>,
    gate_index: &HashMap<CellPath, GateIndex>,
) -> Result<Vec<GuardTarget>, Diagnostic> {
    let mut linked = Vec::with_capacity(gate.guards.len());

    for path in &gate.guards {
        let target = match (index.get(path), gate_index.get(path)) {
            (Some(&cell), _) => GuardTarget::Cell(cell),
            (None, Some(&inner)) => GuardTarget::Gate(inner),
            (None, None) => {
                return Err(Diagnostic {
                    class: DiagClass::UnknownGuardPath,
                    span: Some(gate.span),
                    path: Some(path.clone()),
                    message: format!(
                        "gate `{}` guards `{}`, which names neither a cell nor a gate",
                        gate.path.render(),
                        path.render()
                    ),
                });
            }
        };

        linked.push(target);
    }

    Ok(linked)
}

// One gate's own `gate_parent` clause, resolved. It must name a GATE: the
// clause is the link effective activation climbs (§13.9.7), and a cell has no
// activation to contribute.
fn declared_parent(
    gate: &PendingGate,
    gate_index: &HashMap<CellPath, GateIndex>,
) -> Result<Option<GateIndex>, Diagnostic> {
    let Some(path) = &gate.gate_parent else {
        return Ok(None);
    };

    match gate_index.get(path) {
        Some(&parent) => Ok(Some(parent)),
        None => Err(Diagnostic {
            class: DiagClass::UnknownGateParent,
            span: Some(gate.span),
            path: Some(path.clone()),
            message: format!(
                "gate `{}` names `{}` as its parent, which no gate declares",
                gate.path.render(),
                path.render()
            ),
        }),
    }
}

// One cell's own `gate PATH` clause, resolved — IR_GRAMMAR §3's back-reference
// to a `gate` entry. A path naming a cell, or nothing, declares no activation
// condition.
fn declared_gate(
    cell: &ResolvedCell,
    gate_index: &HashMap<CellPath, GateIndex>,
) -> Result<Option<GateIndex>, Diagnostic> {
    let Some(path) = &cell.gate else {
        return Ok(None);
    };

    match gate_index.get(path) {
        Some(&gate) => Ok(Some(gate)),
        None => Err(Diagnostic {
            class: DiagClass::UnknownGateReference,
            span: Some(cell.span),
            path: Some(path.clone()),
            message: format!(
                "`{}` is gated by `{}`, which no gate declares",
                cell.path.render(),
                path.render()
            ),
        }),
    }
}

// The STRICT INVERSE (C-43, R80 item 5). A construct's own clause — `gate PATH`
// on a cell, `gate_parent PATH` on a gate — is the ONE binding; §15.4.1's
// `guards` list is "the materialized inverse" of it, and an inverse is a
// restatement rather than a second source of truth. So the loader checks EQUALITY
// of the two sets and binds nothing: a gate's `guards` must name exactly the
// constructs whose own clause names that gate.
//
// Both directions of inequality are `guards_inverse_mismatch`, in this order:
//
// - forward, per gate in rank order, per `guards` entry in list order — a target
//   whose own clause names another gate, or none at all. Two gates listing one
//   construct arrives here too: at most one of them is the construct's answer,
//   so the other is a mismatch;
// - backward, cells in rank order then gates in rank order — a construct whose
//   own clause names a gate whose `guards` list omits it.
//
// Naming the same target twice in one list is not a disagreement: set equality
// is what is checked, and a repeat adds no member.
fn guards_inverse(
    pending: &[PendingGate],
    cells: &[ResolvedCell],
    guards: &[Vec<GuardTarget>],
    parent: &[Option<GateIndex>],
    cell_gate: &[Option<GateIndex>],
) -> Result<(), Diagnostic> {
    for (rank, targets) in guards.iter().enumerate() {
        let gate = GateIndex(rank);

        for target in targets {
            let (bound, span, path, clause) = match *target {
                GuardTarget::Cell(cell) => (
                    cell_gate[cell.0],
                    cells[cell.0].span,
                    &cells[cell.0].path,
                    "gate",
                ),
                GuardTarget::Gate(inner) => (
                    parent[inner.0],
                    pending[inner.0].span,
                    &pending[inner.0].path,
                    "gate_parent",
                ),
            };

            if bound == Some(gate) {
                continue;
            }

            return Err(inverse_mismatch(
                span,
                path,
                &format!(
                    "gate `{}` guards `{}`, but `{}`'s own `{clause}` clause names {}",
                    pending[rank].path.render(),
                    path.render(),
                    path.render(),
                    bound_gate(pending, bound)
                ),
            ));
        }
    }

    for (rank, bound) in cell_gate.iter().enumerate() {
        let Some(gate) = *bound else {
            continue;
        };

        if guards[gate.0]
            .iter()
            .any(|target| matches!(target, GuardTarget::Cell(cell) if cell.0 == rank))
        {
            continue;
        }

        return Err(unlisted(pending, gate, &cells[rank].path));
    }

    for (rank, bound) in parent.iter().enumerate() {
        let Some(gate) = *bound else {
            continue;
        };

        if guards[gate.0]
            .iter()
            .any(|target| matches!(target, GuardTarget::Gate(inner) if inner.0 == rank))
        {
            continue;
        }

        return Err(unlisted(pending, gate, &pending[rank].path));
    }

    Ok(())
}

// The backward half's diagnostic: the construct named a gate, and that gate's
// `guards` list does not name it back. The GATE's span is reported — its list is
// the incomplete one — with the unlisted construct's path.
fn unlisted(pending: &[PendingGate], gate: GateIndex, path: &CellPath) -> Diagnostic {
    inverse_mismatch(
        pending[gate.0].span,
        path,
        &format!(
            "`{}` names gate `{}`, which does not name it in `guards`",
            path.render(),
            pending[gate.0].path.render()
        ),
    )
}

// The shared `guards_inverse_mismatch` constructor.
fn inverse_mismatch(span: Span, path: &CellPath, message: &str) -> Diagnostic {
    Diagnostic {
        class: DiagClass::GuardsInverseMismatch,
        span: Some(span),
        path: Some(path.clone()),
        message: message.to_string(),
    }
}

// How a construct's own binding reads in a message: the gate it names, or the
// absence of one.
fn bound_gate(pending: &[PendingGate], bound: Option<GateIndex>) -> String {
    match bound {
        Some(gate) => format!("`{}`", pending[gate.0].path.render()),
        None => "no gate".to_string(),
    }
}

// The chain must terminate: effective activation is "its own gate AND every
// ancestor gate on the path from the root" (§13.9.7), and a loop has no root.
// The corpus states no rule (C-47); this is the loader's.
//
// Walking from every gate and reporting the first REPEAT names a genuine member
// of the loop rather than whichever gate happened to lead into it.
fn acyclic_parents(
    pending: &[PendingGate],
    parent: &[Option<GateIndex>],
) -> Result<(), Diagnostic> {
    for rank in 0..pending.len() {
        let mut seen: HashSet<usize> = HashSet::new();
        let mut at = Some(GateIndex(rank));

        while let Some(gate) = at {
            if !seen.insert(gate.0) {
                return Err(Diagnostic {
                    class: DiagClass::GateParentCycle,
                    span: Some(pending[gate.0].span),
                    path: Some(pending[gate.0].path.clone()),
                    message: format!(
                        "gate `{}` encloses itself through `gate_parent`",
                        pending[gate.0].path.render()
                    ),
                });
            }

            at = parent[gate.0];
        }
    }

    Ok(())
}

// Whether this cell is a NODE of the one order — every COMPUTED cell is, an
// `input` never is: an input is written, never evaluated.
//
// A `deferred` is a node too, and that is the widening R80 item 4 makes: the
// construction pass evaluates it synchronously (C-23, 016-298) and the commit
// pass SKIPS it, so it takes a place in the order and step 3 walks past it —
// §13.10.2's "the cell's own body is never a step-2 DAG node" is honoured where
// the walk happens rather than by keeping a second vector.
fn is_node(role: &CellRole) -> bool {
    !matches!(role, CellRole::Input { .. })
}

// Whether a read OF this cell ORDERS the walk — every computed kind, a
// `recurrent` included.
//
// This is R80 item 4: only self-history and `lookback` reads are dropped from
// ordering, and neither is an `inputs` edge to begin with — self-history rides
// `depth`, and a `lookback` key is required to BE an `inputs` entry, so its ring
// read rides an edge that already exists. Everything left orders, which is what
// makes a reader construct AFTER the recurrent it reads — everywhere except a
// node the deadlock RELEASE lets through, and that exception is wider than a
// cycle (see `released`, FND-40).
//
// The derived/deferred allocation placeholder is unobservable either way: a
// release requires every unmet predecessor to be a RECURRENT, so no released
// node ever reads an unconstructed `derived` or `deferred`.
fn orders(role: &CellRole) -> bool {
    is_node(role)
}

// Whether a read OF this cell can close a CYCLE — a `derived` or a `deferred`,
// never a `recurrent`.
//
// §13.11.3: *"reads OF a recurrent cell … always return the previous-committed
// value. This is the rule that breaks reactive cycles"*, and §13.11.4 makes a
// recurrent on a cycle "a one-*advance* delay element". §13.2.6's init-time
// cycle rule names the other category in the same breath — *"This is distinct
// from runtime cycles (§13.11), which the per-commit DAG handles via
// recurrents-as-delays"* — so a loop a recurrent delays is not the loop that
// section rejects.
//
// So a recurrent read orders where it can and is DROPPED where it cannot: the
// ordering pass releases a node whose only unmet predecessors are recurrent
// reads, which is the same previous-committed reading applied to the schedule.
// What that node reads is the recurrent's depth-1 FALLBACK, which is what the
// engine seeds every recurrent's slot with (§4.9's "before the ring has advanced
// that far"). A `deferred` is not a cycle device (§13.11.4), so a loop closed
// through one is still a `derived_cycle`.
fn closes_a_cycle(role: &CellRole) -> bool {
    matches!(role, CellRole::Derived { .. } | CellRole::Deferred { .. })
}

// One Kahn pass's product, kept whole so the caller can ask whether it
// terminated early and, if it did, report the cycle from the leftovers.
struct Sorted {
    // The nodes in the order they became ready — topologically, ties broken by
    // rank (rank-least first among the simultaneously-ready).
    order: Vec<CellIndex>,
    // Each cell's UNSATISFIED in-degree when the pass stopped. Every entry is
    // zero on a complete pass; a positive one names a node inside or below a
    // cycle.
    in_degree: Vec<usize>,
    // How many nodes a complete pass would have emitted.
    node_count: usize,
}

// Kahn's algorithm over the CYCLE graph with a RANK-ORDERED ready set — check
// 9a, and the only pass that can fail. R64 rules the CARRIER — entry rank is
// what carries declaration order into the IR; the ready-set completion, how that
// rank composes with topology into §13.10.3's order, is FND-25 and is open.
fn kahn(resolved: &[ResolvedCell]) -> Sorted {
    let mut node_count = 0;
    let mut in_degree = vec![0usize; resolved.len()];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); resolved.len()];

    for (rank, cell) in resolved.iter().enumerate() {
        if !is_node(&cell.role) {
            continue;
        }

        node_count += 1;

        for input in cell.role.inputs() {
            if closes_a_cycle(&resolved[input.0].role) {
                in_degree[rank] += 1;
                dependents[input.0].push(rank);
            }
        }
    }

    let mut ready = BinaryHeap::new();

    for (rank, cell) in resolved.iter().enumerate() {
        if is_node(&cell.role) && in_degree[rank] == 0 {
            ready.push(Reverse(rank));
        }
    }

    let mut order = Vec::with_capacity(node_count);

    while let Some(Reverse(rank)) = ready.pop() {
        order.push(CellIndex(rank));

        for &dependent in &dependents[rank] {
            in_degree[dependent] -= 1;

            if in_degree[dependent] == 0 {
                ready.push(Reverse(dependent));
            }
        }
    }

    Sorted {
        order,
        in_degree,
        node_count,
    }
}

// Check 9a — cycle detection, over the edge set §13.11.4 makes cyclic: a read of
// a `derived` or a `deferred`. Leftovers are reported with one concrete witness.
fn acyclic_reads(resolved: &[ResolvedCell]) -> Result<(), Diagnostic> {
    let sorted = kahn(resolved);

    if sorted.order.len() < sorted.node_count {
        return Err(derived_cycle(resolved, &sorted.in_degree));
    }

    Ok(())
}

// Check 9b — THE evaluation order: §13.14.1 step 3's construction walk and
// §13.10.2 step 2's per-commit walk are ONE vector (R80 item 4).
//
// The nodes are every computed cell and the edges are every `inputs` read of
// one, recurrent reads included — so a reader always follows what it reads, in
// construction and at commit alike, and no separate construction order exists to
// disagree with this one.
//
// The one place the two edge sets part is a CYCLE. §13.11.3 breaks reactive
// cycles by reading a recurrent at its previous-committed value, so a recurrent
// read cannot be an ordering constraint when honouring it would deadlock: when
// the ready set empties with nodes left, the rank-least node whose remaining
// predecessors are all recurrent reads is released, and those reads take their
// previous-committed reading — which before any advance is the recurrent's
// depth-1 FALLBACK, the value the engine seeds its slot with. `acyclic_reads`
// has already proved such a node exists — a node with no unemitted CYCLE
// predecessor — so the walk always completes.
//
// WHICH nodes are released is decided by entry rank, and that is an OPEN
// divergence rather than a settled law (FND-40). The release is NOT confined to
// the cycle: `released` admits any waiting node whose unmet predecessors are all
// recurrent reads, which includes a plain reader downstream of an in-cycle
// recurrent, and one cycle can force several releases in turn. So the same graph
// written in two declaration orders can settle at two construction snapshots,
// and the cell that diverges need never have touched a cycle. Reported at the
// wave-2 finish; no corpus sentence answers it.
fn static_order(resolved: &[ResolvedCell]) -> Vec<CellIndex> {
    let count = resolved.len();
    let mut node_count = 0;
    let mut total: Vec<usize> = vec![0; count];
    let mut cyclic: Vec<usize> = vec![0; count];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); count];

    for (rank, cell) in resolved.iter().enumerate() {
        if !is_node(&cell.role) {
            continue;
        }

        node_count += 1;

        for input in cell.role.inputs() {
            let source = &resolved[input.0].role;

            if !orders(source) {
                continue;
            }

            total[rank] += 1;

            if closes_a_cycle(source) {
                cyclic[rank] += 1;
            }

            dependents[input.0].push(rank);
        }
    }

    let mut queued: Vec<bool> = vec![false; count];
    let mut ready = BinaryHeap::new();

    for (rank, cell) in resolved.iter().enumerate() {
        if is_node(&cell.role) && total[rank] == 0 {
            queued[rank] = true;

            ready.push(Reverse(rank));
        }
    }

    let mut order = Vec::with_capacity(node_count);

    while order.len() < node_count {
        let Some(Reverse(rank)) = ready.pop() else {
            let released = released(resolved, &cyclic, &queued);

            queued[released] = true;

            ready.push(Reverse(released));

            continue;
        };

        order.push(CellIndex(rank));

        for &dependent in &dependents[rank] {
            total[dependent] -= 1;

            if closes_a_cycle(&resolved[rank].role) {
                cyclic[dependent] -= 1;
            }

            if total[dependent] == 0 && !queued[dependent] {
                queued[dependent] = true;

                ready.push(Reverse(dependent));
            }
        }
    }

    order
}

// The deadlock release: the rank-least node still waiting whose every unmet
// predecessor is a RECURRENT read. Total on a graph `acyclic_reads` admitted —
// the cycle graph restricted to the unemitted nodes is acyclic and non-empty, so
// one of them has no unemitted cycle predecessor.
//
// This is NOT "a member of the cycle". `cyclic[rank]` counts unemitted
// derived/deferred predecessors and says nothing about whether `rank` lies on a
// loop, so a node hanging off the cycle — a plain reader of an in-cycle
// recurrent, no loop of its own — is an eligible candidate and wins whenever its
// entry rank is the lower one. That is FND-40's open half, and it is why this
// function is named for the condition it tests rather than for cycles.
fn released(resolved: &[ResolvedCell], cyclic: &[usize], queued: &[bool]) -> usize {
    (0..resolved.len())
        .find(|&rank| is_node(&resolved[rank].role) && !queued[rank] && cyclic[rank] == 0)
        .expect("a cycle-free node is always available on an acyclic read graph")
}

// The cycle report (R21: a load error, reported with the cycle's member
// paths). ONE concrete cycle is extracted: walk the cycle graph's edges
// among the leftover (never-ready) nodes from the rank-least leftover until a
// node repeats; the repeat closes the cycle.
fn derived_cycle(resolved: &[ResolvedCell], in_degree: &[usize]) -> Diagnostic {
    // A leftover exists whenever this is called; the 0 default cannot fire.
    let start = in_degree.iter().position(|&degree| degree > 0).unwrap_or(0);
    let mut position: HashMap<usize, usize> = HashMap::new();
    let mut walk: Vec<usize> = Vec::new();
    let mut current = start;

    loop {
        if let Some(&at) = position.get(&current) {
            return cycle_diagnostic(resolved, &walk[at..]);
        }

        position.insert(current, walk.len());
        walk.push(current);
        current = next_leftover_input(resolved, in_degree, current);
    }
}

// The walk's step: the FIRST (list-order) input that is itself a leftover AND a
// cycle-edge source under the construction walk — a `derived` or a `deferred`.
// Total on leftover nodes: a leftover's final in-degree counts exactly its
// leftover source inputs, so one always exists. The self-step fallback fires
// only on a broken invariant, and then terminates the walk as a self-cycle
// rather than hanging or panicking inside an error path.
//
// The source restriction is what keeps a RECURRENT out of the reported cycle. A
// recurrent can be a leftover — it may read a cycle it is not part of — and
// stepping into one would name it as a member of a ring it merely depends on,
// which is the same mistake as naming a dependent. A `deferred` is a source and
// IS named, because it really is a member: §13.11.4 makes the recurrent the one
// cycle device, so a loop through a deferred is a loop.
fn next_leftover_input(resolved: &[ResolvedCell], in_degree: &[usize], current: usize) -> usize {
    for input in resolved[current].role.inputs() {
        let source = closes_a_cycle(&resolved[input.0].role);

        if source && in_degree[input.0] > 0 {
            return input.0;
        }
    }

    current
}

// The rendered report: members in WALK order, rotated to start from the
// cycle's rank-least member, that member repeated at the end to close the
// ring — "derived cycle: App.a -> App.b -> App.a". The diagnostic points at
// the rank-least member: its path, its span.
fn cycle_diagnostic(resolved: &[ResolvedCell], cycle: &[usize]) -> Diagnostic {
    let mut least = 0;

    for (offset, &rank) in cycle.iter().enumerate() {
        if rank < cycle[least] {
            least = offset;
        }
    }

    let rotated: Vec<usize> = cycle[least..]
        .iter()
        .chain(cycle[..least].iter())
        .copied()
        .collect();
    let mut message = String::from("derived cycle: ");

    for &rank in &rotated {
        resolved[rank].path.render_into(&mut message);
        message.push_str(" -> ");
    }

    resolved[rotated[0]].path.render_into(&mut message);

    Diagnostic {
        class: DiagClass::DerivedCycle,
        span: Some(resolved[rotated[0]].span),
        path: Some(resolved[rotated[0]].path.clone()),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    use ductus_ir::graph::{
        Effect, FallbackEntry, Gate, LookbackEntry, Member, MemberDriver, ReopenPair, Stream,
        StreamPolicy, Wire, WireTo,
    };
    use ductus_ir::module::{BehaviorsSection, GraphSection, TypesSection};
    use ductus_ir::types::Prim;

    use crate::provider::Behavior;

    // The resolvability seam's test double (D5): a plain map, resolving what
    // was registered and refusing what was not — the harness registry's shape,
    // reduced to what these tests need.
    struct StubProvider {
        behaviors: HashMap<u32, Behavior>,
    }

    impl BehaviorProvider for StubProvider {
        fn resolve(&self, handle: u32) -> Option<Behavior> {
            self.behaviors.get(&handle).cloned()
        }
    }

    fn stubs(handles: &[u32]) -> StubProvider {
        let mut behaviors: HashMap<u32, Behavior> = HashMap::new();

        for &handle in handles {
            behaviors.insert(handle, Rc::new(|_| Value::I64(0)));
        }

        StubProvider { behaviors }
    }

    fn name(text: &str) -> Segment {
        Segment::Name {
            text: text.to_string(),
            ordinal: None,
        }
    }

    fn path(names: &[&str]) -> CellPath {
        CellPath::new(names.iter().map(|text| name(text)).collect())
    }

    // A blank cell of the kind: every clause absent. Tests fill exactly what
    // they exercise, so each trigger reads as the one thing it triggers.
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

    // A `recurrent` with its handle, inputs, self-history depth and the
    // uniform `init` fallback; the per-depth and per-input clauses are filled
    // by the tests that exercise them.
    fn recurrent(names: &[&str], prim: Prim, uses: u32, inputs: &[&[&str]], depth: u64) -> Cell {
        let mut built = cell(CellKind::Recurrent, names, prim);
        built.uses = Some(uses);
        built.inputs = inputs.iter().map(|names| path(names)).collect();
        built.depth = Some(depth);
        built.init = Some(ValueLiteral::int("0".to_string()));

        built
    }

    // A `deferred` with its handle, inputs and latency depth — the three
    // clauses 033-245 renders, and nothing else.
    fn deferred(names: &[&str], prim: Prim, uses: u32, inputs: &[&[&str]], depth: u64) -> Cell {
        let mut built = cell(CellKind::Deferred, names, prim);
        built.uses = Some(uses);
        built.inputs = inputs.iter().map(|names| path(names)).collect();
        built.depth = Some(depth);

        built
    }

    // The latency half of a loaded deferred, for the assertions below.
    fn latency_of(graph: &LoadedGraph, names: &[&str]) -> (usize, Vec<CellIndex>) {
        let index = graph.index[&path(names)];

        match &graph.cells[index.0].role {
            CellRole::Deferred { depth, inputs, .. } => (*depth, inputs.clone()),
            CellRole::Input { .. } | CellRole::Derived { .. } | CellRole::Recurrent { .. } => {
                panic!("`{}` is a deferred cell", path(names).render())
            }
        }
    }

    // The history half of a loaded recurrent, for the assertions below.
    fn history_of(graph: &LoadedGraph, names: &[&str]) -> (usize, Vec<LookbackLink>, Vec<Value>) {
        let index = graph.index[&path(names)];

        match &graph.cells[index.0].role {
            CellRole::Recurrent {
                depth,
                lookback,
                fallbacks,
                ..
            } => (*depth, lookback.clone(), fallbacks.clone()),
            CellRole::Input { .. } | CellRole::Derived { .. } | CellRole::Deferred { .. } => {
                panic!("`{}` is a recurrent cell", path(names).render())
            }
        }
    }

    // A `gate` entry with its predicate handle, the cells the predicate reads
    // and the constructs it guards. `gate_parent` is filled by the nesting
    // tests, which are the only ones it means anything to.
    fn gate(names: &[&str], pred: u32, inputs: &[&[&str]], guards: &[&[&str]]) -> Gate {
        Gate {
            path: path(names),
            span: Span::new(0, 0),
            pred,
            inputs: inputs.iter().map(|names| path(names)).collect(),
            guards: guards.iter().map(|names| path(names)).collect(),
            gate_parent: None,
        }
    }

    fn scope(names: &[&str], cells: Vec<Cell>) -> Scope {
        entries(names, cells.into_iter().map(Entry::Cell).collect())
    }

    // A scope holding entries of ANY kind in body order — what the rank tests
    // need, since a gate's entry rank is only interesting when a cell stands
    // beside it.
    fn entries(names: &[&str], entries: Vec<Entry>) -> Scope {
        Scope {
            path: path(names),
            span: Span::new(0, 0),
            exposes: Vec::new(),
            effects: Vec::new(),
            reset_on_reopen: Vec::new(),
            entries,
        }
    }

    // A cell's own gate after the loader folded both binding spellings.
    fn gate_of(graph: &LoadedGraph, names: &[&str]) -> Option<GateIndex> {
        graph.cells[graph.index[&path(names)].0].gate
    }

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

    fn loaded(module: &Module, provider: &dyn BehaviorProvider) -> LoadedGraph {
        load(module, provider).expect("the module loads")
    }

    fn failure(module: &Module, provider: &dyn BehaviorProvider) -> Diagnostic {
        load(module, provider).expect_err("the module fails to load")
    }

    fn rendered_path(diagnostic: &Diagnostic) -> Option<String> {
        diagnostic.path.as_ref().map(CellPath::render)
    }

    fn init_of(graph: &LoadedGraph, names: &[&str]) -> Value {
        let index = graph.index[&path(names)];

        match &graph.cells[index.0].role {
            CellRole::Input { init } => *init,
            CellRole::Derived { .. } | CellRole::Recurrent { .. } | CellRole::Deferred { .. } => {
                panic!("`{}` is an input cell", path(names).render())
            }
        }
    }

    // The happy path, whole: two scopes, ranks scope-major in file order, the
    // index total over every path, and the static order both topological and
    // rank-tiebroken — the diamond's b and c depend on the same a, b is
    // declared first, so b evaluates first (R64).
    #[test]
    fn loads_inputs_and_deriveds_across_two_scopes() {
        let graph = loaded(
            &module(vec![
                scope(
                    &["App"],
                    vec![
                        input(&["App", "x"], Prim::I64, ValueLiteral::int("7".to_string())),
                        derived(&["App", "a"], Prim::I64, 1, &[&["App", "x"]]),
                    ],
                ),
                scope(
                    &["App", "panel"],
                    vec![
                        derived(&["App", "panel", "b"], Prim::I64, 1, &[&["App", "a"]]),
                        derived(&["App", "panel", "c"], Prim::I64, 1, &[&["App", "a"]]),
                        derived(
                            &["App", "panel", "d"],
                            Prim::I64,
                            1,
                            &[&["App", "panel", "b"], &["App", "panel", "c"]],
                        ),
                    ],
                ),
            ]),
            &stubs(&[1]),
        );

        let ranks: Vec<String> = graph.cells.iter().map(|cell| cell.path.render()).collect();

        assert_eq!(
            ranks,
            [
                "App.x",
                "App.a",
                "App.panel.b",
                "App.panel.c",
                "App.panel.d"
            ]
        );

        for (rank, cell) in graph.cells.iter().enumerate() {
            assert_eq!(graph.index.get(&cell.path), Some(&CellIndex(rank)));
        }

        assert_eq!(graph.index.len(), graph.cells.len());
        // Topological AND rank-tiebroken: a before both b and c, b before c by
        // declaration order, d last.
        assert_eq!(
            graph.static_order,
            [CellIndex(1), CellIndex(2), CellIndex(3), CellIndex(4)]
        );
    }

    // Forward references are legal: the graph section is a serialization, so a
    // derived at rank 0 may read an input declared after it.
    #[test]
    fn a_forward_reference_loads() {
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![
                    derived(&["App", "a"], Prim::I32, 1, &[&["App", "x"]]),
                    input(&["App", "x"], Prim::I32, ValueLiteral::int("0".to_string())),
                ],
            )]),
            &stubs(&[1]),
        );

        let CellRole::Derived { inputs, .. } = &graph.cells[0].role else {
            panic!("rank 0 is the derived");
        };

        assert_eq!(inputs, &[CellIndex(1)]);
        assert_eq!(graph.static_order, [CellIndex(0)]);
    }

    // A constant derived — `inputs` empty — is legal and takes its place in
    // the static order, where it will evaluate once at startup.
    #[test]
    fn a_constant_derived_loads() {
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![derived(&["App", "answer"], Prim::I64, 1, &[])],
            )]),
            &stubs(&[1]),
        );

        let CellRole::Derived { inputs, .. } = &graph.cells[0].role else {
            panic!("rank 0 is the derived");
        };

        assert!(inputs.is_empty());
        assert_eq!(graph.static_order, [CellIndex(0)]);
    }

    // The conversion table's accepting rows (R57): each literal family lands
    // as a value of the declared type, bit-checked through `bits_eq`.
    #[test]
    fn converts_each_init_family_under_its_own_type() {
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![
                    input(
                        &["App", "a"],
                        Prim::I64,
                        ValueLiteral::int("-7".to_string()),
                    ),
                    input(
                        &["App", "b"],
                        Prim::I32,
                        ValueLiteral::int("42".to_string()),
                    ),
                    input(
                        &["App", "c"],
                        Prim::F64,
                        ValueLiteral::float("1.5".to_string()),
                    ),
                    input(
                        &["App", "d"],
                        Prim::F32,
                        ValueLiteral::float("0.5".to_string()),
                    ),
                    input(&["App", "e"], Prim::Bool, ValueLiteral::True),
                    input(&["App", "f"], Prim::Bool, ValueLiteral::False),
                ],
            )]),
            &stubs(&[]),
        );

        assert!(init_of(&graph, &["App", "a"]).bits_eq(&Value::I64(-7)));
        assert!(init_of(&graph, &["App", "b"]).bits_eq(&Value::I32(42)));
        assert!(init_of(&graph, &["App", "c"]).bits_eq(&Value::F64(1.5)));
        assert!(init_of(&graph, &["App", "d"]).bits_eq(&Value::F32(0.5)));
        assert!(init_of(&graph, &["App", "e"]).bits_eq(&Value::Bool(true)));
        assert!(init_of(&graph, &["App", "f"]).bits_eq(&Value::Bool(false)));
    }

    // The three float words, type-gated (033-248): each converts at the
    // DECLARED width's own constants, under both widths.
    #[test]
    fn converts_the_float_words_at_the_declared_width() {
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![
                    input(&["App", "a"], Prim::F64, ValueLiteral::Nan),
                    input(&["App", "b"], Prim::F32, ValueLiteral::Nan),
                    input(&["App", "c"], Prim::F64, ValueLiteral::Inf),
                    input(&["App", "d"], Prim::F32, ValueLiteral::Inf),
                    input(&["App", "e"], Prim::F64, ValueLiteral::NegInf),
                    input(&["App", "f"], Prim::F32, ValueLiteral::NegInf),
                ],
            )]),
            &stubs(&[]),
        );

        assert!(init_of(&graph, &["App", "a"]).bits_eq(&Value::F64(f64::NAN)));
        assert!(init_of(&graph, &["App", "b"]).bits_eq(&Value::F32(f32::NAN)));
        assert!(init_of(&graph, &["App", "c"]).bits_eq(&Value::F64(f64::INFINITY)));
        assert!(init_of(&graph, &["App", "d"]).bits_eq(&Value::F32(f32::INFINITY)));
        assert!(init_of(&graph, &["App", "e"]).bits_eq(&Value::F64(f64::NEG_INFINITY)));
        assert!(init_of(&graph, &["App", "f"]).bits_eq(&Value::F32(f32::NEG_INFINITY)));
    }

    // A scope holding one non-cell entry — everything the entry needs to be
    // grammatical, nothing more.
    fn scoped(entry: Entry) -> Module {
        let mut shell = scope(&["App"], Vec::new());
        shell.entries.push(entry);

        module(vec![shell])
    }

    // Check 4: the slice holds `cell` and `gate` entries. The three still out
    // reject at their own paths — and two of them, `wire` and `stream`, are out
    // because the corpus does not define them, not because a milestone
    // boundary runs here (M1.md §2.3).
    #[test]
    fn rejects_every_out_of_slice_entry_kind() {
        let entries = [
            Entry::Wire(Wire {
                path: path(&["App", "w"]),
                span: Span::new(0, 0),
                from: path(&["App", "x"]),
                to: WireTo::Null,
                tag: None,
                attrs: Vec::new(),
                gate: None,
            }),
            Entry::Effect(Effect {
                path: path(&["App", "e"]),
                span: Span::new(0, 0),
                reconciler: "dom".to_string(),
                params: Vec::new(),
                desired: Vec::new(),
                observed: Vec::new(),
                gate: None,
            }),
            Entry::Stream(Stream {
                path: path(&["App", "s"]),
                span: Span::new(0, 0),
                tag: TypeTag::Prim(Prim::I64),
                policy: StreamPolicy::Ring,
                capacity: 8,
                source_deps: Vec::new(),
                observes: Vec::new(),
                reset_on_reload: false,
                reset_on_reopen: false,
                history: None,
                lookback: Vec::new(),
            }),
        ];

        for (entry, expected) in entries.into_iter().zip(["App.w", "App.e", "App.s"]) {
            let diagnostic = failure(&scoped(entry), &stubs(&[1]));

            assert_eq!(diagnostic.class.id(), "outside_m0_slice");
            assert_eq!(rendered_path(&diagnostic).as_deref(), Some(expected));
        }
    }

    // Check 5b: `fold` parses — the grammar admits every kind — and the
    // implemented slice does not. `recurrent` left this list when cut B's first
    // wave landed, `deferred` when its second did (R70), so `fold` is the last
    // kind standing.
    #[test]
    fn rejects_the_one_out_of_slice_cell_kind() {
        let diagnostic = failure(
            &module(vec![scope(
                &["App"],
                vec![cell(CellKind::Fold, &["App", "x"], Prim::I64)],
            )]),
            &stubs(&[]),
        );

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));
    }

    // Check 5c, both halves: a primitive outside the word tier (`str`, `u8`),
    // and a tag that is no primitive at all.
    #[test]
    fn rejects_a_cell_type_outside_the_word_tier() {
        for prim in [Prim::Str, Prim::U8, Prim::I128] {
            let diagnostic = failure(
                &module(vec![scope(
                    &["App"],
                    vec![input(
                        &["App", "x"],
                        prim,
                        ValueLiteral::int("0".to_string()),
                    )],
                )]),
                &stubs(&[]),
            );

            assert_eq!(
                diagnostic.class.id(),
                "outside_m0_slice",
                "{}",
                prim.as_str()
            );
        }

        let mut aggregate = input(&["App", "x"], Prim::I64, ValueLiteral::int("0".to_string()));
        aggregate.tag = TypeTag::TypeId("Point".to_string());
        let diagnostic = failure(&module(vec![scope(&["App"], vec![aggregate])]), &stubs(&[]));

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));
    }

    // Check 1: M0 scope paths are plain dotted names — a keyed segment and an
    // ordinal suffix both parse (D7) and both reject, blaming the SCOPE path.
    #[test]
    fn rejects_a_keyed_or_ordinal_scope_path() {
        let keyed = Scope {
            path: CellPath::new(vec![name("App"), Segment::Key("42".to_string())]),
            span: Span::new(0, 0),
            exposes: Vec::new(),
            effects: Vec::new(),
            reset_on_reopen: Vec::new(),
            entries: Vec::new(),
        };
        let diagnostic = failure(&module(vec![keyed]), &stubs(&[]));

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.'42'"));

        let ordinal = Scope {
            path: CellPath::new(vec![
                name("App"),
                Segment::Name {
                    text: "print".to_string(),
                    ordinal: Some(0),
                },
            ]),
            span: Span::new(0, 0),
            exposes: Vec::new(),
            effects: Vec::new(),
            reset_on_reopen: Vec::new(),
            entries: Vec::new(),
        };
        let diagnostic = failure(&module(vec![ordinal]), &stubs(&[]));

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.print:0"));
    }

    // Check 5a: the same path law on a cell.
    #[test]
    fn rejects_a_keyed_or_ordinal_cell_path() {
        let mut keyed = input(&["App", "x"], Prim::I64, ValueLiteral::int("0".to_string()));
        keyed.path = CellPath::new(vec![
            name("App"),
            Segment::Key("row".to_string()),
            name("x"),
        ]);
        let diagnostic = failure(&module(vec![scope(&["App"], vec![keyed])]), &stubs(&[]));

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.row.x"));

        let mut ordinal = input(&["App", "x"], Prim::I64, ValueLiteral::int("0".to_string()));
        ordinal.path = CellPath::new(vec![
            name("App"),
            Segment::Name {
                text: "x".to_string(),
                ordinal: Some(1),
            },
        ]);
        let diagnostic = failure(&module(vec![scope(&["App"], vec![ordinal])]), &stubs(&[]));

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x:1"));
    }

    // One clause-filling edit per forbidden clause — the shape of the three
    // clause-law tests below.
    type ScopeEdit = fn(&mut Scope);
    type CellEdit = fn(&mut Cell);

    // Check 2: a scope carrying placement (`exposes`), effect topology
    // (`effects`) or stream-cursor machinery (`reset_on_reopen`) is outside
    // the slice, blamed at the scope.
    #[test]
    fn rejects_a_scope_with_exposes_effects_or_reopen_pairs() {
        let mutations: [(&str, ScopeEdit); 3] = [
            ("exposes", |scope| scope.exposes.push(path(&["App", "x"]))),
            ("effects", |scope| scope.effects.push(path(&["App", "e"]))),
            ("reset_on_reopen", |scope| {
                scope.reset_on_reopen.push(ReopenPair {
                    consumer: path(&["App", "c"]),
                    stream: path(&["App", "s"]),
                });
            }),
        ];

        for (clause, mutate) in mutations {
            let mut shell = scope(&["App"], Vec::new());
            mutate(&mut shell);
            let diagnostic = failure(&module(vec![shell]), &stubs(&[]));

            assert_eq!(diagnostic.class.id(), "outside_m0_slice", "{clause}");
            assert_eq!(
                rendered_path(&diagnostic).as_deref(),
                Some("App"),
                "{clause}"
            );
        }
    }

    // Check 5d, the `input` row: every clause but `init` and `gate` is
    // forbidden, one trigger per clause.
    #[test]
    fn rejects_every_forbidden_clause_on_an_input() {
        let mutations: [(&str, CellEdit); 9] = [
            ("uses", |cell| cell.uses = Some(1)),
            ("inputs", |cell| cell.inputs.push(path(&["App", "y"]))),
            ("depth", |cell| cell.depth = Some(1)),
            ("lookback", |cell| {
                cell.lookback.push(LookbackEntry {
                    cell: path(&["App", "y"]),
                    k: 1,
                });
            }),
            ("reset_on_reopen", |cell| cell.reset_on_reopen = true),
            ("fallbacks", |cell| {
                cell.fallbacks.push(FallbackEntry {
                    k: 1,
                    value: ValueLiteral::int("0".to_string()),
                });
            }),
            ("combiner", |cell| cell.combiner = Some(2)),
            ("else", |cell| {
                cell.else_value = Some(ValueLiteral::Int("0".to_string()))
            }),
            ("members", |cell| {
                cell.members.push(Member {
                    path: path(&["App", "m"]),
                    driver: MemberDriver::Permanent,
                });
            }),
        ];

        for (clause, mutate) in mutations {
            let mut built = input(&["App", "x"], Prim::I64, ValueLiteral::int("0".to_string()));
            mutate(&mut built);
            let diagnostic = failure(&module(vec![scope(&["App"], vec![built])]), &stubs(&[1]));

            assert_eq!(diagnostic.class.id(), "outside_m0_slice", "{clause}");
            assert_eq!(
                rendered_path(&diagnostic).as_deref(),
                Some("App.x"),
                "{clause}"
            );
        }
    }

    // Check 5d, the `derived` row: everything but `uses`, `inputs` and `gate`
    // is forbidden.
    #[test]
    fn rejects_every_forbidden_clause_on_a_derived() {
        let mutations: [(&str, CellEdit); 8] = [
            ("init", |cell| {
                cell.init = Some(ValueLiteral::Int("0".to_string()))
            }),
            ("depth", |cell| cell.depth = Some(1)),
            ("lookback", |cell| {
                cell.lookback.push(LookbackEntry {
                    cell: path(&["App", "y"]),
                    k: 1,
                });
            }),
            ("reset_on_reopen", |cell| cell.reset_on_reopen = true),
            ("fallbacks", |cell| {
                cell.fallbacks.push(FallbackEntry {
                    k: 1,
                    value: ValueLiteral::int("0".to_string()),
                });
            }),
            ("combiner", |cell| cell.combiner = Some(2)),
            ("else", |cell| {
                cell.else_value = Some(ValueLiteral::Int("0".to_string()))
            }),
            ("members", |cell| {
                cell.members.push(Member {
                    path: path(&["App", "m"]),
                    driver: MemberDriver::GateGuarded,
                });
            }),
        ];

        for (clause, mutate) in mutations {
            let mut built = derived(&["App", "d"], Prim::I64, 1, &[]);
            mutate(&mut built);
            let diagnostic = failure(&module(vec![scope(&["App"], vec![built])]), &stubs(&[1]));

            assert_eq!(diagnostic.class.id(), "outside_m0_slice", "{clause}");
            assert_eq!(
                rendered_path(&diagnostic).as_deref(),
                Some("App.d"),
                "{clause}"
            );
        }
    }

    // Check 5d's required halves: M0.md §2's rows are "input (with init)" and
    // "derived (with uses/inputs)" — absence is outside the slice, not a
    // default.
    #[test]
    fn rejects_an_input_without_init_and_a_derived_without_uses() {
        let bare_input = cell(CellKind::Input, &["App", "x"], Prim::I64);
        let diagnostic = failure(
            &module(vec![scope(&["App"], vec![bare_input])]),
            &stubs(&[]),
        );

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));

        let bare_derived = cell(CellKind::Derived, &["App", "d"], Prim::I64);
        let diagnostic = failure(
            &module(vec![scope(&["App"], vec![bare_derived])]),
            &stubs(&[]),
        );

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.d"));
    }

    // Check 5g: identity is by path across EVERY scope, so two scopes may not
    // both declare `App.panel.x` — and the SECOND declaration is the one
    // blamed, pinned by its span.
    //
    // The shared path is DEEP on purpose: containment (check 5e) runs first, so
    // a cross-scope duplicate must be a path both scopes may legally hold —
    // `App` reaches `App.panel.x` because depth is free, and `App.panel`
    // reaches it because it is one segment down.
    #[test]
    fn rejects_a_duplicate_path_across_scopes_blaming_the_second() {
        let first = input(
            &["App", "panel", "x"],
            Prim::I64,
            ValueLiteral::int("0".to_string()),
        );
        let mut second = input(
            &["App", "panel", "x"],
            Prim::I64,
            ValueLiteral::int("1".to_string()),
        );
        second.span = Span::new(90, 30);

        let diagnostic = failure(
            &module(vec![
                scope(&["App"], vec![first]),
                scope(&["App", "panel"], vec![second]),
            ]),
            &stubs(&[]),
        );

        assert_eq!(diagnostic.class.id(), "duplicate_cell_path");
        assert_eq!(diagnostic.span, Some(Span::new(90, 30)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.panel.x"));
    }

    // Check 3c: the same identity law one level up — two scope entries may not
    // declare one path. The SECOND is blamed, pinned by its span.
    #[test]
    fn rejects_a_duplicate_scope_path_blaming_the_second() {
        let first = scope(&["App", "panel"], Vec::new());
        let mut second = scope(&["App", "panel"], Vec::new());
        second.span = Span::new(120, 40);

        let diagnostic = failure(&module(vec![first, second]), &stubs(&[]));

        assert_eq!(diagnostic.class.id(), "duplicate_scope_path");
        assert_eq!(diagnostic.span, Some(Span::new(120, 40)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.panel"));
    }

    // Check 3b — the collision, scope AFTER cell: the first scope declares
    // `App.panel` as a CELL, so the scope entry that follows may not take the
    // same path. One namespace, and the scope is the second claim.
    #[test]
    fn rejects_a_scope_path_an_earlier_cell_claimed() {
        let mut second = scope(&["App", "panel"], Vec::new());
        second.span = Span::new(60, 25);

        let diagnostic = failure(
            &module(vec![
                scope(
                    &["App"],
                    vec![input(
                        &["App", "panel"],
                        Prim::I64,
                        ValueLiteral::int("0".to_string()),
                    )],
                ),
                second,
            ]),
            &stubs(&[]),
        );

        assert_eq!(diagnostic.class.id(), "scope_cell_collision");
        assert_eq!(diagnostic.span, Some(Span::new(60, 25)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.panel"));
    }

    // Check 5f — the same collision the other way round: the SCOPE comes first,
    // so the cell is the second claim and the cell is blamed. File order alone
    // decides which of the two is second.
    #[test]
    fn rejects_a_cell_path_an_earlier_scope_claimed() {
        let mut colliding = input(
            &["App", "panel"],
            Prim::I64,
            ValueLiteral::int("0".to_string()),
        );
        colliding.span = Span::new(75, 18);

        let diagnostic = failure(
            &module(vec![
                scope(&["App", "panel"], Vec::new()),
                scope(&["App"], vec![colliding]),
            ]),
            &stubs(&[]),
        );

        assert_eq!(diagnostic.class.id(), "scope_cell_collision");
        assert_eq!(diagnostic.span, Some(Span::new(75, 18)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.panel"));
    }

    // Check 5e: the braces and the dots carry ONE hierarchy, so a cell written
    // inside `scope App.panel` whose path goes somewhere else is rejected at
    // the cell.
    #[test]
    fn rejects_a_cell_outside_its_scope() {
        let mut stray = input(
            &["App", "other"],
            Prim::I64,
            ValueLiteral::int("0".to_string()),
        );
        stray.span = Span::new(50, 12);

        let diagnostic = failure(
            &module(vec![scope(&["App", "panel"], vec![stray])]),
            &stubs(&[]),
        );

        assert_eq!(diagnostic.class.id(), "cell_outside_scope");
        assert_eq!(diagnostic.span, Some(Span::new(50, 12)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.other"));
    }

    // Check 5e's PROPER half: a cell path equal to its scope's is not
    // contained — the prefix must be strictly shorter — so a scope and a cell
    // are never the same place. This is also why check 5f can never fire for a
    // cell's own scope: containment has already rejected it.
    #[test]
    fn rejects_a_cell_whose_path_is_its_scopes_path() {
        let twin = input(
            &["App", "panel"],
            Prim::I64,
            ValueLiteral::int("0".to_string()),
        );

        let diagnostic = failure(
            &module(vec![scope(&["App", "panel"], vec![twin])]),
            &stubs(&[]),
        );

        assert_eq!(diagnostic.class.id(), "cell_outside_scope");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.panel"));
    }

    // Depth is FREE: only the prefix is law, so a cell may sit two or more
    // segments below its scope with nothing declared in between — no
    // `App.print` scope, no `App.a` scope, and both cells load.
    #[test]
    fn loads_a_cell_declared_below_its_scope_with_nothing_in_between() {
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![
                    input(
                        &["App", "print", "text"],
                        Prim::I64,
                        ValueLiteral::int("0".to_string()),
                    ),
                    derived(
                        &["App", "a", "b", "c"],
                        Prim::I64,
                        1,
                        &[&["App", "print", "text"]],
                    ),
                ],
            )]),
            &stubs(&[1]),
        );

        let ranks: Vec<String> = graph.cells.iter().map(|cell| cell.path.render()).collect();

        assert_eq!(ranks, ["App.print.text", "App.a.b.c"]);
        assert_eq!(graph.static_order, [CellIndex(1)]);
    }

    // Check 3a: every path is anchored at the module root (§15.4.1.1), so a
    // scope of `module App` starts at the segment `App` — a foreign root is
    // rejected, and so is a spelling that merely BEGINS with the module's name.
    #[test]
    fn rejects_an_unanchored_scope_path() {
        for names in [&["Foo"][..], &["Foo", "App"][..], &["Application"][..]] {
            let mut foreign = scope(names, Vec::new());
            foreign.span = Span::new(30, 9);

            let diagnostic = failure(&module(vec![foreign]), &stubs(&[]));

            assert_eq!(diagnostic.class.id(), "unanchored_scope_path", "{names:?}");
            assert_eq!(diagnostic.span, Some(Span::new(30, 9)), "{names:?}");
            assert_eq!(
                rendered_path(&diagnostic),
                Some(path(names).render()),
                "{names:?}"
            );
        }
    }

    // A cell needs no anchoring check of its own, and this is why: its scope is
    // anchored and the cell extends its scope, so an unanchored CELL is
    // unreachable — the only way to write one is under an unanchored scope,
    // which check 3a rejects first, or under an anchored one, where check 5e
    // rejects it.
    #[test]
    fn an_unanchored_cell_is_caught_by_its_scope_or_by_containment() {
        let under_foreign = failure(
            &module(vec![scope(
                &["Foo"],
                vec![input(
                    &["Foo", "x"],
                    Prim::I64,
                    ValueLiteral::int("0".to_string()),
                )],
            )]),
            &stubs(&[]),
        );

        assert_eq!(under_foreign.class.id(), "unanchored_scope_path");
        assert_eq!(rendered_path(&under_foreign).as_deref(), Some("Foo"));

        let under_anchored = failure(
            &module(vec![scope(
                &["App"],
                vec![input(
                    &["Foo", "x"],
                    Prim::I64,
                    ValueLiteral::int("0".to_string()),
                )],
            )]),
            &stubs(&[]),
        );

        assert_eq!(under_anchored.class.id(), "cell_outside_scope");
        assert_eq!(rendered_path(&under_anchored).as_deref(), Some("Foo.x"));
    }

    // Check 6: an input path no cell declares. The diagnostic carries THE
    // MISSING path — the thing the author must fix — with the reading cell's
    // span.
    #[test]
    fn rejects_an_unknown_input_path_carrying_the_missing_path() {
        let mut reader = derived(&["App", "d"], Prim::I64, 1, &[&["App", "ghost"]]);
        reader.span = Span::new(40, 20);

        let diagnostic = failure(&module(vec![scope(&["App"], vec![reader])]), &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "unknown_input_path");
        assert_eq!(diagnostic.span, Some(Span::new(40, 20)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.ghost"));
    }

    // Check 7 (D5): the module's behavior table is empty throughout M0, so the
    // provider registry is the whole answer — a handle it does not know is
    // unresolvable, and a handle it does know resolves.
    #[test]
    fn rejects_an_unresolvable_behavior_handle() {
        let build = || {
            module(vec![scope(
                &["App"],
                vec![derived(&["App", "d"], Prim::I64, 0xd1, &[])],
            )])
        };
        let diagnostic = failure(&build(), &stubs(&[0xaa]));

        assert_eq!(diagnostic.class.id(), "unresolvable_behavior");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.d"));

        // The same module under a provider that knows the handle loads.
        loaded(&build(), &stubs(&[0xd1]));
    }

    // Fail-fast determinism, same pass: two Pass A errors present, and the one
    // earlier in FILE ORDER wins — the second scope's out-of-slice kind is
    // never reached.
    #[test]
    fn the_file_order_first_error_wins() {
        let bad_init = input(&["App", "x"], Prim::I64, ValueLiteral::True);
        let bad_kind = cell(CellKind::Fold, &["App", "panel", "f"], Prim::I64);

        let diagnostic = failure(
            &module(vec![
                scope(&["App"], vec![bad_init]),
                scope(&["App", "panel"], vec![bad_kind]),
            ]),
            &stubs(&[]),
        );

        assert_eq!(diagnostic.class.id(), "type_mismatched_init");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));
    }

    // Fail-fast determinism, across passes: Pass A runs to completion before
    // Pass B begins, so a LATER cell's slice error beats an EARLIER cell's
    // unknown input.
    #[test]
    fn a_pass_a_error_wins_over_an_earlier_cells_pass_b_error() {
        let unknown_reader = derived(&["App", "d"], Prim::I64, 1, &[&["App", "ghost"]]);
        let bad_kind = cell(CellKind::Recurrent, &["App", "r"], Prim::I64);

        let diagnostic = failure(
            &module(vec![scope(&["App"], vec![unknown_reader, bad_kind])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.r"));
    }

    // Check 6 before check 7, both full rank-order loops: a LATER derived's
    // unknown input beats an EARLIER derived's unresolvable handle.
    #[test]
    fn an_unknown_input_wins_over_an_earlier_unresolvable_behavior() {
        let unresolvable = derived(&["App", "a"], Prim::I64, 0xff, &[]);
        let unknown_reader = derived(&["App", "b"], Prim::I64, 1, &[&["App", "ghost"]]);

        let diagnostic = failure(
            &module(vec![scope(&["App"], vec![unresolvable, unknown_reader])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "unknown_input_path");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.ghost"));
    }

    // One input cell whose `init` fails to convert — the shared trigger of the
    // mismatch tests.
    fn mismatch(prim: Prim, init: ValueLiteral) -> Diagnostic {
        let diagnostic = failure(
            &module(vec![scope(
                &["App"],
                vec![
                    input(&["App", "x"], Prim::I64, ValueLiteral::int("0".to_string())),
                    {
                        let mut built = cell(CellKind::Input, &["App", "bad"], prim);
                        built.init = Some(init);

                        built
                    },
                ],
            )]),
            &stubs(&[]),
        );

        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.bad"));

        diagnostic
    }

    // Check 5h, the refusing rows: STRICT by lexeme family (R57) — no family
    // converts under a type its lexeme does not name, one witness per family.
    #[test]
    fn rejects_every_cross_family_init() {
        let pairs = [
            (
                "int under f64",
                Prim::F64,
                ValueLiteral::int("1".to_string()),
            ),
            (
                "int under bool",
                Prim::Bool,
                ValueLiteral::int("1".to_string()),
            ),
            (
                "float under i32",
                Prim::I32,
                ValueLiteral::float("1.5".to_string()),
            ),
            ("true under i64", Prim::I64, ValueLiteral::True),
            ("false under f32", Prim::F32, ValueLiteral::False),
            ("nan under i32", Prim::I32, ValueLiteral::Nan),
            ("inf under i64", Prim::I64, ValueLiteral::Inf),
            ("-inf under bool", Prim::Bool, ValueLiteral::NegInf),
            (
                "str under bool",
                Prim::Bool,
                ValueLiteral::Str("true".to_string()),
            ),
            (
                "str under i64",
                Prim::I64,
                ValueLiteral::Str("7".to_string()),
            ),
        ];

        for (case, prim, literal) in pairs {
            assert_eq!(
                mismatch(prim, literal).class.id(),
                "type_mismatched_init",
                "{case}"
            );
        }
    }

    // Overflow IS a conversion failure: the digits are INT-shaped, the value
    // does not fit the declared width.
    #[test]
    fn rejects_an_integer_init_that_overflows_its_width() {
        let overflows = [
            ("i32 + 1", Prim::I32, "2147483648"),
            ("i32 - 1 past MIN", Prim::I32, "-2147483649"),
            ("i64 + 1", Prim::I64, "9223372036854775808"),
            ("i64 - 1 past MIN", Prim::I64, "-9223372036854775809"),
        ];

        for (case, prim, text) in overflows {
            let literal = ValueLiteral::int(text.to_string());

            assert_eq!(
                mismatch(prim, literal).class.id(),
                "type_mismatched_init",
                "{case}"
            );
        }
    }

    // The exact boundaries convert — MIN included, whose magnitude has no
    // positive counterpart at the same width.
    #[test]
    fn converts_the_exact_integer_boundaries() {
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![
                    input(
                        &["App", "a"],
                        Prim::I32,
                        ValueLiteral::int("-2147483648".to_string()),
                    ),
                    input(
                        &["App", "b"],
                        Prim::I32,
                        ValueLiteral::int("2147483647".to_string()),
                    ),
                    input(
                        &["App", "c"],
                        Prim::I64,
                        ValueLiteral::int("-9223372036854775808".to_string()),
                    ),
                ],
            )]),
            &stubs(&[]),
        );

        assert!(init_of(&graph, &["App", "a"]).bits_eq(&Value::I32(i32::MIN)));
        assert!(init_of(&graph, &["App", "b"]).bits_eq(&Value::I32(i32::MAX)));
        assert!(init_of(&graph, &["App", "c"]).bits_eq(&Value::I64(i64::MIN)));
    }

    // FLOAT text has no overflow failure: out-of-range digits round per IEEE —
    // to infinity at the extreme — because that is float semantics, and the
    // parse happens at the DECLARED width, so a value only f64 holds rounds to
    // f32's infinity under an f32 cell.
    #[test]
    fn a_float_init_out_of_range_rounds_per_ieee() {
        let huge = format!("4{}.0", "0".repeat(400));
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![
                    input(&["App", "a"], Prim::F64, ValueLiteral::float(huge)),
                    input(
                        &["App", "b"],
                        Prim::F32,
                        ValueLiteral::float(
                            "1000000000000000000000000000000000000000.0".to_string(),
                        ),
                    ),
                ],
            )]),
            &stubs(&[]),
        );

        assert!(init_of(&graph, &["App", "a"]).bits_eq(&Value::F64(f64::INFINITY)));
        assert!(init_of(&graph, &["App", "b"]).bits_eq(&Value::F32(f32::INFINITY)));
    }

    // Check 8: a two-cycle. The diagnostic points at the cycle's rank-least
    // member — path and span — and the message names the members (R21), which
    // is the one place tests read a message: the member list IS the contract
    // there, not wording.
    #[test]
    fn rejects_a_cycle_of_two() {
        let mut first = derived(&["App", "a"], Prim::I64, 1, &[&["App", "b"]]);
        first.span = Span::new(10, 5);
        let second = derived(&["App", "b"], Prim::I64, 1, &[&["App", "a"]]);

        let diagnostic = failure(
            &module(vec![scope(&["App"], vec![first, second])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "derived_cycle");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.a"));
        assert_eq!(diagnostic.span, Some(Span::new(10, 5)));
        assert!(
            diagnostic.message.contains("App.a"),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains("App.b"),
            "{}",
            diagnostic.message
        );
    }

    // A derived reading itself is the one-member cycle.
    #[test]
    fn rejects_a_self_cycle_of_one() {
        let diagnostic = failure(
            &module(vec![scope(
                &["App"],
                vec![derived(&["App", "a"], Prim::I64, 1, &[&["App", "a"]])],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "derived_cycle");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.a"));
        assert!(
            diagnostic.message.contains("App.a"),
            "{}",
            diagnostic.message
        );
    }

    // The extraction walks in from OUTSIDE the cycle when the rank-least
    // leftover is only a dependent: `d` reads the `b`↔`c` cycle and never
    // becomes ready, yet the reported cycle is `b`↔`c` — `d` is not a member —
    // and the blame lands on the cycle's own rank-least member.
    #[test]
    fn a_cycle_report_names_members_only_not_dependents() {
        let dependent = derived(&["App", "d"], Prim::I64, 1, &[&["App", "b"]]);
        let mut member = derived(&["App", "b"], Prim::I64, 1, &[&["App", "c"]]);
        member.span = Span::new(70, 8);
        let closer = derived(&["App", "c"], Prim::I64, 1, &[&["App", "b"]]);

        let diagnostic = failure(
            &module(vec![scope(&["App"], vec![dependent, member, closer])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "derived_cycle");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.b"));
        assert_eq!(diagnostic.span, Some(Span::new(70, 8)));
        assert!(
            diagnostic.message.contains("App.b"),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains("App.c"),
            "{}",
            diagnostic.message
        );
        assert!(
            !diagnostic.message.contains("App.d"),
            "{}",
            diagnostic.message
        );
    }

    // The pipeline composes: corpus-style text through lex + parse + load. The
    // module exercises both kinds, a forward reference, and a cross-scope
    // read; the loaded graph carries ranks, links and the static order.
    #[test]
    fn loads_a_lexed_and_parsed_module_end_to_end() {
        let source = "module App {\n  types {}\n  graph {\n    scope App exposes [] effects [] {\n      derived App.doubled : i64 uses B@d1 inputs [App.count]\n      input App.count : i64 init 20\n    }\n    scope App.panel exposes [] effects [] {\n      derived App.panel.label : i64 uses B@d1 inputs [App.doubled]\n    }\n  }\n  behaviors {}\n}\n";
        let tokens = ductus_ir::text::lexer::lex(source).expect("the source lexes");
        let parsed = ductus_ir::text::parser::parse(tokens).expect("the source parses");
        let graph = loaded(&parsed, &stubs(&[0xd1]));

        let ranks: Vec<String> = graph.cells.iter().map(|cell| cell.path.render()).collect();

        assert_eq!(ranks, ["App.doubled", "App.count", "App.panel.label"]);
        assert!(init_of(&graph, &["App", "count"]).bits_eq(&Value::I64(20)));
        assert_eq!(graph.static_order, [CellIndex(0), CellIndex(2)]);

        let CellRole::Derived { inputs, .. } = &graph.cells[0].role else {
            panic!("rank 0 is the derived");
        };

        assert_eq!(inputs, &[CellIndex(1)]);
    }

    // And the rejecting half of the pipeline: the parser admits the grammar
    // (F4), the loader rejects the slice — a `fold` cell and a keyed path,
    // straight from text.
    #[test]
    fn rejects_out_of_slice_text_end_to_end() {
        let fold = "module App {\n  types {}\n  graph {\n    scope App exposes [] effects [] {\n      input App.x : i64 init 0\n      fold App.sum : i64 combiner B@d2 else 0\n    }\n  }\n  behaviors {}\n}\n";
        let tokens = ductus_ir::text::lexer::lex(fold).expect("the source lexes");
        let parsed = ductus_ir::text::parser::parse(tokens).expect("the source parses");
        let diagnostic = failure(&parsed, &stubs(&[0xd2]));

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.sum"));

        let keyed = "module App {\n  types {}\n  graph {\n    scope App exposes [] effects [] {\n      input App.rows.'42'.value : i64 init 0\n    }\n  }\n  behaviors {}\n}\n";
        let tokens = ductus_ir::text::lexer::lex(keyed).expect("the source lexes");
        let parsed = ductus_ir::text::parser::parse(tokens).expect("the source parses");
        let diagnostic = failure(&parsed, &stubs(&[]));

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(
            rendered_path(&diagnostic).as_deref(),
            Some("App.rows.'42'.value")
        );
    }

    // The hierarchy law through the whole pipeline, rejecting half: `scope Foo`
    // inside `module App` is grammatical (F4) and unanchored, so the text lexes
    // and parses and the LOADER is what refuses it.
    #[test]
    fn rejects_an_unanchored_scope_end_to_end() {
        let source = "module App {\n  types {}\n  graph {\n    scope Foo exposes [] effects [] {\n      input Foo.x : i64 init 0\n    }\n  }\n  behaviors {}\n}\n";
        let tokens = ductus_ir::text::lexer::lex(source).expect("the source lexes");
        let parsed = ductus_ir::text::parser::parse(tokens).expect("the source parses");
        let diagnostic = failure(&parsed, &stubs(&[]));

        assert_eq!(diagnostic.class.id(), "unanchored_scope_path");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("Foo"));
    }

    // A recurrent's whole load shape in one witness: `uses` resolved, `inputs`
    // linked, an omitted `depth` defaulting to 1 (C-7/C-8), `init` expanded
    // into the uniform per-depth fallbacks (R71), the `lookback` key linked to
    // the input it names, and the cell taking its place in the static order —
    // which no M0 graph's order held.
    #[test]
    fn loads_a_recurrent_with_its_linked_history() {
        let mut reader = recurrent(&["App", "r"], Prim::I64, 1, &[&["App", "x"]], 3);
        reader.depth = None;
        reader.lookback = vec![LookbackEntry {
            cell: path(&["App", "x"]),
            k: 2,
        }];

        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![
                    input(&["App", "x"], Prim::I64, ValueLiteral::int("4".to_string())),
                    reader,
                ],
            )]),
            &stubs(&[1]),
        );
        let (depth, lookback, fallbacks) = history_of(&graph, &["App", "r"]);

        assert_eq!(depth, 1);
        assert_eq!(fallbacks.len(), 1);
        assert!(fallbacks[0].bits_eq(&Value::I64(0)));
        assert_eq!(lookback.len(), 1);
        assert_eq!(lookback[0].input, CellIndex(0));
        assert_eq!(lookback[0].k, 2);
        assert_eq!(graph.static_order, [CellIndex(1)]);
    }

    // `init` on a recurrent IS the uniform `fallbacks` spelling: every depth
    // gets the same converted value (R71, 033-268).
    #[test]
    fn init_on_a_recurrent_seeds_every_depth() {
        let mut uniform = recurrent(&["App", "r"], Prim::I64, 1, &[], 3);
        uniform.init = Some(ValueLiteral::int("-2".to_string()));

        let graph = loaded(&module(vec![scope(&["App"], vec![uniform])]), &stubs(&[1]));
        let (depth, _, fallbacks) = history_of(&graph, &["App", "r"]);

        assert_eq!(depth, 3);
        assert_eq!(fallbacks.len(), 3);

        for fallback in &fallbacks {
            assert!(fallback.bits_eq(&Value::I64(-2)));
        }
    }

    // The general form, one seed per depth, each converting under the declared
    // type — the corpus's `fib` shape.
    #[test]
    fn a_fallback_map_seeds_each_depth_in_key_order() {
        let mut fib = recurrent(&["App", "fib"], Prim::I64, 1, &[], 2);
        fib.init = None;
        fib.fallbacks = vec![
            FallbackEntry {
                k: 2,
                value: ValueLiteral::int("0".to_string()),
            },
            FallbackEntry {
                k: 1,
                value: ValueLiteral::int("1".to_string()),
            },
        ];

        let graph = loaded(&module(vec![scope(&["App"], vec![fib])]), &stubs(&[1]));
        let (_, _, fallbacks) = history_of(&graph, &["App", "fib"]);

        assert!(fallbacks[0].bits_eq(&Value::I64(1)));
        assert!(fallbacks[1].bits_eq(&Value::I64(0)));
    }

    // 016-51: prior self-values are readable only through `.previous`/`.past`,
    // which ride `depth` and `fallbacks`. `inputs` carries VALUE reads, so a
    // recurrent naming ITSELF there asks for the value it is at that moment
    // computing — a read no source expression can spell, and one §13.2.4.1
    // pointedly denies even to the cell's own history accessors ("the value
    // being computed in the current commit is not visible").
    //
    // Left standing it also free-runs: §4.12 makes its own advance a dirty root
    // at the next commit, its own dirty bit is among its staleness inputs, so it
    // advances once per commit forever with no host activity. The sibling
    // rejection is `deferred_self_reference`.
    #[test]
    fn rejects_a_recurrent_reading_its_own_value() {
        let mut narcissus = recurrent(&["App", "r"], Prim::I64, 1, &[&["App", "r"]], 1);
        narcissus.span = Span::new(12, 30);

        let diagnostic = failure(
            &module(vec![scope(&["App"], vec![narcissus])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "recurrent_self_reference");
        assert_eq!(diagnostic.span, Some(Span::new(12, 30)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.r"));
    }

    // And the shape it is NOT: a recurrent whose only history is its own ring.
    // `depth` is the self-read, no `inputs` entry names the cell, and the module
    // loads — §6.1's witness 4.
    #[test]
    fn a_recurrent_reading_only_its_own_ring_loads() {
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![recurrent(&["App", "r"], Prim::I64, 1, &[], 2)],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(graph.static_order, [CellIndex(0)]);
    }

    // RW-4, the general case: a loop closed THROUGH a recurrent is not a cycle
    // either, because the edge out of a recurrent is a delay element
    // (§13.11.4). The derived evaluates first — the only edge that counts runs
    // from it into the recurrent.
    #[test]
    fn a_loop_closed_through_a_recurrent_loads() {
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![
                    derived(&["App", "d"], Prim::I64, 1, &[&["App", "r"]]),
                    recurrent(&["App", "r"], Prim::I64, 1, &[&["App", "d"]], 1),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(graph.static_order, [CellIndex(0), CellIndex(1)]);
    }

    // And the report stays honest about who is IN the cycle: the recurrent
    // reads the `b`↔`c` ring and never becomes ready, but it is not a member
    // and is not named.
    #[test]
    fn a_cycle_report_never_names_a_recurrent() {
        let diagnostic = failure(
            &module(vec![scope(
                &["App"],
                vec![
                    recurrent(&["App", "r"], Prim::I64, 1, &[&["App", "b"]], 1),
                    derived(&["App", "b"], Prim::I64, 1, &[&["App", "c"]]),
                    derived(&["App", "c"], Prim::I64, 1, &[&["App", "b"]]),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "derived_cycle");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.b"));
        assert!(
            !diagnostic.message.contains("App.r"),
            "{}",
            diagnostic.message
        );
    }

    // The `recurrent` row's required clause, and the clauses no wave of the
    // slice runs yet — gate machinery and the fold pair.
    #[test]
    fn rejects_a_recurrent_without_uses_and_every_forbidden_clause() {
        let mut bare = recurrent(&["App", "r"], Prim::I64, 1, &[], 1);
        bare.uses = None;
        let diagnostic = failure(&module(vec![scope(&["App"], vec![bare])]), &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.r"));

        let mutations: [(&str, CellEdit); 3] = [
            ("combiner", |cell| cell.combiner = Some(2)),
            ("else", |cell| {
                cell.else_value = Some(ValueLiteral::Int("0".to_string()))
            }),
            ("members", |cell| {
                cell.members.push(Member {
                    path: path(&["App", "m"]),
                    driver: MemberDriver::GateGuarded,
                });
            }),
        ];

        for (clause, mutate) in mutations {
            let mut built = recurrent(&["App", "r"], Prim::I64, 1, &[], 1);
            mutate(&mut built);
            let diagnostic = failure(&module(vec![scope(&["App"], vec![built])]), &stubs(&[1]));

            assert_eq!(diagnostic.class.id(), "outside_m0_slice", "{clause}");
            assert_eq!(
                rendered_path(&diagnostic).as_deref(),
                Some("App.r"),
                "{clause}"
            );
        }
    }

    // One meaning, one spelling (R71): `init` IS the uniform `fallbacks`, so a
    // cell carrying both gives one question two answers.
    #[test]
    fn rejects_a_recurrent_carrying_both_init_and_fallbacks() {
        let mut both = recurrent(&["App", "r"], Prim::I64, 1, &[], 1);
        both.span = Span::new(15, 40);
        both.fallbacks = vec![FallbackEntry {
            k: 1,
            value: ValueLiteral::int("1".to_string()),
        }];

        let diagnostic = failure(&module(vec![scope(&["App"], vec![both])]), &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "conflicting_history_fallbacks");
        assert_eq!(diagnostic.span, Some(Span::new(15, 40)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.r"));
    }

    // C-7/C-8: a zero-slot ring names no ABI parameter and allocates nothing,
    // and neither does a zero lookback.
    #[test]
    fn rejects_a_zero_depth_and_a_zero_lookback() {
        let zero_depth = recurrent(&["App", "r"], Prim::I64, 1, &[], 0);
        let diagnostic = failure(
            &module(vec![scope(&["App"], vec![zero_depth])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "invalid_history_depth");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.r"));

        let mut zero_lookback = recurrent(&["App", "r"], Prim::I64, 1, &[&["App", "x"]], 1);
        zero_lookback.lookback = vec![LookbackEntry {
            cell: path(&["App", "x"]),
            k: 0,
        }];

        let diagnostic = failure(
            &module(vec![scope(
                &["App"],
                vec![
                    input(&["App", "x"], Prim::I64, ValueLiteral::int("0".to_string())),
                    zero_lookback,
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "invalid_history_depth");
    }

    // A key outside `1 ..= depth` addresses a slot the cell does not have —
    // the same law seen from the other end.
    #[test]
    fn rejects_a_fallback_key_outside_the_declared_depth() {
        for key in [0, 3] {
            let mut out_of_range = recurrent(&["App", "r"], Prim::I64, 1, &[], 2);
            out_of_range.init = None;
            out_of_range.fallbacks = vec![FallbackEntry {
                k: key,
                value: ValueLiteral::int("0".to_string()),
            }];

            let diagnostic = failure(
                &module(vec![scope(&["App"], vec![out_of_range])]),
                &stubs(&[1]),
            );

            assert_eq!(diagnostic.class.id(), "invalid_history_depth", "{key}");
            assert_eq!(
                rendered_path(&diagnostic).as_deref(),
                Some("App.r"),
                "{key}"
            );
        }
    }

    // The clause is a MAP: one seed per depth, so a repeated key is two
    // answers for one slot.
    #[test]
    fn rejects_a_duplicate_fallback_key() {
        let mut repeated = recurrent(&["App", "r"], Prim::I64, 1, &[], 2);
        repeated.init = None;
        repeated.fallbacks = vec![
            FallbackEntry {
                k: 1,
                value: ValueLiteral::int("1".to_string()),
            },
            FallbackEntry {
                k: 1,
                value: ValueLiteral::int("2".to_string()),
            },
        ];

        let diagnostic = failure(&module(vec![scope(&["App"], vec![repeated])]), &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "duplicate_fallback_key");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.r"));
    }

    // TOTALITY: every depth is read before the ring has advanced that far, so
    // every depth needs a seed. A map with a hole and a cell carrying neither
    // clause are the same failure.
    #[test]
    fn rejects_a_history_depth_no_fallback_answers_for() {
        let mut hole = recurrent(&["App", "r"], Prim::I64, 1, &[], 2);
        hole.init = None;
        hole.fallbacks = vec![FallbackEntry {
            k: 1,
            value: ValueLiteral::int("1".to_string()),
        }];

        let diagnostic = failure(&module(vec![scope(&["App"], vec![hole])]), &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "missing_fallback_key");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.r"));

        let mut seedless = recurrent(&["App", "r"], Prim::I64, 1, &[], 1);
        seedless.init = None;

        let diagnostic = failure(&module(vec![scope(&["App"], vec![seedless])]), &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "missing_fallback_key");
    }

    // A fallback converts under the cell's declared type exactly as `init`
    // does (R57), and fails into the same class.
    #[test]
    fn rejects_a_fallback_that_does_not_convert() {
        let mut mistyped = recurrent(&["App", "r"], Prim::I64, 1, &[], 1);
        mistyped.init = None;
        mistyped.fallbacks = vec![FallbackEntry {
            k: 1,
            value: ValueLiteral::True,
        }];

        let diagnostic = failure(&module(vec![scope(&["App"], vec![mistyped])]), &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "type_mismatched_init");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.r"));
    }

    // A `lookback` key answers to two laws. A path no cell declares is the
    // ordinary `unknown_input_path`, carrying the MISSING path…
    #[test]
    fn rejects_an_unknown_lookback_key_carrying_the_missing_path() {
        let mut ghost_reader = recurrent(&["App", "r"], Prim::I64, 1, &[], 1);
        ghost_reader.lookback = vec![LookbackEntry {
            cell: path(&["App", "ghost"]),
            k: 1,
        }];

        let diagnostic = failure(
            &module(vec![scope(&["App"], vec![ghost_reader])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "unknown_input_path");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.ghost"));
    }

    // …while a path naming a REAL cell the recurrent does not read has no
    // argument position to arrive in, and gets its own class.
    #[test]
    fn rejects_a_lookback_key_that_is_not_an_input() {
        let mut stranger = recurrent(&["App", "r"], Prim::I64, 1, &[&["App", "x"]], 1);
        stranger.lookback = vec![LookbackEntry {
            cell: path(&["App", "y"]),
            k: 1,
        }];

        let diagnostic = failure(
            &module(vec![scope(
                &["App"],
                vec![
                    input(&["App", "x"], Prim::I64, ValueLiteral::int("0".to_string())),
                    input(&["App", "y"], Prim::I64, ValueLiteral::int("0".to_string())),
                    stranger,
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "lookback_not_an_input");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.y"));
    }

    // The recurrent line through the whole pipeline: IR_GRAMMAR §5's
    // accumulator and the corpus's two-fallback `fib`, both from text.
    #[test]
    fn loads_recurrent_text_end_to_end() {
        let source = "module App {\n  types {}\n  graph {\n    scope App exposes [] effects [] {\n      input App.count : i64 init 0\n      recurrent App.total : i64 uses B@d2 inputs [App.count] depth 1 init 0\n      recurrent App.fib : i64 uses B@d2 inputs [App.count] depth 2 lookback [App.count: 2] fallbacks [1: 1, 2: 0]\n    }\n  }\n  behaviors {}\n}\n";
        let tokens = ductus_ir::text::lexer::lex(source).expect("the source lexes");
        let parsed = ductus_ir::text::parser::parse(tokens).expect("the source parses");
        let graph = loaded(&parsed, &stubs(&[0xd2]));
        let (depth, lookback, fallbacks) = history_of(&graph, &["App", "fib"]);

        assert_eq!(depth, 2);
        assert_eq!(lookback.len(), 1);
        assert_eq!(lookback[0].input, CellIndex(0));
        assert_eq!(lookback[0].k, 2);
        assert!(fallbacks[0].bits_eq(&Value::I64(1)));
        assert!(fallbacks[1].bits_eq(&Value::I64(0)));
        assert_eq!(graph.static_order, [CellIndex(1), CellIndex(2)]);
    }

    // A deferred's whole load shape in one witness: `uses` resolved, `inputs`
    // linked, the latency carried — and the cell present in the ONE order,
    // because it evaluates once at startup (C-23). §13.10.2's "the cell's own
    // body is never a step-2 DAG node" is honoured by step 3 walking past it,
    // which is the engine's law rather than the loader's (R80 item 4).
    #[test]
    fn loads_a_deferred_into_the_order() {
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![
                    input(&["App", "x"], Prim::I64, ValueLiteral::int("4".to_string())),
                    deferred(&["App", "later"], Prim::I64, 1, &[&["App", "x"]], 2),
                ],
            )]),
            &stubs(&[1]),
        );
        let (depth, inputs) = latency_of(&graph, &["App", "later"]);

        assert_eq!(depth, 2);
        assert_eq!(inputs, [CellIndex(0)]);
        assert_eq!(graph.static_order, [CellIndex(1)]);
    }

    // A read of a `deferred` orders like every other read: `App.a` must see the
    // value `App.f`'s startup evaluation produced, so it follows `App.f`, which
    // follows the `App.p` it reads. The result is `q, p, f, a` — declaration
    // order broken only where an edge says so.
    #[test]
    fn a_read_of_a_deferred_orders_its_reader_after_it() {
        let graph = loaded(
            &module(vec![scope(
                &["App"],
                vec![
                    input(&["App", "x"], Prim::I64, ValueLiteral::int("0".to_string())),
                    derived(&["App", "a"], Prim::I64, 1, &[&["App", "f"]]),
                    derived(&["App", "q"], Prim::I64, 1, &[]),
                    derived(&["App", "p"], Prim::I64, 1, &[&["App", "x"]]),
                    deferred(&["App", "f"], Prim::I64, 1, &[&["App", "p"]], 1),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(
            graph.static_order,
            [CellIndex(2), CellIndex(3), CellIndex(4), CellIndex(1)]
        );
    }

    // §13.11.4: "A deferred cell is not, however, a cycle device … cycle
    // breaking remains the recurrent's role." So a loop closed through one is
    // an ordinary `derived_cycle`, and the deferred IS named as a member —
    // unlike a recurrent, which the report never names.
    #[test]
    fn a_loop_closed_through_a_deferred_is_a_cycle() {
        let diagnostic = failure(
            &module(vec![scope(
                &["App"],
                vec![
                    derived(&["App", "d"], Prim::I64, 1, &[&["App", "f"]]),
                    deferred(&["App", "f"], Prim::I64, 1, &[&["App", "d"]], 1),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "derived_cycle");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.d"));
        assert_eq!(diagnostic.message, "derived cycle: App.d -> App.f -> App.d");
    }

    // §13.2.12: "its expression may not read the cell it declares." The delay
    // does not make the self-read well-founded — construction would evaluate
    // the cell over the slot it is at that moment computing.
    #[test]
    fn rejects_a_deferred_reading_itself() {
        let mut narcissus = deferred(&["App", "f"], Prim::I64, 1, &[&["App", "f"]], 1);
        narcissus.span = Span::new(7, 44);

        let diagnostic = failure(
            &module(vec![scope(&["App"], vec![narcissus])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "deferred_self_reference");
        assert_eq!(diagnostic.span, Some(Span::new(7, 44)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.f"));
    }

    // The `deferred` row's required clause, and the clauses it has no use for.
    // The three history clauses are the interesting half: a latency is not a
    // memory, so `init`, `fallbacks` and `lookback` have nothing to say about
    // one.
    #[test]
    fn rejects_a_deferred_without_uses_and_every_forbidden_clause() {
        let mut bare = deferred(&["App", "f"], Prim::I64, 1, &[], 1);
        bare.uses = None;
        let diagnostic = failure(&module(vec![scope(&["App"], vec![bare])]), &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.f"));

        let mutations: [(&str, CellEdit); 7] = [
            ("lookback", |cell| {
                cell.lookback.push(LookbackEntry {
                    cell: path(&["App", "x"]),
                    k: 1,
                });
            }),
            ("reset_on_reopen", |cell| cell.reset_on_reopen = true),
            ("init", |cell| {
                cell.init = Some(ValueLiteral::int("0".to_string()));
            }),
            ("fallbacks", |cell| {
                cell.fallbacks.push(FallbackEntry {
                    k: 1,
                    value: ValueLiteral::int("0".to_string()),
                });
            }),
            ("combiner", |cell| cell.combiner = Some(2)),
            ("else", |cell| {
                cell.else_value = Some(ValueLiteral::int("0".to_string()));
            }),
            ("members", |cell| {
                cell.members.push(Member {
                    path: path(&["App", "m"]),
                    driver: MemberDriver::GateGuarded,
                });
            }),
        ];

        for (clause, mutate) in mutations {
            let mut built = deferred(&["App", "f"], Prim::I64, 1, &[], 1);
            mutate(&mut built);
            let diagnostic = failure(&module(vec![scope(&["App"], vec![built])]), &stubs(&[1]));

            assert_eq!(diagnostic.class.id(), "outside_m0_slice", "{clause}");
            assert_eq!(
                rendered_path(&diagnostic).as_deref(),
                Some("App.f"),
                "{clause}"
            );
        }
    }

    // C-24 / 016-301: only a WRITTEN zero is the error. A zero-commit latency is
    // a `derived` under another name, so it is rejected — with the deferred's
    // own message, not the recurrent's.
    #[test]
    fn rejects_a_deferred_with_a_zero_latency() {
        let mut built = deferred(&["App", "f"], Prim::I64, 1, &[], 1);
        built.depth = Some(0);
        let diagnostic = failure(&module(vec![scope(&["App"], vec![built])]), &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "invalid_history_depth");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.f"));
    }

    // And the half the loader used to get wrong (R80 item 3): 016-301 gives a
    // deferred the SAME default a recurrent gets — *"an omitted depth on a
    // `deferred` is 1"* — so a line without the clause loads at latency 1.
    #[test]
    fn an_omitted_deferred_depth_is_one_commit() {
        let mut built = deferred(&["App", "f"], Prim::I64, 1, &[], 1);
        built.depth = None;
        let graph = loaded(&module(vec![scope(&["App"], vec![built])]), &stubs(&[1]));
        let (depth, _) = latency_of(&graph, &["App", "f"]);

        assert_eq!(depth, 1);
    }

    // The deferred line through the whole pipeline — the same text the M0
    // loader used to reject as out of slice, now loading.
    #[test]
    fn loads_deferred_text_end_to_end() {
        let source = "module App {\n  types {}\n  graph {\n    scope App exposes [] effects [] {\n      input App.x : i64 init 0\n      deferred App.later : i64 uses B@d2 inputs [App.x] depth 3\n      derived App.shown : i64 uses B@d2 inputs [App.later]\n    }\n  }\n  behaviors {}\n}\n";
        let tokens = ductus_ir::text::lexer::lex(source).expect("the source lexes");
        let parsed = ductus_ir::text::parser::parse(tokens).expect("the source parses");
        let graph = loaded(&parsed, &stubs(&[0xd2]));
        let (depth, inputs) = latency_of(&graph, &["App", "later"]);

        assert_eq!(depth, 3);
        assert_eq!(inputs, [CellIndex(0)]);
        assert_eq!(graph.static_order, [CellIndex(1), CellIndex(2)]);
    }

    // And the accepting half: a cell two segments below its scope, with no
    // scope declared for the segment in between, loads from text — depth is
    // free, only the prefix is law.
    #[test]
    fn loads_a_deeply_contained_cell_end_to_end() {
        let source = "module App {\n  types {}\n  graph {\n    scope App exposes [] effects [] {\n      input App.print.text : i64 init 3\n      derived App.print.shown : i64 uses B@d1 inputs [App.print.text]\n    }\n  }\n  behaviors {}\n}\n";
        let tokens = ductus_ir::text::lexer::lex(source).expect("the source lexes");
        let parsed = ductus_ir::text::parser::parse(tokens).expect("the source parses");
        let graph = loaded(&parsed, &stubs(&[0xd1]));

        let ranks: Vec<String> = graph.cells.iter().map(|cell| cell.path.render()).collect();

        assert_eq!(ranks, ["App.print.text", "App.print.shown"]);
        assert!(init_of(&graph, &["App", "print", "text"]).bits_eq(&Value::I64(3)));
        assert_eq!(graph.static_order, [CellIndex(1)]);
    }

    // Check 4, the admitting half: a `gate` entry loads, carrying its resolved
    // predicate, its linked inputs and no parent — and it joins the evaluation
    // order NOT AT ALL, because a predicate is not a DAG node (R73).
    #[test]
    fn a_gate_loads_with_its_predicate_and_linked_inputs() {
        let mut guarded = input(&["App", "x"], Prim::I64, ValueLiteral::int("1".to_string()));
        guarded.gate = Some(path(&["App", "g"]));

        let graph = loaded(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Cell(input(&["App", "on"], Prim::Bool, ValueLiteral::True)),
                    Entry::Cell(guarded),
                    Entry::Gate(gate(&["App", "g"], 1, &[&["App", "on"]], &[&["App", "x"]])),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(graph.gates.len(), 1);
        assert_eq!(graph.gates[0].path.render(), "App.g");
        assert_eq!(graph.gates[0].inputs, [CellIndex(0)]);
        assert_eq!(graph.gates[0].parent, None);
        assert_eq!(gate_of(&graph, &["App", "x"]), Some(GateIndex(0)));
        assert_eq!(gate_of(&graph, &["App", "on"]), None);
        assert!(graph.static_order.is_empty());
    }

    // C-45: a gate takes graph-section entry rank exactly like every entry, so
    // the entry ranks count cells and gates together while the `CellIndex`
    // sequence stays dense over cells alone. Step 1's root walk merges the two
    // by the first number.
    #[test]
    fn a_gate_takes_entry_rank_beside_the_cells() {
        let graph = loaded(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Cell(input(&["App", "on"], Prim::Bool, ValueLiteral::True)),
                    Entry::Gate(gate(&["App", "g"], 1, &[&["App", "on"]], &[])),
                    Entry::Cell(input(
                        &["App", "x"],
                        Prim::I64,
                        ValueLiteral::int("1".to_string()),
                    )),
                ],
            )]),
            &stubs(&[1]),
        );

        let ranks: Vec<usize> = graph.cells.iter().map(|cell| cell.entry_rank).collect();

        assert_eq!(ranks, [0, 2]);
        assert_eq!(graph.gates[0].entry_rank, 1);
    }

    // A cell's own `gate PATH` clause is the BINDING, and the gate may be
    // declared LATER — it is resolved in Pass B, so forward references work here
    // as everywhere. The gate's `guards` list restates it (C-43).
    #[test]
    fn a_cells_own_gate_clause_binds_it() {
        let mut guarded = input(&["App", "x"], Prim::I64, ValueLiteral::int("1".to_string()));
        guarded.gate = Some(path(&["App", "g"]));

        let graph = loaded(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Cell(input(&["App", "on"], Prim::Bool, ValueLiteral::True)),
                    Entry::Cell(guarded),
                    Entry::Gate(gate(&["App", "g"], 1, &[&["App", "on"]], &[&["App", "x"]])),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(gate_of(&graph, &["App", "x"]), Some(GateIndex(0)));
    }

    // The two spellings naming the SAME gate is what the inverse law REQUIRES:
    // one fact stated from both ends binds once.
    #[test]
    fn the_two_binding_spellings_agree_and_bind_once() {
        let mut guarded = input(&["App", "x"], Prim::I64, ValueLiteral::int("1".to_string()));
        guarded.gate = Some(path(&["App", "g"]));

        let graph = loaded(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Cell(input(&["App", "on"], Prim::Bool, ValueLiteral::True)),
                    Entry::Cell(guarded),
                    Entry::Gate(gate(&["App", "g"], 1, &[&["App", "on"]], &[&["App", "x"]])),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(gate_of(&graph, &["App", "x"]), Some(GateIndex(0)));
    }

    // Gate nesting spells the same binding from both ends: the inner gate's
    // `gate_parent` names the enclosing gate, and the outer gate's `guards`
    // names the inner one back. Only the first is the binding the engine reads.
    #[test]
    fn a_gate_parent_clause_nests_a_gate() {
        let mut inner = gate(&["App", "inner"], 1, &[&["App", "on"]], &[]);
        inner.gate_parent = Some(path(&["App", "outer"]));

        let graph = loaded(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Cell(input(&["App", "on"], Prim::Bool, ValueLiteral::True)),
                    Entry::Gate(gate(
                        &["App", "outer"],
                        1,
                        &[&["App", "on"]],
                        &[&["App", "inner"]],
                    )),
                    Entry::Gate(inner),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(graph.gates[0].parent, None);
        assert_eq!(graph.gates[1].parent, Some(GateIndex(0)));
    }

    // The inverse is checked in BOTH directions, and this is the half a fold
    // would have hidden: an outer gate that guards an inner one which does not
    // name it back has stated a nesting the inner gate never agreed to.
    #[test]
    fn rejects_a_guards_entry_the_inner_gate_does_not_name_back() {
        let diagnostic = failure(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Cell(input(&["App", "on"], Prim::Bool, ValueLiteral::True)),
                    Entry::Gate(gate(
                        &["App", "outer"],
                        1,
                        &[&["App", "on"]],
                        &[&["App", "inner"]],
                    )),
                    Entry::Gate(gate(&["App", "inner"], 1, &[&["App", "on"]], &[])),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "guards_inverse_mismatch");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.inner"));
    }

    // Check 8: a predicate reads ordinary graph cells, so an unknown path is
    // the same `unknown_input_path` a derived's is — carrying the MISSING path
    // with the reading gate's span.
    #[test]
    fn rejects_a_gate_reading_an_undeclared_cell() {
        let mut reader = gate(&["App", "g"], 1, &[&["App", "missing"]], &[]);
        reader.span = Span::new(11, 24);

        let diagnostic = failure(
            &module(vec![entries(&["App"], vec![Entry::Gate(reader)])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "unknown_input_path");
        assert_eq!(diagnostic.span, Some(Span::new(11, 24)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.missing"));
    }

    // A guard is not a read, so a path naming neither a cell nor a gate gets
    // its own class.
    #[test]
    fn rejects_a_guard_naming_neither_a_cell_nor_a_gate() {
        let mut guarding = gate(&["App", "g"], 1, &[], &[&["App", "nowhere"]]);
        guarding.span = Span::new(4, 30);

        let diagnostic = failure(
            &module(vec![entries(&["App"], vec![Entry::Gate(guarding)])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "unknown_guard_path");
        assert_eq!(diagnostic.span, Some(Span::new(4, 30)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.nowhere"));
    }

    // `gate_parent` must name a GATE: it is the link effective activation
    // climbs, and a cell has no activation to contribute.
    #[test]
    fn rejects_a_gate_parent_that_names_no_gate() {
        let mut child = gate(&["App", "g"], 1, &[], &[]);
        child.gate_parent = Some(path(&["App", "x"]));

        let diagnostic = failure(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Cell(input(
                        &["App", "x"],
                        Prim::I64,
                        ValueLiteral::int("1".to_string()),
                    )),
                    Entry::Gate(child),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "unknown_gate_parent");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));
    }

    // C-47: the chain must terminate, because effective activation is composed
    // from the ROOT down. The report names a genuine member of the loop.
    //
    // Each gate guards the other, so the inverse AGREES and the cycle check is
    // what the module reaches — the loop is in the nesting, not in the spelling.
    #[test]
    fn rejects_a_gate_parent_cycle() {
        let mut first = gate(&["App", "a"], 1, &[], &[&["App", "b"]]);
        first.gate_parent = Some(path(&["App", "b"]));
        let mut second = gate(&["App", "b"], 1, &[], &[&["App", "a"]]);
        second.gate_parent = Some(path(&["App", "a"]));

        let diagnostic = failure(
            &module(vec![entries(
                &["App"],
                vec![Entry::Gate(first), Entry::Gate(second)],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "gate_parent_cycle");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.a"));
    }

    // A gate that encloses ITSELF is the one-member loop, and it answers the
    // same way. Both spellings name the gate, so the inverse agrees and the
    // acyclicity check is what rejects it.
    #[test]
    fn rejects_a_gate_that_guards_itself() {
        let mut looping = gate(&["App", "g"], 1, &[], &[&["App", "g"]]);
        looping.gate_parent = Some(path(&["App", "g"]));

        let diagnostic = failure(
            &module(vec![entries(&["App"], vec![Entry::Gate(looping)])]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "gate_parent_cycle");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.g"));
    }

    // IR_GRAMMAR §3's `('gate' PATH)?` is a back-reference to a GATE entry;
    // pointing it at a cell declares no activation condition.
    #[test]
    fn rejects_a_cell_gated_by_something_that_is_not_a_gate() {
        let mut guarded = input(&["App", "x"], Prim::I64, ValueLiteral::int("1".to_string()));
        guarded.span = Span::new(7, 19);
        guarded.gate = Some(path(&["App", "on"]));

        let diagnostic = failure(
            &module(vec![scope(
                &["App"],
                vec![
                    input(&["App", "on"], Prim::Bool, ValueLiteral::True),
                    guarded,
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "unknown_gate_reference");
        assert_eq!(diagnostic.span, Some(Span::new(7, 19)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.on"));
    }

    // Three ways for the inverse to disagree, and all three are one class
    // (C-43): two gates listing one construct, a `guards` list contradicting the
    // construct's own clause, and a construct listed by a gate while declaring
    // no gate at all.
    #[test]
    fn rejects_every_disagreement_between_guards_and_the_own_clause() {
        let both_claim = module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(input(
                    &["App", "x"],
                    Prim::I64,
                    ValueLiteral::int("1".to_string()),
                )),
                Entry::Gate(gate(&["App", "a"], 1, &[], &[&["App", "x"]])),
                Entry::Gate(gate(&["App", "b"], 1, &[], &[&["App", "x"]])),
            ],
        )]);
        let diagnostic = failure(&both_claim, &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "guards_inverse_mismatch");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));

        let mut disagreeing = input(&["App", "x"], Prim::I64, ValueLiteral::int("1".to_string()));
        disagreeing.gate = Some(path(&["App", "a"]));
        let inverse_disagrees = module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(disagreeing),
                Entry::Gate(gate(&["App", "a"], 1, &[], &[&["App", "x"]])),
                Entry::Gate(gate(&["App", "b"], 1, &[], &[&["App", "x"]])),
            ],
        )]);
        let diagnostic = failure(&inverse_disagrees, &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "guards_inverse_mismatch");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));

        let unbound = module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(input(
                    &["App", "x"],
                    Prim::I64,
                    ValueLiteral::int("1".to_string()),
                )),
                Entry::Gate(gate(&["App", "a"], 1, &[], &[&["App", "x"]])),
            ],
        )]);
        let diagnostic = failure(&unbound, &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "guards_inverse_mismatch");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));
    }

    // And the backward direction on a CELL: the cell names its gate, the gate's
    // `guards` list does not name it back. The gate's span is what the
    // diagnostic points at, because its list is the incomplete one.
    #[test]
    fn rejects_a_cell_its_gate_does_not_name_in_guards() {
        let mut guarded = input(&["App", "x"], Prim::I64, ValueLiteral::int("1".to_string()));
        guarded.gate = Some(path(&["App", "g"]));
        let mut silent = gate(&["App", "g"], 1, &[], &[]);
        silent.span = Span::new(3, 21);

        let diagnostic = failure(
            &module(vec![entries(
                &["App"],
                vec![Entry::Cell(guarded), Entry::Gate(silent)],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "guards_inverse_mismatch");
        assert_eq!(diagnostic.span, Some(Span::new(3, 21)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));
    }

    // The gate-side sibling of `duplicate_scope_path` and
    // `duplicate_cell_path`: one path, one gate, and the SECOND declaration is
    // blamed.
    #[test]
    fn rejects_two_gates_declaring_one_path() {
        let mut repeat = gate(&["App", "g"], 1, &[], &[]);
        repeat.span = Span::new(50, 70);

        let diagnostic = failure(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Gate(gate(&["App", "g"], 1, &[], &[])),
                    Entry::Gate(repeat),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "duplicate_gate_path");
        assert_eq!(diagnostic.span, Some(Span::new(50, 70)));
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.g"));
    }

    // R66's law extends to gates in both directions: a path names a scope, a
    // cell OR a gate, never two of the three, and the second claim is the one
    // rejected.
    #[test]
    fn rejects_a_gate_colliding_with_a_cell_in_either_order() {
        let gate_then_cell = module(vec![entries(
            &["App"],
            vec![
                Entry::Gate(gate(&["App", "x"], 1, &[], &[])),
                Entry::Cell(input(
                    &["App", "x"],
                    Prim::I64,
                    ValueLiteral::int("1".to_string()),
                )),
            ],
        )]);
        let diagnostic = failure(&gate_then_cell, &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "scope_cell_collision");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));

        let cell_then_gate = module(vec![entries(
            &["App"],
            vec![
                Entry::Cell(input(
                    &["App", "x"],
                    Prim::I64,
                    ValueLiteral::int("1".to_string()),
                )),
                Entry::Gate(gate(&["App", "x"], 1, &[], &[])),
            ],
        )]);
        let diagnostic = failure(&cell_then_gate, &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "scope_cell_collision");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));
    }

    // And against a scope, in both directions too.
    #[test]
    fn rejects_a_gate_colliding_with_a_scope_in_either_order() {
        let gate_then_scope = module(vec![
            entries(
                &["App"],
                vec![Entry::Gate(gate(&["App", "panel"], 1, &[], &[]))],
            ),
            scope(&["App", "panel"], Vec::new()),
        ]);
        let diagnostic = failure(&gate_then_scope, &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "scope_cell_collision");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.panel"));

        let scope_then_gate = module(vec![
            scope(&["App", "panel"], Vec::new()),
            entries(
                &["App"],
                vec![Entry::Gate(gate(&["App", "panel"], 1, &[], &[]))],
            ),
        ]);
        let diagnostic = failure(&scope_then_gate, &stubs(&[1]));

        assert_eq!(diagnostic.class.id(), "scope_cell_collision");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.panel"));
    }

    // Containment binds a gate as it binds a cell: the braces and the dots
    // carry one hierarchy, and a gate's path is a place in the same tree.
    #[test]
    fn rejects_a_gate_outside_its_scope() {
        let diagnostic = failure(
            &module(vec![
                scope(&["App"], Vec::new()),
                entries(
                    &["App", "panel"],
                    vec![Entry::Gate(gate(&["App", "g"], 1, &[], &[]))],
                ),
            ]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "cell_outside_scope");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.g"));
    }

    // A `pred` resolves through the same seam a cell's `uses` does (D5).
    #[test]
    fn rejects_an_unresolvable_gate_predicate() {
        let diagnostic = failure(
            &module(vec![entries(
                &["App"],
                vec![Entry::Gate(gate(&["App", "g"], 0xbeef, &[], &[]))],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(diagnostic.class.id(), "unresolvable_behavior");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.g"));
    }

    // The SELF-CONDITIONAL gate — SPEC §13.9.6, and the owner's 2026-08-24
    // ruling (a) on R73's rider. A predicate reading a cell its own gate guards
    // is LEGAL and carries no special handling: the predicate runs in step 1a
    // over prior-commit values, so the read is already defined and no cycle
    // exists to break. The load-time rejection this used to draw
    // (`gate_predicate_reads_guarded`) is deleted, class and all.
    #[test]
    fn a_predicate_reading_a_cell_its_own_gate_guards_loads() {
        let reader = gate(&["App", "g"], 1, &[&["App", "x"]], &[&["App", "x"]]);
        let mut guarded = input(&["App", "x"], Prim::Bool, ValueLiteral::True);
        guarded.gate = Some(path(&["App", "g"]));

        let graph = loaded(
            &module(vec![entries(
                &["App"],
                vec![Entry::Cell(guarded), Entry::Gate(reader)],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(graph.gates[0].inputs, [CellIndex(0)]);
        assert_eq!(gate_of(&graph, &["App", "x"]), Some(GateIndex(0)));
    }

    // And transitively, on the same ruling: an ancestor gate may read a cell a
    // DESCENDANT gate guards, which is the same prior-commit read one level up.
    #[test]
    fn a_predicate_reading_a_cell_a_descendant_gate_guards_loads() {
        let mut inner = gate(&["App", "inner"], 1, &[], &[&["App", "x"]]);
        inner.gate_parent = Some(path(&["App", "outer"]));
        let mut guarded = input(&["App", "x"], Prim::Bool, ValueLiteral::True);
        guarded.gate = Some(path(&["App", "inner"]));

        let graph = loaded(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Cell(guarded),
                    Entry::Gate(gate(
                        &["App", "outer"],
                        1,
                        &[&["App", "x"]],
                        &[&["App", "inner"]],
                    )),
                    Entry::Gate(inner),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(graph.gates[0].inputs, [CellIndex(0)]);
        assert_eq!(gate_of(&graph, &["App", "x"]), Some(GateIndex(1)));
    }

    // Reading a cell OUTSIDE the subtree is ordinary, and so is a sibling gate
    // reading what this one guards. Kept as the neighbouring case to the two
    // above: no arrangement of predicate reads is a load error any more.
    #[test]
    fn admits_a_predicate_reading_outside_its_own_subtree() {
        let mut guarded = input(&["App", "x"], Prim::I64, ValueLiteral::int("1".to_string()));
        guarded.gate = Some(path(&["App", "a"]));

        let graph = loaded(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Cell(input(&["App", "on"], Prim::Bool, ValueLiteral::True)),
                    Entry::Cell(guarded),
                    Entry::Gate(gate(&["App", "a"], 1, &[&["App", "on"]], &[&["App", "x"]])),
                    Entry::Gate(gate(&["App", "b"], 1, &[&["App", "x"]], &[])),
                ],
            )]),
            &stubs(&[1]),
        );

        assert_eq!(graph.gates.len(), 2);
        assert_eq!(graph.gates[1].inputs, [CellIndex(1)]);
    }

    // C-42's other half: `reset_on_reopen` is a recurrent's clause, and a
    // recurrent may now carry it beside a `gate` back-reference.
    #[test]
    fn a_recurrent_carries_reset_on_reopen_beside_its_gate() {
        let mut resetting = recurrent(&["App", "r"], Prim::I64, 1, &[], 1);
        resetting.reset_on_reopen = true;
        resetting.gate = Some(path(&["App", "g"]));

        let graph = loaded(
            &module(vec![entries(
                &["App"],
                vec![
                    Entry::Cell(resetting),
                    Entry::Gate(gate(&["App", "g"], 1, &[], &[&["App", "r"]])),
                ],
            )]),
            &stubs(&[1]),
        );

        assert!(matches!(
            graph.cells[0].role,
            CellRole::Recurrent {
                reset_on_reopen: true,
                ..
            }
        ));
        assert_eq!(gate_of(&graph, &["App", "r"]), Some(GateIndex(0)));
    }

    // The whole gate surface from TEXT, through the real lexer and parser: the
    // grammar already had every clause, so nothing about `.duir` changed for
    // gates — only what the loader does with what it reads.
    #[test]
    fn loads_a_gated_module_end_to_end() {
        let source = "module App {\n  types {}\n  graph {\n    scope App exposes [] effects [] {\n      input App.on : bool init true\n      input App.x : i64 init 2 gate App.g\n      derived App.d : i64 uses B@d1 inputs [App.x] gate App.g\n      gate App.g pred B@d2 inputs [App.on] guards [App.x, App.d]\n    }\n  }\n  behaviors {}\n}\n";
        let tokens = ductus_ir::text::lexer::lex(source).expect("the source lexes");
        let parsed = ductus_ir::text::parser::parse(tokens).expect("the source parses");
        let graph = loaded(&parsed, &stubs(&[0xd1, 0xd2]));

        assert_eq!(graph.gates.len(), 1);
        assert_eq!(graph.gates[0].entry_rank, 3);
        assert_eq!(graph.gates[0].inputs, [CellIndex(0)]);
        assert_eq!(gate_of(&graph, &["App", "x"]), Some(GateIndex(0)));
        assert_eq!(gate_of(&graph, &["App", "d"]), Some(GateIndex(0)));
    }
}
