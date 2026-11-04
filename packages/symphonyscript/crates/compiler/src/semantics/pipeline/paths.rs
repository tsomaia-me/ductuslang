use std::collections::HashMap;

use symphonyscript_core::{AttrDefault, Value};

use super::attr_read::AttrRead;
use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::reason::Reason;
use crate::diagnostics::{Code, Diagnostic};
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{ABSENT, AttrForm, ItemRec, NONE, Op};
use crate::semantics::operators::EvalTy;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum End {
    Row(u32),
    To,
    Attr(u32),
    Placeholder,
    Value,
}

impl Core<'_> {
    pub fn mark_field_receivers(&mut self) {
        let mut receivers = Vec::new();
        for (local, op) in self.ir.op.iter().enumerate() {
            if *op == Op::Field
                && let Some(receiver) = self.ir.a.get(local)
            {
                receivers.push(*receiver);
            }
        }
        receivers.sort_unstable();
        self.field_receivers = receivers;
    }

    pub fn is_field_receiver(&self, node: u32) -> bool {
        self.field_receivers.binary_search(&node).is_ok()
    }

    fn row_sym(&self, row: u32) -> Result<Option<u32>, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let placement = self.placement(record.item)?;
        if !self.ir.named(placement.as_name) {
            return Ok(None);
        }
        Ok(self.ir.name(placement.as_name).map(|name| name.sym.0))
    }

    fn row_name_span(&self, row: u32) -> Result<symphonyscript_core::Span, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let placement = self.placement(record.item)?;
        self.name_span(record.file, placement.as_name)
    }

    fn named_row_text(&self, row: u32) -> Result<String, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let placement = self.placement(record.item)?;
        Ok(self
            .ir
            .name_text(placement.as_name)
            .unwrap_or_default()
            .to_string())
    }

    pub fn index_children(&mut self) -> Result<(), Internal> {
        let mut parent = vec![NONE; self.instances.len()];
        let mut index = Vec::new();
        for row in 0..self.instances.len() {
            let record = *self.instances.get(row).ok_or(Internal::missed("row"))?;
            let row = u32::try_from(row).unwrap_or(u32::MAX);
            for child in record.children.0..record.children.1 {
                CountedWalk::CorePathStep.step();
                if let Some(slot) = parent.get_mut(child as usize) {
                    *slot = row;
                }
                if let Some(sym) = self.row_sym(child)? {
                    index.push((row, sym, child));
                }
            }
        }
        index.sort_unstable();
        self.child_index = index;
        for row in 0..self.instances.len() {
            let row = u32::try_from(row).unwrap_or(u32::MAX);
            let Some(sym) = self.row_sym(row)? else {
                continue;
            };
            let text = self.named_row_text(row)?;
            if let Some(owner) = self.variants.owner(&text) {
                let owner = self
                    .vocabulary
                    .enum_decl(owner)
                    .ok_or(Internal::missed("enum"))?
                    .name
                    .clone();
                let span = self.row_name_span(row)?;
                self.draw(
                    Code::VariantName,
                    span,
                    format!(
                        "`{text}` is a variant of the enum `{owner}` and is never a declared name"
                    ),
                );
                continue;
            }
            let mut body = parent.get(row as usize).copied().unwrap_or(NONE);
            let mut other = None;
            if let Some(earlier) = self.child_named(body, sym).filter(|earlier| *earlier < row) {
                other = Some(earlier);
            }
            if body != NONE {
                let owner = self
                    .instances
                    .get(body as usize)
                    .ok_or(Internal::missed("row"))?
                    .kind;
                let decl = self
                    .vocabulary
                    .node_kind(owner)
                    .ok_or(Internal::missed("node kind"))?;
                let text = self.named_row_text(row)?;
                if decl.attrs.iter().any(|attr| attr.name == text) {
                    let message = format!(
                        "`{text}` is an attribute of the parent's kind `{}`",
                        decl.name
                    );
                    let span = self.row_name_span(row)?;
                    self.draw(Code::ChildNameClash, span, message);
                }
            }
            while other.is_none() && body != NONE {
                body = parent.get(body as usize).copied().unwrap_or(NONE);
                if body != NONE {
                    other = self.child_named(body, sym);
                }
            }
            if let Some(other) = other {
                let text = self.named_row_text(row)?;
                let span = self.row_name_span(row)?;
                let earlier = self.row_name_span(other)?;
                self.draw_related(
                    Code::ChildNameClash,
                    span,
                    vec![(earlier, "the other name".to_string())],
                    format!(
                        "`{text}` names another child or wire of this body or of an enclosing one"
                    ),
                );
            }
        }
        Ok(())
    }

    fn child_named(&self, parent: u32, sym: u32) -> Option<u32> {
        if parent == NONE {
            return None;
        }
        CountedWalk::CorePathStep.step();
        let at = self
            .child_index
            .partition_point(|entry| (entry.0, entry.1) < (parent, sym));
        self.child_index
            .get(at)
            .filter(|entry| (entry.0, entry.1) == (parent, sym))
            .map(|entry| entry.2)
    }

    fn attr_source(&self, row: u32, attr: usize) -> Result<(u32, u32, u32), Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let placement = self.placement(record.item)?;
        let decl = self
            .vocabulary
            .node_kind(record.kind)
            .ok_or(Internal::missed("node kind"))?;
        if placement.default != NONE && decl.default_attr.map(|at| at.index()) == Some(attr) {
            let (lo, hi) = self.ir.span(placement.default)?;
            return Ok((placement.default, lo, hi));
        }
        let name = decl
            .attrs
            .get(attr)
            .ok_or(Internal::missed("attribute"))?
            .name
            .as_str();
        for at in placement.attrs.0..placement.attrs.1 {
            let entry = *self
                .ir
                .attr_entries
                .get(at as usize)
                .ok_or(Internal::missed("IR attribute"))?;
            if self.ir.name_text(entry.name) != Some(name) {
                continue;
            }
            let AttrForm::Value(root) = entry.form else {
                return Err(Internal::missed("a flag the coverage table leaves out"));
            };
            return Ok((root, entry.lo, entry.hi));
        }
        Ok((NONE, 0, 0))
    }

    fn attr_read(&mut self, row: u32, attr: usize, ty: EvalTy) -> Result<u32, Internal> {
        let key = (row, u32::try_from(attr).unwrap_or(u32::MAX));
        if let Some(found) = self.attr_index.get(&key) {
            return Ok(*found);
        }
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let (root, lo, hi) = self.attr_source(row, attr)?;
        let name = self
            .vocabulary
            .node_kind(record.kind)
            .and_then(|decl| decl.attrs.get(attr))
            .ok_or(Internal::missed("attribute"))?
            .name
            .clone();
        let label = format!("{}.{name}", self.named_row_text(row)?);
        let at = u32::try_from(self.attr_reads.len()).unwrap_or(u32::MAX);
        self.attr_reads.push(AttrRead {
            row,
            attr,
            root,
            ty,
            label,
            span: self.span(record.file, lo, hi),
            key: (record.file, lo),
            cyclic: false,
        });
        self.attr_index.insert(key, at);
        self.attr_values
            .push([None, None, None, None, None, None, None]);
        Ok(at)
    }

    fn step_row(&mut self, node: u32, row: u32, field: u32) -> Stage<End> {
        let entry = *self.ir.name(field).ok_or(Internal::missed("IR name"))?;
        if let Some(child) = self.child_named(row, entry.sym.0) {
            return Ok(Ok(End::Row(child)));
        }
        let kind = self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?
            .kind;
        let decl = self
            .vocabulary
            .node_kind(kind)
            .ok_or(Internal::missed("node kind"))?;
        let text = self.ir.symbols.text(entry.sym).to_string();
        let Some(attr) = decl.attrs.iter().position(|attr| attr.name == text) else {
            if self.may_name_any(row)? || self.names_unexpanded_child(row, entry.sym.0)? {
                return Ok(Ok(End::Placeholder));
            }
            let span = self.span(self.file_of(node), entry.lo, entry.hi);
            let owner = self.row_owner(row)?;
            self.draw(
                Code::UnknownAttribute,
                span,
                format!("{owner} has no child, wire or attribute `{text}`"),
            );
            return Ok(Ok(End::Value));
        };
        let declared = decl.attrs.get(attr).ok_or(Internal::missed("attribute"))?;
        let Some(ty) = super::check::expected(declared) else {
            return Ok(Err(Declined(super::check::attribute_reason(declared))));
        };
        Ok(Ok(End::Attr(self.attr_read(row, attr, ty)?)))
    }

    fn path_end(&mut self, node: u32, ends: &HashMap<u32, End>) -> Stage<End> {
        let (_, receiver, field, missing) = self.ir.node(node)?;
        if missing == 1 {
            return Ok(Ok(End::Placeholder));
        }
        let (op, _, _, _) = self.ir.node(receiver)?;
        let start = if op == Op::Name {
            match self.bound(receiver)? {
                DeclRef::Instance(item) => {
                    match self.row_of_item.get(item as usize).copied().unwrap_or(NONE) {
                        NONE => End::Placeholder,
                        row => End::Row(row),
                    }
                }
                DeclRef::None
                | DeclRef::Duplicate
                | DeclRef::Let(_)
                | DeclRef::Cell(_)
                | DeclRef::Param(_)
                | DeclRef::Fn(_) => {
                    return Ok(Err(Declined(Reason::Member)));
                }
            }
        } else if op == Op::To {
            if self.to_wire(receiver).is_some() {
                End::To
            } else {
                End::Placeholder
            }
        } else if op == Op::Field {
            *ends
                .get(&receiver)
                .ok_or(Internal::missed("a path's receiver"))?
        } else {
            return Ok(Err(Declined(Reason::Member)));
        };
        match start {
            End::Row(row) => self.step_row(node, row, field),
            End::To if self.is_field_receiver(node) => Ok(Ok(End::To)),
            End::To => self.destination_path_end(node),
            End::Attr(_) => {
                let text = self.ir.name_text(field).unwrap_or_default().to_string();
                self.draw_at(
                    Code::UnknownAttribute,
                    node,
                    format!(
                        "`.{text}` reads an attribute of an instance or a wire, and this is a value"
                    ),
                )?;
                Ok(Ok(End::Value))
            }
            End::Placeholder | End::Value => Ok(Ok(End::Placeholder)),
        }
    }

    pub fn resolve_paths(&mut self) -> Stage<()> {
        self.index_children()?;
        self.index_to_wires()?;
        self.misplaced_to()?;
        let mut ends: HashMap<u32, End> = HashMap::new();
        let mut held: HashMap<u32, Vec<Diagnostic>> = HashMap::new();
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Field)
                || !self.reached.get(local).copied().unwrap_or(false)
            {
                continue;
            }
            CountedWalk::CorePathStep.step();
            let node = self.ir.base + u32::try_from(local).unwrap_or(u32::MAX);
            let before = self.diagnostics.len();
            let end = match self.path_end(node, &ends)? {
                Ok(end) => end,
                Err(declined) => return Ok(Err(declined)),
            };
            self.keep_body_faults(node, before, &mut held)?;
            if !self.is_field_receiver(node) {
                match end {
                    End::Row(row) => {
                        self.path_rows.insert(node, row);
                    }
                    End::Attr(at) => {
                        if let Some(slot) = self.paths.get_mut(local) {
                            *slot = at;
                        }
                    }
                    End::Placeholder | End::Value | End::To => {}
                }
            }
            ends.insert(node, end);
        }
        let mut left: Vec<(u32, Vec<Diagnostic>)> = held.into_iter().collect();
        left.sort_by_key(|(node, _)| *node);
        for (_, faults) in left {
            self.diagnostics.extend(faults);
        }
        if !self.attr_reads.is_empty() && self.recursive()? {
            return Ok(Err(Declined(Reason::Recursion)));
        }
        Ok(Ok(()))
    }

    fn keep_body_faults(
        &mut self,
        node: u32,
        before: usize,
        held: &mut HashMap<u32, Vec<Diagnostic>>,
    ) -> Result<(), Internal> {
        let drawn = self
            .diagnostics
            .split_off(before.min(self.diagnostics.len()));
        if self.is_field_receiver(node) {
            if !drawn.is_empty() {
                held.insert(node, drawn);
            }
            return Ok(());
        }
        let mut faults = Vec::new();
        let mut current = node;
        while let Ok((Op::Field, receiver, _, _)) = self.ir.node(current) {
            if let Some(inner) = held.remove(&receiver) {
                faults.splice(0..0, inner);
            }
            current = receiver;
        }
        faults.extend(drawn);
        if faults.is_empty() {
            return Ok(());
        }
        if self.function_owner(node)? == NONE {
            self.diagnostics.extend(faults);
        } else {
            self.body_path_faults.insert(node, (faults, false));
        }
        Ok(())
    }

    pub fn draw_body_path_faults(&mut self, node: u32) {
        let Some((faults, drawn)) = self.body_path_faults.get_mut(&node) else {
            return;
        };
        *drawn = true;
        for diagnostic in faults.clone() {
            let diagnostic = self.chained(diagnostic);
            self.push_drawn(diagnostic);
        }
    }

    pub fn unreached_body_path_faults(&mut self) {
        let mut left: Vec<(u32, Vec<Diagnostic>)> = self
            .body_path_faults
            .iter()
            .filter(|(_, (_, drawn))| !*drawn)
            .map(|(node, (faults, _))| (*node, faults.clone()))
            .collect();
        left.sort_by_key(|(node, _)| *node);
        for (_, faults) in left {
            self.diagnostics.extend(faults);
        }
    }

    pub fn read_root(&self, node: u32) -> Result<Option<u32>, Internal> {
        Ok(self
            .path_read(node)?
            .filter(|read| !read.cyclic && read.root != NONE)
            .map(|read| read.root))
    }

    pub fn bare_cell_read(&self, node: u32) -> Result<Option<u32>, Internal> {
        let Some(root) = self.read_root(node)? else {
            return Ok(None);
        };
        if self.ir.node(root)?.0 != Op::Name {
            return Ok(None);
        }
        Ok(match self.bound(root)? {
            DeclRef::Cell(at) => Some(at),
            DeclRef::None
            | DeclRef::Instance(_)
            | DeclRef::Duplicate
            | DeclRef::Let(_)
            | DeclRef::Param(_)
            | DeclRef::Fn(_) => None,
        })
    }

    pub fn path_read(&self, node: u32) -> Result<Option<&AttrRead>, Internal> {
        let at = self.paths.get(self.local(node)?).copied().unwrap_or(NONE);
        Ok(self.attr_reads.get(at as usize))
    }
}

impl Core<'_> {
    pub fn default_read(&self, read: &AttrRead) -> Result<CoreValue, Internal> {
        let kind = self
            .instances
            .get(read.row as usize)
            .ok_or(Internal::missed("row"))?
            .kind;
        let default = self
            .vocabulary
            .node_kind(kind)
            .and_then(|decl| decl.attrs.get(read.attr))
            .and_then(|attr| attr.default.clone());
        let Some(AttrDefault::Elements(values)) = default else {
            return Ok(CoreValue::Error);
        };
        Ok(match values.as_slice() {
            [Value::U8(value)] => CoreValue::Int(i128::from(*value)),
            [Value::I32(value)] => CoreValue::Int(i128::from(*value)),
            [Value::I64(value)] => CoreValue::Int(i128::from(*value)),
            [Value::F32(value)] => CoreValue::F32(*value),
            [Value::F64(value)] => CoreValue::F64(*value),
            [Value::Bool(value)] => CoreValue::Bool(*value),
            _ => CoreValue::Error,
        })
    }
}

impl Core<'_> {
    pub fn type_attr_reads(&mut self) -> Stage<()> {
        for at in self.attr_order.clone() {
            let Some(read) = self.attr_reads.get(at as usize) else {
                continue;
            };
            if read.cyclic || read.root == NONE {
                continue;
            }
            if let Err(declined) = self.ensure_typed(read.root)? {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    fn index_to_wires(&mut self) -> Result<(), Internal> {
        let mut rows: HashMap<u32, u32> = HashMap::new();
        for (at, row) in self.wires.iter().enumerate() {
            rows.insert(row.item, u32::try_from(at).unwrap_or(u32::MAX));
        }
        let mut found = Vec::new();
        for (item, rec) in self.ir.items.iter().enumerate() {
            let ItemRec::Wire(wire) = rec else {
                continue;
            };
            let item = u32::try_from(item).unwrap_or(u32::MAX);
            if self.ir.dropped(item) {
                continue;
            }
            let mut roots = vec![wire.default];
            let entries = self
                .ir
                .attr_entries
                .get(wire.attrs.0 as usize..wire.attrs.1 as usize)
                .unwrap_or_default();
            for entry in entries {
                if let AttrForm::Value(value) = entry.form {
                    roots.push(value);
                }
            }
            let wire_at = rows.get(&item).copied().unwrap_or(NONE);
            for root in roots.into_iter().filter(|root| *root != NONE) {
                for node in self.deep_postorder(root)? {
                    let mut current = node;
                    loop {
                        CountedWalk::CorePathStep.step();
                        let (op, receiver, _, _) = self.ir.node(current)?;
                        if op == Op::Field {
                            current = receiver;
                            continue;
                        }
                        if op == Op::To {
                            found.push((current, wire_at));
                        }
                        break;
                    }
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        self.to_wires = found;
        Ok(())
    }

    fn to_wire(&self, node: u32) -> Option<u32> {
        let at = self.to_wires.partition_point(|entry| entry.0 < node);
        self.to_wires
            .get(at)
            .filter(|entry| entry.0 == node)
            .map(|entry| entry.1)
    }

    fn misplaced_to(&mut self) -> Result<(), Internal> {
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::To)
                || !self.reached.get(local).copied().unwrap_or(false)
            {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(u32::MAX);
            if self.to_wire(node).is_none() {
                self.draw_at(
                    Code::MisplacedTo,
                    node,
                    "`to` is written only in a wire placement's `/expr`, gate and attribute clause",
                )?;
            } else if !self.is_field_receiver(node) {
                self.draw_at(
                    Code::MisplacedTo,
                    node,
                    "`to` starts a path that ends at an attribute, as in `to.gain`",
                )?;
            }
        }
        Ok(())
    }

    fn row_owner(&self, row: u32) -> Result<String, Internal> {
        let kind = self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?
            .kind;
        let decl = self
            .vocabulary
            .node_kind(kind)
            .ok_or(Internal::missed("node kind"))?;
        Ok(format!(
            "the `{}` named `{}`",
            decl.name,
            self.named_row_text(row)?
        ))
    }

    fn segments_to_wire(&self, node: u32) -> Result<(Vec<u32>, u32), Internal> {
        let mut segments = Vec::new();
        let mut current = node;
        loop {
            CountedWalk::CorePathStep.step();
            let (op, receiver, field, _) = self.ir.node(current)?;
            if op != Op::Field {
                break;
            }
            segments.push(field);
            current = receiver;
        }
        segments.reverse();
        let wire = self
            .to_wire(current)
            .ok_or(Internal::missed("a `to` path's wire"))?;
        Ok((segments, wire))
    }

    fn destination_path_end(&mut self, node: u32) -> Stage<End> {
        let (segments, wire) = self.segments_to_wire(node)?;
        if wire == NONE {
            return Ok(Ok(End::Placeholder));
        }
        let row = *self
            .wires
            .get(wire as usize)
            .ok_or(Internal::missed("wire row"))?;
        if row.chosen {
            self.deferred_to.push(node);
            return Ok(Ok(End::Placeholder));
        }
        let targets = self.wire_targets(&row);
        if targets.is_empty() {
            return Ok(Ok(End::Placeholder));
        }
        self.end_over_candidates(node, &segments, &targets)
    }

    pub fn resolve_chosen_paths(&mut self) -> Stage<()> {
        let mut deferred = std::mem::take(&mut self.deferred_to);
        deferred.sort_unstable();
        for node in deferred {
            let (segments, wire) = self.segments_to_wire(node)?;
            let row = *self
                .wires
                .get(wire as usize)
                .ok_or(Internal::missed("wire row"))?;
            let targets = self
                .candidates
                .get(row.candidates.0 as usize..row.candidates.1 as usize)
                .unwrap_or_default()
                .to_vec();
            if targets.is_empty() {
                continue;
            }
            let end = match self.end_over_candidates(node, &segments, &targets)? {
                Ok(end) => end,
                Err(declined) => return Ok(Err(declined)),
            };
            let End::Attr(at) = end else {
                continue;
            };
            let local = self.local(node)?;
            if let Some(slot) = self.paths.get_mut(local) {
                *slot = at;
            }
            let read = self
                .attr_reads
                .get(at as usize)
                .ok_or(Internal::missed("attribute read"))?;
            if !read.cyclic
                && read.root != NONE
                && let Err(declined) = self.ensure_typed(read.root)?
            {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }

    fn end_over_candidates(&mut self, node: u32, segments: &[u32], targets: &[u32]) -> Stage<End> {
        let several = targets.len() > 1;
        let mut found = Vec::with_capacity(targets.len());
        for target in targets {
            CountedWalk::CorePathStep.step();
            match self.candidate_attr(node, segments, *target, several)? {
                Ok(Ok(attr)) => found.push(attr),
                Ok(Err(end)) => return Ok(Ok(end)),
                Err(declined) => return Ok(Err(declined)),
            }
        }
        let Some((row, attr, ty)) = found.first().copied() else {
            return Ok(Ok(End::Placeholder));
        };
        if !several {
            return Ok(Ok(End::Attr(self.attr_read(row, attr, ty)?)));
        }
        if let Some((other, _, _)) = found.iter().find(|(_, _, own)| *own != ty).copied() {
            let name = self
                .vocabulary
                .node_kind(
                    self.instances
                        .get(row as usize)
                        .ok_or(Internal::missed("row"))?
                        .kind,
                )
                .and_then(|decl| decl.attrs.get(attr))
                .ok_or(Internal::missed("attribute"))?
                .name
                .clone();
            let (one, other) = (self.row_owner(row)?, self.row_owner(other)?);
            self.draw_at(
                Code::ToCandidates,
                node,
                format!(
                    "the destination candidates hold `{name}` with different types or unit families, {one} and {other} among them, and a `to` read takes one"
                ),
            )?;
            return Ok(Ok(End::Value));
        }
        self.to_choices.insert(node, ty);
        Ok(Ok(End::Value))
    }

    fn candidate_attr(
        &mut self,
        node: u32,
        segments: &[u32],
        start: u32,
        several: bool,
    ) -> Stage<Result<(u32, usize, EvalTy), End>> {
        let mut target = start;
        let text = segments
            .iter()
            .map(|segment| self.ir.name_text(*segment).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(".");
        let file = self.file_of(node);
        let (last, steps) = segments
            .split_last()
            .ok_or(Internal::missed("a `to` path's segment"))?;
        let missing = if several {
            Code::ToCandidates
        } else {
            Code::UnknownAttribute
        };
        for step in steps {
            let entry = *self.ir.name(*step).ok_or(Internal::missed("IR name"))?;
            if let Some(child) = self.child_named(target, entry.sym.0) {
                target = child;
                continue;
            }
            if self.may_name_any(target)? {
                return Ok(Ok(Err(End::Placeholder)));
            }
            let owner = self.row_owner(target)?;
            let span = self.span(file, entry.lo, entry.hi);
            let name = self.ir.symbols.text(entry.sym).to_string();
            self.draw(
                missing,
                span,
                format!(
                    "{owner} has no named child, wire or pattern use `{name}`, so `to.{text}` reads nothing there"
                ),
            );
            return Ok(Ok(Err(End::Value)));
        }
        let entry = *self.ir.name(*last).ok_or(Internal::missed("IR name"))?;
        let span = self.span(file, entry.lo, entry.hi);
        if self.child_named(target, entry.sym.0).is_some() {
            self.draw(
                Code::MisplacedTo,
                span,
                format!(
                    "`to.{text}` ends at an instance or a wire; a `to` path ends at an attribute"
                ),
            );
            return Ok(Ok(Err(End::Value)));
        }
        let kind = self
            .instances
            .get(target as usize)
            .ok_or(Internal::missed("row"))?
            .kind;
        let decl = self
            .vocabulary
            .node_kind(kind)
            .ok_or(Internal::missed("node kind"))?;
        let name = self.ir.symbols.text(entry.sym).to_string();
        let Some(attr) = decl.attrs.iter().position(|attr| attr.name == name) else {
            if self.may_name_any(target)? {
                return Ok(Ok(Err(End::Placeholder)));
            }
            let owner = self.row_owner(target)?;
            let code = if steps.is_empty() {
                Code::UnknownAttribute
            } else {
                missing
            };
            self.draw(
                code,
                span,
                format!("the destination, {owner}, has no attribute `{name}`"),
            );
            return Ok(Ok(Err(End::Value)));
        };
        let declared = decl.attrs.get(attr).ok_or(Internal::missed("attribute"))?;
        let Some(ty) = super::check::expected(declared) else {
            return Ok(Err(Declined(super::check::attribute_reason(declared))));
        };
        Ok(Ok(Ok((target, attr, ty))))
    }
}

impl Core<'_> {
    fn names_unexpanded_child(&self, row: u32, sym: u32) -> Result<bool, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let placement = self.placement(record.item)?;
        let Some(block) = self.ir.blocks.get(placement.body as usize) else {
            return Ok(false);
        };
        let mut items: Vec<u32> = self
            .ir
            .block_items
            .get(block.items.0 as usize..block.items.1 as usize)
            .unwrap_or_default()
            .to_vec();
        while let Some(item) = items.pop() {
            match *self.ir.item(item)? {
                ItemRec::Placement(child) => {
                    if self.ir.named(child.as_name)
                        && self
                            .ir
                            .name(child.as_name)
                            .is_some_and(|name| name.sym.0 == sym)
                    {
                        return Ok(true);
                    }
                }
                ItemRec::Line(members) => items.extend(
                    self.ir
                        .members
                        .get(members.0 as usize..members.1 as usize)
                        .unwrap_or_default(),
                ),
                ItemRec::Wire(_)
                | ItemRec::Bundle(_)
                | ItemRec::For(_)
                | ItemRec::When(_)
                | ItemRec::Paren(_) => {}
            }
        }
        Ok(false)
    }

    fn may_name_any(&self, row: u32) -> Result<bool, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let placement = self.placement(record.item)?;
        if placement.body == NONE || placement.body == ABSENT {
            return Ok(false);
        }
        Ok(self
            .ir
            .blocks
            .get(placement.body as usize)
            .is_some_and(|block| {
                block.items.0 == block.items.1 && block.members.0 == block.members.1
            }))
    }
}
