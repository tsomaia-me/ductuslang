//! [`load()`] — Module → [`LoadedGraph`]: the loader as the sole legality
//! authority (R21, F4).

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use ductus_ir::diag::{DiagClass, Diagnostic, Span};
use ductus_ir::graph::{Cell, CellKind, Entry, Scope, ValueLiteral};
use ductus_ir::module::Module;
use ductus_ir::path::{CellPath, Segment};
use ductus_ir::types::TypeTag;

use super::cell_index::CellIndex;
use super::cell_role::CellRole;
use super::loaded_cell::LoadedCell;
use super::loaded_graph::LoadedGraph;
use crate::provider::BehaviorProvider;
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
///    root, §15.4.1.1); a path some CELL already claims →
///    `scope_cell_collision`; a path some SCOPE already claims →
///    `duplicate_scope_path`.
/// 4. Entry kind: `gate`/`wire`/`effect`/`stream` → `outside_m0_slice`.
/// 5. Per cell: path shape as in 1; kind `recurrent`/`deferred`/`fold` →
///    `outside_m0_slice`; a type outside the word tier → `outside_m0_slice`;
///    the kind's clause law (`input` requires `init`, `derived` requires
///    `uses`, everything else forbidden); CONTAINMENT — the enclosing scope's
///    path must be a proper prefix of the cell's → `cell_outside_scope`; a
///    path some SCOPE claims → `scope_cell_collision`; a duplicate path (any
///    prior cell, any scope) → `duplicate_cell_path`; and an `input`'s `init`
///    conversion under the declared type (R57) → `type_mismatched_init` on
///    failure.
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
/// 6. Every derived's `inputs` path, rank order then list order: unknown →
///    `unknown_input_path` carrying the MISSING path.
/// 7. Every derived's `uses` handle, rank order: unresolvable through the
///    provider → `unresolvable_behavior` (D5: module table ∪ provider
///    registry — and the module table is empty throughout M0, so the registry
///    is the whole answer).
/// 8. Cycle detection over derived→derived edges, whose sort is ALSO the
///    static evaluation order: a cycle → `derived_cycle` with the members in
///    the message.
///
/// **What load() does NOT do:** no store allocation, no evaluation, no first
/// commit — those are the engine's (§13.14.1 steps 2–4). This is Module →
/// validated graph with resolved behaviors and the static order, nothing more.
pub fn load(module: &Module, provider: &dyn BehaviorProvider) -> Result<LoadedGraph, Diagnostic> {
    let mut pending: Vec<PendingCell> = Vec::new();
    let mut index: HashMap<CellPath, CellIndex> = HashMap::new();
    let mut scopes: HashSet<CellPath> = HashSet::new();

    // Pass A — file order: scopes in section order, entries in body order,
    // checks 1–5 per construct, rank assigned as each cell survives.
    for scope in &module.graph.scopes {
        admitted_scope(scope)?;
        anchored(scope, &module.name)?;
        claim_scope(&mut scopes, &index, scope)?;

        for entry in &scope.entries {
            let cell = admitted_entry(entry)?;

            plain_path(&cell.path, cell.span)?;
            let kind = admitted_kind(cell)?;
            let value_type = word_tier(cell)?;
            admitted_clauses(cell, kind)?;
            contained(scope, cell)?;
            claim(&mut index, &scopes, cell, CellIndex(pending.len()))?;
            let form = pending_form(cell, kind, value_type)?;

            pending.push(PendingCell {
                path: cell.path.clone(),
                span: cell.span,
                value_type,
                form,
            });
        }
    }

    // Pass B — checks 6–8 over the collected cells, each check a full
    // rank-order loop of its own so the error order stays deterministic.
    let links = linked_inputs(&pending, &index)?;
    let resolved = resolved_cells(pending, links, provider)?;
    let static_order = static_order(&resolved)?;

    let cells = resolved
        .into_iter()
        .map(|cell| LoadedCell {
            path: cell.path,
            value_type: cell.value_type,
            role: cell.role,
        })
        .collect();

    Ok(LoadedGraph {
        cells,
        index,
        static_order,
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
    value_type: ValueType,
    form: PendingForm,
}

/// The kind-specific half of a [`PendingCell`].
enum PendingForm {
    /// An `input`: its `init`, converted under the declared type (R57).
    Input { init: Value },
    /// A `derived`: its handle and its input PATHS — indices are check 6's.
    Derived { uses: u32, inputs: Vec<CellPath> },
}

/// A cell past checks 6 and 7: the role is built, the span still at hand for
/// check 8's cycle report.
struct ResolvedCell {
    path: CellPath,
    span: Span,
    value_type: ValueType,
    role: CellRole,
}

/// The two kinds inside the M0 slice, as their own type — downstream checks
/// match on this and structurally cannot meet the other three kinds.
#[derive(Clone, Copy)]
enum M0Kind {
    Input,
    Derived,
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

// Check 4 — the M0 slice holds cells only; the other four entry kinds are
// M1/M2/M5 machinery, rejected at the entry's own span and path.
fn admitted_entry(entry: &Entry) -> Result<&Cell, Diagnostic> {
    match entry {
        Entry::Cell(cell) => Ok(cell),
        Entry::Gate(gate) => Err(slice_error(
            gate.span,
            &gate.path,
            "`gate` entries are outside the M0 slice",
        )),
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

// Check 5b — the slice's cell kinds are `input` and `derived` (M0.md §2);
// `recurrent` and `deferred` are M1, `fold` is M5-adjacent.
fn admitted_kind(cell: &Cell) -> Result<M0Kind, Diagnostic> {
    match cell.kind {
        CellKind::Input => Ok(M0Kind::Input),
        CellKind::Derived => Ok(M0Kind::Derived),
        CellKind::Recurrent | CellKind::Deferred | CellKind::Fold => Err(slice_error(
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
fn admitted_clauses(cell: &Cell, kind: M0Kind) -> Result<(), Diagnostic> {
    match kind {
        M0Kind::Input => input_clauses(cell),
        M0Kind::Derived => derived_clauses(cell),
    }
}

// The `input` row of the clause law — M0.md §2's "input (with init)": `init`
// is REQUIRED, and every other clause belongs to another kind or another
// milestone.
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
        (cell.reset_on_reopen, "reset_on_reopen"),
        (cell.gate.is_some(), "gate"),
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
        (cell.init.is_some(), "init"),
        (cell.depth.is_some(), "depth"),
        (cell.reset_on_reopen, "reset_on_reopen"),
        (cell.gate.is_some(), "gate"),
        (cell.combiner.is_some(), "combiner"),
        (cell.else_value.is_some(), "else"),
        (!cell.members.is_empty(), "members"),
    ];

    admitted_clause_set(cell, "a `derived` cell", &forbidden)
}

// The shared tail of the two clause rows: the FIRST present forbidden clause,
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
// cell's, segment by segment. The braces and the dots carry ONE hierarchy, so
// a cell written inside `scope App.panel` lives at `App.panel.…` and nowhere
// else.
//
// Two consequences are load-bearing elsewhere. A cell path EQUAL to its
// scope's is rejected HERE, because a proper prefix is strictly shorter — so a
// cell can never collide with its own scope, only with another scope's path.
// And DEPTH IS FREE: only the prefix is law, so `scope App { input
// App.print.text }` is legal with no `App.print` scope declared anywhere.
//
// The comparison is structural — `Segment`'s own equality, segment by segment,
// the same authority `CellPath` compares by — never rendered text.
fn contained(scope: &Scope, cell: &Cell) -> Result<(), Diagnostic> {
    let enclosing = scope.path.segments();
    let inner = cell.path.segments();

    if inner.len() > enclosing.len() && inner.starts_with(enclosing) {
        return Ok(());
    }

    Err(Diagnostic {
        class: DiagClass::CellOutsideScope,
        span: Some(cell.span),
        path: Some(cell.path.clone()),
        message: format!(
            "cell `{}` does not extend its scope `{}`",
            cell.path.render(),
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
    kind: M0Kind,
    value_type: ValueType,
) -> Result<PendingForm, Diagnostic> {
    match kind {
        M0Kind::Input => {
            let Some(literal) = &cell.init else {
                return Err(slice_error(
                    cell.span,
                    &cell.path,
                    "an `input` cell without `init` is outside the M0 slice",
                ));
            };

            let init = converted_init(cell, literal, value_type)?;

            Ok(PendingForm::Input { init })
        }
        M0Kind::Derived => {
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
fn converted_init(
    cell: &Cell,
    literal: &ValueLiteral,
    value_type: ValueType,
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
                "`init` of `{}` does not convert under `{}`",
                cell.path.render(),
                value_type.as_str()
            ),
        }),
    }
}

// Check 6 — link every derived's `inputs` paths to dense indices: rank order,
// then list order. Forward references resolve like any other — the whole point
// of linking after Pass A — and the diagnostic carries the MISSING path, with
// the reading cell's span.
fn linked_inputs(
    pending: &[PendingCell],
    index: &HashMap<CellPath, CellIndex>,
) -> Result<Vec<Vec<CellIndex>>, Diagnostic> {
    let mut links = Vec::with_capacity(pending.len());

    for cell in pending {
        let mut linked = Vec::new();

        if let PendingForm::Derived { inputs, .. } = &cell.form {
            for input in inputs {
                match index.get(input) {
                    Some(&target) => linked.push(target),
                    None => {
                        return Err(Diagnostic {
                            class: DiagClass::UnknownInputPath,
                            span: Some(cell.span),
                            path: Some(input.clone()),
                            message: format!(
                                "`{}` reads `{}`, which no cell declares",
                                cell.path.render(),
                                input.render()
                            ),
                        });
                    }
                }
            }
        }

        links.push(linked);
    }

    Ok(links)
}

// Check 7 — resolve every derived's handle through the provider, rank order.
// D5: resolvability = module table ∪ provider registry, and the module's
// behavior table is empty throughout M0, so the registry is the whole answer.
// The resolved `Rc` is kept in the role — a clone shares the executable.
fn resolved_cells(
    pending: Vec<PendingCell>,
    links: Vec<Vec<CellIndex>>,
    provider: &dyn BehaviorProvider,
) -> Result<Vec<ResolvedCell>, Diagnostic> {
    let mut resolved = Vec::with_capacity(pending.len());

    for (cell, linked) in pending.into_iter().zip(links) {
        let role = match cell.form {
            PendingForm::Input { init } => CellRole::Input { init },
            PendingForm::Derived { uses, .. } => {
                let Some(behavior) = provider.resolve(uses) else {
                    return Err(Diagnostic {
                        class: DiagClass::UnresolvableBehavior,
                        span: Some(cell.span),
                        path: Some(cell.path.clone()),
                        message: format!(
                            "`{}` uses B@{uses:x}, which neither the module table nor the \
                             provider defines",
                            cell.path.render()
                        ),
                    });
                };

                CellRole::Derived {
                    behavior,
                    inputs: linked,
                }
            }
        };

        resolved.push(ResolvedCell {
            path: cell.path,
            span: cell.span,
            value_type: cell.value_type,
            role,
        });
    }

    Ok(resolved)
}

// Check 8 and the static order, one computation — Kahn's algorithm over the
// derived→derived dependency graph with a RANK-ORDERED ready set (§13.10.2
// step 2's order, computed once at load). R64 rules the CARRIER — entry rank
// is what carries declaration order into the IR; the ready-set completion,
// how that rank composes with topology into §13.10.3's order, is FND-25 and
// is open. An edge to an `input` cell is never a cycle edge: an input is
// written, never evaluated, so nothing waits on it.
//
// The produced order is: every derived, topologically, ties broken by rank —
// rank-least first among the simultaneously-ready. If any derived never
// becomes ready, the leftover nodes contain a cycle and `derived_cycle`
// extracts one concrete witness.
fn static_order(resolved: &[ResolvedCell]) -> Result<Vec<CellIndex>, Diagnostic> {
    let mut derived_count = 0;
    let mut in_degree = vec![0usize; resolved.len()];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); resolved.len()];

    for (rank, cell) in resolved.iter().enumerate() {
        let CellRole::Derived { inputs, .. } = &cell.role else {
            continue;
        };

        derived_count += 1;

        for input in inputs {
            if matches!(resolved[input.0].role, CellRole::Derived { .. }) {
                in_degree[rank] += 1;
                dependents[input.0].push(rank);
            }
        }
    }

    let mut ready = BinaryHeap::new();

    for (rank, cell) in resolved.iter().enumerate() {
        if matches!(cell.role, CellRole::Derived { .. }) && in_degree[rank] == 0 {
            ready.push(Reverse(rank));
        }
    }

    let mut order = Vec::with_capacity(derived_count);

    while let Some(Reverse(rank)) = ready.pop() {
        order.push(CellIndex(rank));

        for &dependent in &dependents[rank] {
            in_degree[dependent] -= 1;

            if in_degree[dependent] == 0 {
                ready.push(Reverse(dependent));
            }
        }
    }

    if order.len() < derived_count {
        return Err(derived_cycle(resolved, &in_degree));
    }

    Ok(order)
}

// The cycle report (R21: a load error, reported with the cycle's member
// paths). ONE concrete cycle is extracted: walk derived→derived edges among
// the leftover (never-ready) nodes from the rank-least leftover until a node
// repeats; the repeat closes the cycle.
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

// The walk's step: the FIRST (list-order) input that is itself a leftover.
// Total on leftover nodes — a leftover's final in-degree counts exactly its
// leftover derived inputs, so one always exists. The self-step fallback fires
// only on a broken invariant, and then terminates the walk as a self-cycle
// rather than hanging or panicking inside an error path.
fn next_leftover_input(resolved: &[ResolvedCell], in_degree: &[usize], current: usize) -> usize {
    if let CellRole::Derived { inputs, .. } = &resolved[current].role {
        for input in inputs {
            if in_degree[input.0] > 0 {
                return input.0;
            }
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
        Effect, Gate, Member, MemberDriver, ReopenPair, Stream, StreamPolicy, Wire, WireTo,
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
            reset_on_reopen: false,
            init: None,
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
            CellRole::Derived { .. } => panic!("`{}` is an input cell", path(names).render()),
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

    // Check 4: the slice holds cells only. Each of the four other entry kinds
    // rejects at its own path.
    #[test]
    fn rejects_every_non_cell_entry_kind() {
        let entries = [
            Entry::Gate(Gate {
                path: path(&["App", "g"]),
                span: Span::new(0, 0),
                pred: 1,
                inputs: Vec::new(),
                guards: Vec::new(),
                gate_parent: None,
            }),
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

        for (entry, expected) in entries
            .into_iter()
            .zip(["App.g", "App.w", "App.e", "App.s"])
        {
            let diagnostic = failure(&scoped(entry), &stubs(&[1]));

            assert_eq!(diagnostic.class.id(), "outside_m0_slice");
            assert_eq!(rendered_path(&diagnostic).as_deref(), Some(expected));
        }
    }

    // Check 5b: `recurrent`, `deferred` and `fold` parse — the grammar admits
    // them — and the slice does not.
    #[test]
    fn rejects_every_out_of_slice_cell_kind() {
        for kind in [CellKind::Recurrent, CellKind::Deferred, CellKind::Fold] {
            let diagnostic = failure(
                &module(vec![scope(
                    &["App"],
                    vec![cell(kind, &["App", "x"], Prim::I64)],
                )]),
                &stubs(&[]),
            );

            assert_eq!(
                diagnostic.class.id(),
                "outside_m0_slice",
                "{}",
                kind.as_str()
            );
            assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.x"));
        }
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

    // Check 5d, the `input` row: every clause but `init` is forbidden, one
    // trigger per clause.
    #[test]
    fn rejects_every_forbidden_clause_on_an_input() {
        let mutations: [(&str, CellEdit); 8] = [
            ("uses", |cell| cell.uses = Some(1)),
            ("inputs", |cell| cell.inputs.push(path(&["App", "y"]))),
            ("depth", |cell| cell.depth = Some(1)),
            ("reset_on_reopen", |cell| cell.reset_on_reopen = true),
            ("gate", |cell| cell.gate = Some(path(&["App", "g"]))),
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

    // Check 5d, the `derived` row: everything but `uses` and `inputs` is
    // forbidden.
    #[test]
    fn rejects_every_forbidden_clause_on_a_derived() {
        let mutations: [(&str, CellEdit); 7] = [
            ("init", |cell| {
                cell.init = Some(ValueLiteral::Int("0".to_string()))
            }),
            ("depth", |cell| cell.depth = Some(1)),
            ("reset_on_reopen", |cell| cell.reset_on_reopen = true),
            ("gate", |cell| cell.gate = Some(path(&["App", "g"]))),
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
    // (F4), the loader rejects the slice — a `recurrent` cell and a keyed
    // path, straight from text.
    #[test]
    fn rejects_out_of_slice_text_end_to_end() {
        let recurrent = "module App {\n  types {}\n  graph {\n    scope App exposes [] effects [] {\n      recurrent App.total : i64 uses B@d2 inputs [App.total] depth 1 init 0\n    }\n  }\n  behaviors {}\n}\n";
        let tokens = ductus_ir::text::lexer::lex(recurrent).expect("the source lexes");
        let parsed = ductus_ir::text::parser::parse(tokens).expect("the source parses");
        let diagnostic = failure(&parsed, &stubs(&[0xd2]));

        assert_eq!(diagnostic.class.id(), "outside_m0_slice");
        assert_eq!(rendered_path(&diagnostic).as_deref(), Some("App.total"));

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
}
