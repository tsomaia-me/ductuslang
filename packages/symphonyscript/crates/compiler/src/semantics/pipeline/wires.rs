use std::collections::VecDeque;

use symphonyscript_core::{Count, WireKind};

use super::check::describe;
use super::core::{Core, Stage};
use super::cycles::strongly_connected;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::reason::Reason;
use super::wire_row::WireRow;
use crate::diagnostics::{Code, Diagnostic};
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{ItemRec, NONE, Op, WireRec};
use crate::semantics::reporting::grouped;

fn index(at: usize) -> u32 {
    u32::try_from(at).unwrap_or(u32::MAX)
}

type Pairs = Vec<(u32, usize, u32)>;

fn group(pairs: &[(u32, usize, u32)], row: u32, kind: usize) -> Vec<u32> {
    let from = pairs.partition_point(|pair| (pair.0, pair.1) < (row, kind));
    pairs
        .get(from..)
        .unwrap_or_default()
        .iter()
        .take_while(|pair| (pair.0, pair.1) == (row, kind))
        .map(|pair| pair.2)
        .collect()
}

impl Core<'_> {
    pub fn wire_rec(&self, item: u32) -> Result<WireRec, Internal> {
        match self.ir.item(item)? {
            ItemRec::Wire(wire) => Ok(*wire),
            ItemRec::Placement(_)
            | ItemRec::Bundle(_)
            | ItemRec::For(_)
            | ItemRec::When(_)
            | ItemRec::Line(_)
            | ItemRec::Paren(_) => Err(Internal::missed("IR wire")),
        }
    }

    pub fn charge_item(&mut self, file: u32, lo: u32, hi: u32) -> Result<bool, Internal> {
        if self.stop.is_some() {
            return Ok(false);
        }
        self.expanded += 1;
        let bound = self.limits.expanded_items;
        let span = self.span(file, lo, hi);
        if self.expanded > bound {
            let message = format!(
                "the expansion yields more than {} instances and wires",
                grouped(bound)
            );
            self.stop = Some((
                self.diagnostics.len(),
                Diagnostic::new(Code::ExpansionLimit, span, message),
            ));
            return Ok(false);
        }
        if self.expanded - 1 == self.limits.expanded_items_warning {
            let counted = format!(
                "the expansion yields more than {} instances and wires",
                grouped(self.limits.expanded_items_warning)
            );
            self.warn_span(span, "expanded_items", counted, bound);
        }
        Ok(true)
    }

    pub fn wire_row(&mut self, item: u32, file: u32, source: u32, env: u32) -> Stage<()> {
        CountedWalk::CoreExpandRow.step();
        let wire = self.wire_rec(item)?;
        let found = self
            .ir
            .name_text(wire.kind)
            .filter(|_| self.ir.named(wire.kind))
            .and_then(|name| self.vocabulary.wire_kind_named(name));
        let Some(kind) = found else {
            return Ok(Err(Declined(Reason::WireKind)));
        };
        if !self.charge_item(file, wire.lo, wire.hi)? {
            return Ok(Ok(()));
        }
        let chosen = self.ir.node(wire.destination)?.0 != Op::Name
            || !matches!(self.bound(wire.destination)?, DeclRef::Instance(_));
        self.wires.push(WireRow {
            kind,
            item,
            file,
            source,
            candidates: (0, 0),
            fixed: false,
            chosen,
            env,
        });
        Ok(Ok(()))
    }

    pub fn candidate_row(&self, node: u32) -> Result<u32, Internal> {
        Ok(match self.bind.get(self.local(node)?).copied() {
            Some(DeclRef::Instance(item)) => {
                self.row_of_item.get(item as usize).copied().unwrap_or(NONE)
            }
            Some(_) | None => NONE,
        })
    }

    pub fn resolve_destinations(&mut self) -> Result<(), Internal> {
        for at in 0..self.wires.len() {
            CountedWalk::CoreWireDestination.step();
            let row = *self.wires.get(at).ok_or(Internal::missed("wire row"))?;
            if row.chosen {
                continue;
            }
            let root = self.wire_rec(row.item)?.destination;
            let target = self.candidate_row(root)?;
            let first = index(self.candidates.len());
            if target != NONE {
                self.candidates.push(target);
            }
            let end = index(self.candidates.len());
            if let Some(slot) = self.wires.get_mut(at) {
                slot.candidates = (first, end);
                slot.fixed = target != NONE;
            }
        }
        Ok(())
    }

    pub fn wire_targets(&self, row: &WireRow) -> Vec<u32> {
        let mut targets: Vec<u32> = self
            .candidates
            .get(row.candidates.0 as usize..row.candidates.1 as usize)
            .unwrap_or_default()
            .to_vec();
        targets.sort_unstable();
        targets.dedup();
        targets
    }

    pub fn unknown_destination(&self, kind: WireKind) -> bool {
        self.wires
            .iter()
            .any(|row| row.kind == kind && row.candidates.0 == row.candidates.1)
    }

    fn wire_span(&self, wire: u32) -> Result<symphonyscript_core::Span, Internal> {
        let row = *self
            .wires
            .get(wire as usize)
            .ok_or(Internal::missed("wire row"))?;
        let rec = self.wire_rec(row.item)?;
        Ok(self.span(row.file, rec.lo, rec.hi))
    }

    fn wire_kind_name(&self, kind: WireKind) -> Result<String, Internal> {
        Ok(self
            .vocabulary
            .wire_kind(kind)
            .ok_or(Internal::missed("wire kind"))?
            .name
            .clone())
    }

    fn row_kind_name(&self, row: u32) -> Result<String, Internal> {
        let kind = self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?
            .kind;
        Ok(self
            .vocabulary
            .node_kind(kind)
            .ok_or(Internal::missed("node kind"))?
            .name
            .clone())
    }

    fn row_label(&self, row: u32) -> Result<String, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        if self.ir.named(record.name)
            && let Some(name) = self.ir.name(record.name)
        {
            return Ok(self.ir.symbols.text(name.sym).to_string());
        }
        Ok(format!("a `{}`", self.row_kind_name(row)?))
    }

    pub fn check_wire_counts(&mut self) -> Result<(), Internal> {
        let mut outgoing: Pairs = Vec::new();
        let mut incoming_any: Pairs = Vec::new();
        let mut incoming_fixed: Pairs = Vec::new();
        let vocabulary = self.vocabulary;
        for wire in 0..self.wires.len() {
            CountedWalk::CheckWireRow.step();
            let row = *self.wires.get(wire).ok_or(Internal::missed("wire row"))?;
            let wire = index(wire);
            outgoing.push((row.source, row.kind.index(), wire));
            let decl = vocabulary
                .wire_kind(row.kind)
                .ok_or(Internal::missed("wire kind"))?;
            let source_kind = self
                .instances
                .get(row.source as usize)
                .ok_or(Internal::missed("row"))?
                .kind;
            for target in self.wire_targets(&row) {
                CountedWalk::CheckWireCandidate.step();
                let target_kind = self
                    .instances
                    .get(target as usize)
                    .ok_or(Internal::missed("row"))?
                    .kind;
                if !decl.connects.contains(&(source_kind, target_kind)) {
                    let message = format!(
                        "a `{}` wire does not connect a `{}` to a `{}`",
                        decl.name,
                        self.row_kind_name(row.source)?,
                        self.row_kind_name(target)?
                    );
                    let span = self.wire_span(wire)?;
                    self.draw_wire(wire, Code::WireEndpoint, span, message);
                }
                incoming_any.push((target, row.kind.index(), wire));
                if row.fixed {
                    incoming_fixed.push((target, row.kind.index(), wire));
                }
            }
        }
        outgoing.sort_unstable();
        incoming_any.sort_unstable();
        incoming_fixed.sort_unstable();
        for row in 0..self.instances.len() {
            CountedWalk::CheckWireRow.step();
            let row = index(row);
            let kind = self
                .instances
                .get(row as usize)
                .ok_or(Internal::missed("row"))?
                .kind;
            let decl = vocabulary
                .node_kind(kind)
                .ok_or(Internal::missed("node kind"))?;
            for (wire_kind, count) in &decl.outgoing {
                let found = group(&outgoing, row, wire_kind.index());
                let known = !self.unknown_children.contains(&row);
                self.wire_bounds(
                    row,
                    *wire_kind,
                    *count,
                    &found,
                    known.then_some(&found[..]),
                    "outgoing",
                )?;
            }
            for (wire_kind, count) in &decl.incoming {
                let any = group(&incoming_any, row, wire_kind.index());
                let fixed = group(&incoming_fixed, row, wire_kind.index());
                let known = !self.unknown_wires && !self.unknown_destination(*wire_kind);
                self.wire_bounds(
                    row,
                    *wire_kind,
                    *count,
                    &any,
                    known.then_some(&fixed[..]),
                    "incoming",
                )?;
            }
        }
        Ok(())
    }

    fn wire_bounds(
        &mut self,
        row: u32,
        wire_kind: WireKind,
        count: Count,
        toward_max: &[u32],
        toward_min: Option<&[u32]>,
        direction: &str,
    ) -> Result<(), Internal> {
        let name = self.wire_kind_name(wire_kind)?;
        let kind = self.row_kind_name(row)?;
        if let Some(max) = count.max()
            && let Some(wire) = toward_max.get(max as usize).copied()
        {
            let span = self.wire_span(wire)?;
            self.draw_wire(
                wire,
                Code::CountExceeded,
                span,
                format!(
                    "a `{kind}` takes {} {direction} `{name}` wires",
                    describe(count)
                ),
            );
        }
        if let Some(toward_min) = toward_min
            && toward_min.len() < count.min() as usize
        {
            let span = self.row_span(row)?;
            self.draw(
                Code::CountBelowMinimum,
                span,
                format!(
                    "this `{kind}` has {} {direction} `{name}` wires, and it needs {}",
                    toward_min.len(),
                    describe(count)
                ),
            );
        }
        Ok(())
    }

    pub fn wire_edges(&self) -> Vec<Vec<u32>> {
        let mut edges: Vec<Vec<u32>> = vec![Vec::new(); self.instances.len()];
        for row in &self.wires {
            for target in self.wire_targets(row) {
                if let Some(out) = edges.get_mut(row.source as usize) {
                    out.push(target);
                }
            }
        }
        edges
    }

    pub fn check_wire_cycles(&mut self) -> Result<(), Internal> {
        if self.wires.is_empty() {
            return Ok(());
        }
        let edges = self.wire_edges();
        let components = strongly_connected(&edges);
        let mut component_of = vec![0usize; edges.len()];
        for (position, component) in components.iter().enumerate() {
            for member in component {
                CountedWalk::CheckWireCycle.step();
                if let Some(slot) = component_of.get_mut(*member as usize) {
                    *slot = position;
                }
            }
        }
        let mut held = vec![0u64; components.len()];
        let mut first: Vec<Option<(u32, Vec<u32>)>> = vec![None; components.len()];
        for (wire, row) in self.wires.iter().enumerate() {
            CountedWalk::CheckWireCycle.step();
            let home = component_of.get(row.source as usize).copied().unwrap_or(0);
            let inside: Vec<u32> = self
                .wire_targets(row)
                .into_iter()
                .filter(|target| component_of.get(*target as usize) == Some(&home))
                .collect();
            if inside.is_empty() {
                continue;
            }
            if let Some(slot) = held.get_mut(home) {
                *slot += 1;
            }
            let cyclic = self
                .vocabulary
                .wire_kind(row.kind)
                .ok_or(Internal::missed("wire kind"))?
                .cyclic;
            if let Some(slot) = first.get_mut(home)
                && !cyclic
                && slot.is_none()
            {
                *slot = Some((index(wire), inside));
            }
        }
        for (home, found) in first.into_iter().enumerate() {
            let Some((wire, targets)) = found else {
                continue;
            };
            let row = *self
                .wires
                .get(wire as usize)
                .ok_or(Internal::missed("wire row"))?;
            let path = cycle_path(&edges, &component_of, home, row.source, &targets)?;
            let names: Vec<String> = path
                .iter()
                .map(|member| self.row_label(*member))
                .collect::<Result<_, _>>()?;
            let count = held.get(home).copied().unwrap_or(0);
            let kind_name = self.wire_kind_name(row.kind)?;
            let held = if count == 1 {
                String::from("1 wire")
            } else {
                format!("{} wires", grouped(count))
            };
            let span = self.wire_span(wire)?;
            self.draw_wire(
                wire,
                Code::WireCycle,
                span,
                format!(
                    "this `{kind_name}` wire closes the cycle of wires {}, and its group of cycles holds {held}; `{kind_name}` wires form no cycles",
                    names.join(" → ")
                ),
            );
        }
        Ok(())
    }
}

fn cycle_path(
    edges: &[Vec<u32>],
    component_of: &[usize],
    home: usize,
    source: u32,
    targets: &[u32],
) -> Result<Vec<u32>, Internal> {
    let mut previous = vec![NONE; edges.len()];
    let mut queue = VecDeque::new();
    for target in targets {
        if let Some(slot) = previous.get_mut(*target as usize)
            && *slot == NONE
        {
            *slot = source;
            queue.push_back(*target);
        }
    }
    while let Some(node) = queue.pop_front() {
        CountedWalk::CheckWireCycle.step();
        if node == source {
            break;
        }
        for next in edges
            .get(node as usize)
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            if component_of.get(*next as usize) != Some(&home) {
                continue;
            }
            if let Some(slot) = previous.get_mut(*next as usize)
                && *slot == NONE
            {
                *slot = node;
                queue.push_back(*next);
            }
        }
    }
    let step = |at: u32| {
        previous
            .get(at as usize)
            .copied()
            .filter(|before| *before != NONE)
            .ok_or(Internal::missed("cycle step"))
    };
    let mut path = vec![source];
    let mut current = step(source)?;
    while current != source {
        path.push(current);
        current = step(current)?;
    }
    path.push(source);
    path.reverse();
    Ok(path)
}

impl Core<'_> {
    fn draw_wire(
        &mut self,
        wire: u32,
        code: Code,
        span: symphonyscript_core::Span,
        summary: impl Into<String>,
    ) {
        let env = self.wires.get(wire as usize).map_or(NONE, |row| row.env);
        let saved = self.enter_env(env);
        self.draw(code, span, summary);
        self.leave_env(saved);
    }
}
