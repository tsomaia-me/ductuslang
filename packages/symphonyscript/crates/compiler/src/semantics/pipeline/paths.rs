use std::collections::HashMap;

use symphonyscript_core::{AttrDefault, Value};

use super::attr_read::AttrRead;
use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::folds::ANY_LENGTH;
use super::leaf::Leaf;
use super::reason::Reason;
use crate::diagnostics::{Code, Diagnostic};
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{ABSENT, AttrForm, ItemRec, NONE, Op};
use crate::semantics::operators::EvalTy;

fn earliest(
    one: symphonyscript_core::Span,
    other: Option<symphonyscript_core::Span>,
) -> symphonyscript_core::Span {
    match other {
        Some(other) if other.file == one.file && other.range.start < one.range.start => other,
        Some(_) | None => one,
    }
}

fn same_end(one: (End, Option<EvalTy>), other: (End, Option<EvalTy>)) -> bool {
    let unknown = |(end, ty): (End, Option<EvalTy>)| {
        ty.is_none() && matches!(end, End::Value | End::Placeholder)
    };
    unknown(one)
        || unknown(other)
        || (std::mem::discriminant(&one.0) == std::mem::discriminant(&other.0) && one.1 == other.1)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum End {
    Row(u32),
    Use(u32),
    To,
    Attr(u32),
    Wire(u32),
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
        if !record.reachable || !self.ir.named(record.name) {
            return Ok(None);
        }
        Ok(self.ir.name(record.name).map(|name| name.sym.0))
    }

    pub fn row_name_span(&self, row: u32) -> Result<symphonyscript_core::Span, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        self.name_span(record.name_file, record.name)
    }

    fn named_row_text(&self, row: u32) -> Result<String, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        Ok(self
            .ir
            .name_text(record.name)
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
        self.index_wire_names()?;
        self.use_index.sort_unstable();
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
            let own = self.row_name_span(row)?;
            let mut wired = self.wire_named_before(body, sym, &own)?;
            while other.is_none() && wired.is_none() && body != NONE {
                body = parent.get(body as usize).copied().unwrap_or(NONE);
                if body != NONE {
                    other = self.child_named(body, sym);
                    if other.is_none() {
                        wired = self
                            .wire_named(body, sym)
                            .map(|wire| self.wire_name_span(wire))
                            .transpose()?;
                    }
                }
            }
            let held = self.placeholder_named(parent.get(row as usize).copied(), sym, row);
            let held = match (held, wired) {
                (Some(held), wired) => Some(earliest(held, wired)),
                (None, wired) => wired,
            };
            let earlier = match other {
                Some(other) => Some(earliest(self.row_name_span(other)?, held)),
                None => held,
            };
            if let Some(earlier) = earlier {
                let text = self.named_row_text(row)?;
                let span = self.row_name_span(row)?;
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
        self.placeholder_clashes(&parent)?;
        self.wire_clashes(&parent)
    }

    fn placeholder_named(
        &self,
        body: Option<u32>,
        sym: u32,
        before: u32,
    ) -> Option<symphonyscript_core::Span> {
        let body = body.filter(|body| *body != NONE)?;
        self.placeholder_uses
            .iter()
            .find(|held| {
                held.0 == body
                    && held.3 <= before
                    && self.ir.name(held.1).is_some_and(|entry| entry.sym.0 == sym)
            })
            .map(|held| held.2.clone())
    }

    fn placeholder_clashes(&mut self, parent: &[u32]) -> Result<(), Internal> {
        let held = self.placeholder_uses.clone();
        for (at, (body, name, span, key)) in held.iter().enumerate() {
            let entry = *self.ir.name(*name).ok_or(Internal::missed("IR name"))?;
            let text = self.ir.symbols.text(entry.sym).to_string();
            let owner = self
                .instances
                .get(*body as usize)
                .ok_or(Internal::missed("row"))?
                .kind;
            let decl = self
                .vocabulary
                .node_kind(owner)
                .ok_or(Internal::missed("node kind"))?;
            if decl.attrs.iter().any(|attr| attr.name == text) {
                let message = format!(
                    "`{text}` is an attribute of the parent's kind `{}`",
                    decl.name
                );
                self.draw(Code::ChildNameClash, span.clone(), message);
            }
            let mut earlier = self
                .child_named(*body, entry.sym.0)
                .filter(|row| row < key)
                .map(|row| self.row_name_span(row))
                .transpose()?;
            let before = held
                .iter()
                .take(at)
                .find(|other| {
                    other.0 == *body
                        && self
                            .ir
                            .name(other.1)
                            .is_some_and(|found| found.sym == entry.sym)
                })
                .map(|other| other.2.clone());
            earlier = match earlier {
                Some(row) => Some(earliest(row, before)),
                None => before,
            };
            let mut up = *body;
            while earlier.is_none() && up != NONE {
                up = parent.get(up as usize).copied().unwrap_or(NONE);
                if up != NONE {
                    earlier = self
                        .child_named(up, entry.sym.0)
                        .map(|row| self.row_name_span(row))
                        .transpose()?;
                }
            }
            if let Some(earlier) = earlier {
                self.draw_related(
                    Code::ChildNameClash,
                    span.clone(),
                    vec![(earlier, "the other name".to_string())],
                    format!(
                        "`{text}` names another child or wire of this body or of an enclosing one"
                    ),
                );
            }
        }
        Ok(())
    }

    pub fn child_named(&self, parent: u32, sym: u32) -> Option<u32> {
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
        let flagged = decl.attrs.get(attr).and_then(|attr| attr.flag);
        for at in placement.flags.0..placement.flags.1 {
            let flag = *self
                .ir
                .flags
                .get(at as usize)
                .ok_or(Internal::missed("IR flag"))?;
            if Some(flag.flag) == flagged {
                return Ok((ABSENT, flag.lo, flag.hi));
            }
        }
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
                return Ok((ABSENT, entry.lo, entry.hi));
            };
            return Ok((root, entry.lo, entry.hi));
        }
        Ok((NONE, 0, 0))
    }

    pub fn attr_read(&mut self, row: u32, attr: usize, ty: EvalTy) -> Stage<u32> {
        let key = (row, u32::try_from(attr).unwrap_or(u32::MAX));
        if let Some(found) = self.attr_index.get(&key) {
            return Ok(Ok(*found));
        }
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let (root, lo, hi) = self.attr_source(row, attr)?;
        let set = if root == ABSENT {
            let placement = self.placement(record.item)?;
            let declared = self
                .vocabulary
                .node_kind(record.kind)
                .and_then(|decl| decl.attrs.get(attr))
                .ok_or(Internal::missed("attribute"))?;
            Some(self.set_value(placement.flags, placement.attrs, declared)?)
        } else {
            None
        };
        let root = if set.is_some() { NONE } else { root };
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
            wire: false,
            set,
        });
        self.attr_index.insert(key, at);
        self.attr_values
            .push([None, None, None, None, None, None, None]);
        Ok(Ok(at))
    }

    fn step_row(&mut self, node: u32, row: u32, field: u32) -> Stage<End> {
        let entry = *self.ir.name(field).ok_or(Internal::missed("IR name"))?;
        if let Some(child) = self.child_named(row, entry.sym.0) {
            return Ok(Ok(End::Row(child)));
        }
        if let Some(child) = self.use_child_named(row, entry.sym)? {
            return Ok(Ok(End::Row(child)));
        }
        if let Some(env) = self.use_named(row, entry.sym.0) {
            return Ok(Ok(End::Use(env)));
        }
        if let Some(wire) = self.wire_named(row, entry.sym.0) {
            if self
                .wires
                .get(wire as usize)
                .is_none_or(|found| found.env != NONE)
            {
                return Ok(Err(Declined(Reason::PatternUse)));
            }
            return Ok(Ok(End::Wire(wire)));
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
            if self.may_name_any(row)?
                || self.names_unexpanded_child(row, entry.sym.0)?
                || self.skipped_top().binary_search(&entry.sym.0).is_ok()
            {
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
        let read_ty = if let Some(unit) = declared.unit {
            if self.unit_scale(unit.family, unit.default_unit)? != 1.0 {
                return Ok(Err(Declined(Reason::UnitAttribute)));
            }
            super::check::plain_expected(declared)
        } else {
            super::check::expected(declared)
        };
        let Some(ty) = read_ty else {
            return Ok(Err(Declined(super::check::attribute_reason(declared))));
        };
        match self.attr_read(row, attr, ty)? {
            Ok(read) => Ok(Ok(End::Attr(read))),
            Err(declined) => Ok(Err(declined)),
        }
    }

    fn step_wire(&mut self, node: u32, wire: u32, field: u32) -> Stage<End> {
        let entry = *self.ir.name(field).ok_or(Internal::missed("IR name"))?;
        let kind = self
            .wires
            .get(wire as usize)
            .ok_or(Internal::missed("wire row"))?
            .kind;
        let decl = self
            .vocabulary
            .wire_kind(kind)
            .ok_or(Internal::missed("wire kind"))?;
        let text = self.ir.symbols.text(entry.sym).to_string();
        let Some(attr) = decl.attrs.iter().position(|attr| attr.name == text) else {
            let span = self.span(self.file_of(node), entry.lo, entry.hi);
            let owner = decl.name.clone();
            self.draw(
                Code::UnknownAttribute,
                span,
                format!("the `{owner}` wire has no child, wire or attribute `{text}`"),
            );
            return Ok(Ok(End::Value));
        };
        let declared = decl.attrs.get(attr).ok_or(Internal::missed("attribute"))?;
        let read_ty = if let Some(unit) = declared.unit {
            if self.unit_scale(unit.family, unit.default_unit)? != 1.0 {
                return Ok(Err(Declined(Reason::UnitAttribute)));
            }
            super::check::plain_expected(declared)
        } else {
            super::check::expected(declared)
        };
        let Some(ty) = read_ty else {
            return Ok(Err(Declined(super::check::attribute_reason(declared))));
        };
        match self.wire_attr_read(wire, attr, ty)? {
            Ok(read) => Ok(Ok(End::Attr(read))),
            Err(declined) => Ok(Err(declined)),
        }
    }

    fn path_end(&mut self, node: u32, ends: &HashMap<u32, End>) -> Stage<End> {
        let (_, receiver, field, missing) = self.ir.node(node)?;
        if missing == 1 {
            return Ok(Ok(End::Placeholder));
        }
        let (op, _, _, _) = self.ir.node(receiver)?;
        let start = if op == Op::Name {
            match self.bound(receiver)? {
                DeclRef::Instance(item) if matches!(self.ir.item(item)?, ItemRec::Wire(_)) => {
                    let mut rows = (0u32..)
                        .zip(self.wires.iter())
                        .filter(|(_, row)| row.item == item)
                        .map(|(at, _)| at);
                    match (rows.next(), rows.next()) {
                        (Some(wire), None) => End::Wire(wire),
                        (None, _) => End::Placeholder,
                        (Some(_), Some(_)) => return Ok(Err(Declined(Reason::Member))),
                    }
                }
                DeclRef::Instance(item) => {
                    match self.row_of_item.get(item as usize).copied().unwrap_or(NONE) {
                        NONE => self
                            .use_names
                            .get(&(item, self.path_env))
                            .map_or(End::Placeholder, |env| End::Use(*env)),
                        row => End::Row(row),
                    }
                }
                DeclRef::Param(position) => {
                    let owner = self.binding_owner(receiver)?;
                    if self.pattern_at(owner).is_some() {
                        return self.use_param_path_end(node, owner, position, field);
                    }
                    return self.loop_name_path_end(node, receiver, position, field);
                }
                DeclRef::Let(at) => match self.let_reference_row(at, node, field)? {
                    Ok(row) => row,
                    Err(declined) => return Ok(Err(declined)),
                },
                DeclRef::Cell(at) => match self.cell_member(at, node, field)? {
                    Ok(end) => end,
                    Err(declined) => return Ok(Err(declined)),
                },
                DeclRef::None if self.bare.contains(&receiver) || self.pi.contains(&receiver) => {
                    return Ok(Err(Declined(Reason::Member)));
                }
                DeclRef::None | DeclRef::Duplicate | DeclRef::Fn(_) => End::Placeholder,
                DeclRef::Pattern(_) => {
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
            if self.per_inst_paths.contains(&receiver) {
                return Ok(Err(Declined(Reason::Member)));
            }
            if self.array_members.contains_key(&receiver) || self.cell_sizes.contains_key(&receiver)
            {
                let text = self.ir.name_text(field).unwrap_or_default().to_string();
                self.draw_at(
                    Code::UnknownAttribute,
                    node,
                    format!(
                        "`.{text}` reads an attribute of an instance or a wire, and this is a value"
                    ),
                )?;
                return Ok(Ok(End::Value));
            }
            *ends
                .get(&receiver)
                .ok_or(Internal::missed("a path's receiver"))?
        } else if let Some(count) = match self.literal_array_length(receiver, field)? {
            Some(count) => Some(count),
            None => self.element_array_length(receiver, field)?,
        } {
            self.array_members.insert(node, i128::from(count));
            End::Value
        } else if matches!(
            self.ir.name_text(field).unwrap_or_default(),
            "size" | "capacity"
        ) {
            self.expr_members.insert(node);
            End::Value
        } else {
            return Ok(Err(Declined(Reason::Member)));
        };
        match start {
            End::Row(row) => {
                let end = match self.step_row(node, row, field)? {
                    Ok(end) => end,
                    Err(declined) => return Ok(Err(declined)),
                };
                Ok(Ok(end))
            }
            End::Use(env) => self.step_use(node, env, field),
            End::Wire(wire) => self.step_wire(node, wire, field),
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
        if self.group_use_named()? {
            return Ok(Err(Declined(Reason::PatternUse)));
        }
        if let Err(declined) = self.bind_reference_args()? {
            return Ok(Err(declined));
        }
        self.index_to_wires()?;
        self.misplaced_to()?;
        for (wire, env) in std::mem::take(&mut self.unrun_to_wires) {
            let links = self.env_links(env);
            let saved = std::mem::replace(&mut self.use_links, links);
            let outcome = self.unrun_to_paths(wire);
            self.use_links = saved;
            if let Err(declined) = outcome? {
                return Ok(Err(declined));
            }
        }
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
            if self.use_local(node)?.is_some() {
                continue;
            }
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
                    End::Wire(_) => return Ok(Err(Declined(Reason::Member))),
                    End::Use(_) => return Ok(Err(Declined(Reason::PatternUse))),
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
        if let Err(declined) = self.resolve_use_paths()? {
            return Ok(Err(declined));
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
            | DeclRef::Fn(_)
            | DeclRef::Pattern(_) => None,
        })
    }

    pub fn read_at(&self, node: u32) -> Result<u32, Internal> {
        if !self.env_reads.is_empty() {
            for (env, _, _) in self.own_calls.iter().rev() {
                if let Some(at) = self.env_reads.get(&(node, *env)) {
                    return Ok(*at);
                }
            }
        }
        if let Some(inst) = self.path_inst(node)
            && let Some(at) = self.inst_reads.get(&(node, inst))
        {
            return Ok(*at);
        }
        let at = self.paths.get(self.local(node)?).copied().unwrap_or(NONE);
        if self.wire_now == NONE {
            return Ok(at);
        }
        Ok(self
            .row_reads
            .get(&(node, self.wire_now))
            .copied()
            .unwrap_or(at))
    }

    pub fn node_reads(&self, node: u32) -> Result<Vec<u32>, Internal> {
        let mut reads: Vec<u32> = self
            .row_reads
            .range((node, 0)..=(node, u32::MAX))
            .map(|(_, at)| *at)
            .chain(
                self.choice_reads
                    .range((node, 0)..=(node, u32::MAX))
                    .map(|(_, at)| *at),
            )
            .chain(
                self.env_reads
                    .range((node, 0)..=(node, u32::MAX))
                    .map(|(_, at)| *at),
            )
            .chain(
                self.inst_reads
                    .range((node, 0)..=(node, u32::MAX))
                    .map(|(_, at)| *at),
            )
            .collect();
        let at = self.paths.get(self.local(node)?).copied().unwrap_or(NONE);
        if at != NONE && !reads.contains(&at) {
            reads.insert(0, at);
        }
        Ok(reads)
    }

    pub fn to_choice(&self, node: u32) -> Option<EvalTy> {
        self.row_choices
            .get(&(node, self.wire_now))
            .or_else(|| self.to_choices.get(&node))
            .copied()
    }

    pub fn path_read(&self, node: u32) -> Result<Option<&AttrRead>, Internal> {
        Ok(self.attr_reads.get(self.read_at(node)? as usize))
    }
}

impl Core<'_> {
    pub fn default_read(&self, read: &AttrRead) -> Result<CoreValue, Internal> {
        if let Some(value) = read.set {
            return Ok(CoreValue::Bool(value));
        }
        let default = if read.wire {
            let kind = self
                .wires
                .get(read.row as usize)
                .ok_or(Internal::missed("wire row"))?
                .kind;
            self.vocabulary
                .wire_kind(kind)
                .and_then(|decl| decl.attrs.get(read.attr))
                .and_then(|attr| attr.default.clone())
        } else {
            let kind = self
                .instances
                .get(read.row as usize)
                .ok_or(Internal::missed("row"))?
                .kind;
            self.vocabulary
                .node_kind(kind)
                .and_then(|decl| decl.attrs.get(read.attr))
                .and_then(|attr| attr.default.clone())
        };
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
    fn reads_a_chosen_path(&self, root: u32) -> Result<bool, Internal> {
        if self.deferred_to.is_empty() {
            return Ok(false);
        }
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![root];
        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            for node in self.deep_postorder(current)? {
                let (op, _, _, _) = self.ir.node(node)?;
                if op == Op::Field {
                    if self.deferred_to.iter().any(|(path, _)| *path == node) {
                        return Ok(true);
                    }
                    if let Some(read) = self.path_read(node)?
                        && !read.cyclic
                        && read.root != NONE
                    {
                        stack.push(read.root);
                    }
                }
                if op == Op::Name
                    && let DeclRef::Let(at) = self.bound(node)?
                {
                    stack.push(self.let_root(at)?);
                }
            }
        }
        Ok(false)
    }

    pub fn type_attr_reads(&mut self) -> Stage<()> {
        for at in self.attr_order.clone() {
            let Some(read) = self.attr_reads.get(at as usize).cloned() else {
                continue;
            };
            if read.cyclic || read.root == NONE {
                continue;
            }
            if self.reads_a_chosen_path(read.root)? {
                self.untyped_reads.push(at);
                continue;
            }
            if let Err(declined) = self.typed_in_env(&read)? {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    fn index_to_wires(&mut self) -> Result<(), Internal> {
        let mut rows: HashMap<u32, Vec<u32>> = HashMap::new();
        for (at, row) in self.wires.iter().enumerate() {
            rows.entry(row.item)
                .or_default()
                .push(u32::try_from(at).unwrap_or(u32::MAX));
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
            let mut roots = vec![wire.default, wire.gate];
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
            let wire_rows = rows.get(&item).cloned().unwrap_or_else(|| vec![NONE]);
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
                            found.extend(wire_rows.iter().map(|wire| (current, *wire)));
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

    pub fn row_owner(&self, row: u32) -> Result<String, Internal> {
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

    fn to_rows(&self, node: u32) -> Vec<u32> {
        let at = self.to_wires.partition_point(|entry| entry.0 < node);
        self.to_wires
            .get(at..)
            .unwrap_or_default()
            .iter()
            .take_while(|entry| entry.0 == node)
            .map(|entry| entry.1)
            .collect()
    }

    fn segments_to_wire(&self, node: u32) -> Result<(Vec<u32>, Vec<u32>), Internal> {
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
        let wires = self.to_rows(current);
        if wires.is_empty() {
            return Err(Internal::missed("a `to` path's wire"));
        }
        Ok((segments, wires))
    }

    fn destination_path_end(&mut self, node: u32) -> Stage<End> {
        let (segments, wires) = self.segments_to_wire(node)?;
        if let [wire] = wires.as_slice() {
            return self.row_path_end(node, &segments, *wire);
        }
        let mut first: Option<(End, Option<EvalTy>)> = None;
        for wire in wires {
            self.wire_now = wire;
            let end = self.row_path_end(node, &segments, wire);
            self.wire_now = NONE;
            let end = match end? {
                Ok(end) => end,
                Err(declined) => return Ok(Err(declined)),
            };
            if let End::Attr(at) = end {
                self.row_reads.insert((node, wire), at);
            }
            let seen = (end, self.row_end_ty(node, wire, end));
            match first {
                Some(earlier) if !same_end(earlier, seen) => {
                    return Ok(Err(Declined(Reason::BodyItem)));
                }
                Some((End::Attr(_), _)) => {}
                Some(_) | None if seen.1.is_some() || first.is_none() => first = Some(seen),
                Some(_) | None => {}
            }
        }
        Ok(Ok(first.map_or(End::Placeholder, |(end, _)| end)))
    }

    fn row_end_ty(&self, node: u32, wire: u32, end: End) -> Option<EvalTy> {
        match end {
            End::Attr(at) => self.attr_reads.get(at as usize).map(|read| read.ty),
            End::Value | End::Placeholder | End::Row(_) | End::Use(_) | End::To | End::Wire(_) => {
                self.row_choices.get(&(node, wire)).copied()
            }
        }
    }

    fn row_path_end(&mut self, node: u32, segments: &[u32], wire: u32) -> Stage<End> {
        if wire == NONE {
            return Ok(Ok(End::Placeholder));
        }
        let row = *self
            .wires
            .get(wire as usize)
            .ok_or(Internal::missed("wire row"))?;
        let targets = if row.chosen {
            let destination = self.wire_rec(row.item)?.destination;
            let Some(targets) = self.runtime_targets(destination)? else {
                self.deferred_to.push((node, wire));
                return Ok(Ok(End::Placeholder));
            };
            self.runtime_to.insert(node);
            targets
        } else {
            self.wire_targets(&row)
        };
        if targets.is_empty() {
            return Ok(Ok(End::Placeholder));
        }
        let links = self.env_links(row.env);
        let saved = std::mem::replace(&mut self.use_links, links);
        let end = self.end_over_candidates(node, segments, &targets);
        self.use_links = saved;
        end
    }

    pub fn resolve_chosen_paths(&mut self) -> Stage<()> {
        let mut deferred = std::mem::take(&mut self.deferred_to);
        deferred.sort_unstable();
        let mut seen: HashMap<u32, (End, Option<EvalTy>)> = HashMap::new();
        let (mut resolved, mut chose) = (Vec::new(), false);
        for (node, wire) in deferred {
            let (segments, wires) = self.segments_to_wire(node)?;
            let several = wires.len() > 1;
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
            chose = true;
            if several {
                self.wire_now = wire;
            }
            let links = self.env_links(row.env);
            let saved = std::mem::replace(&mut self.use_links, links);
            let end = self.end_over_candidates(node, &segments, &targets);
            self.use_links = saved;
            self.wire_now = NONE;
            let end = match end? {
                Ok(end) => end,
                Err(declined) => return Ok(Err(declined)),
            };
            if several {
                if let End::Attr(at) = end {
                    self.row_reads.insert((node, wire), at);
                }
                let found = (end, self.row_end_ty(node, wire, end));
                match seen.get(&node).copied() {
                    None => {
                        seen.insert(node, found);
                    }
                    Some(earlier) if same_end(earlier, found) => {}
                    Some(_) => return Ok(Err(Declined(Reason::BodyItem))),
                }
            }
            let End::Attr(at) = end else {
                continue;
            };
            let local = self.local(node)?;
            if let Some(slot) = self.paths.get_mut(local)
                && *slot == NONE
            {
                *slot = at;
            }
            resolved.push(at);
        }
        if chose {
            self.chosen_destination_cycles()?;
        }
        resolved.extend(std::mem::take(&mut self.untyped_reads));
        for at in resolved {
            let read = self
                .attr_reads
                .get(at as usize)
                .cloned()
                .ok_or(Internal::missed("attribute read"))?;
            if !read.cyclic
                && read.root != NONE
                && let Err(declined) = self.typed_in_env(&read)?
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
            return match self.attr_read(row, attr, ty)? {
                Ok(read) => Ok(Ok(End::Attr(read))),
                Err(declined) => Ok(Err(declined)),
            };
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
        for (row, attr, own) in found {
            match self.attr_read(row, attr, own)? {
                Ok(read) => {
                    self.choice_reads.insert((node, read));
                }
                Err(declined) => return Ok(Err(declined)),
            }
        }
        self.to_choices.insert(node, ty);
        if self.wire_now != NONE {
            self.row_choices.insert((node, self.wire_now), ty);
        }
        Ok(Ok(End::Value))
    }

    pub fn candidate_attr(
        &mut self,
        node: u32,
        segments: &[u32],
        start: u32,
        several: bool,
    ) -> Stage<Result<(u32, usize, EvalTy), End>> {
        let mut target = End::Row(start);
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
            match self.to_step(target, entry.sym)? {
                Some(next @ (End::Row(_) | End::Use(_))) => {
                    target = next;
                    continue;
                }
                Some(End::Wire(wire)) if !several => {
                    target = End::Wire(wire);
                    continue;
                }
                Some(_) => return Ok(Ok(Err(End::Placeholder))),
                None => {}
            }
            let owner = match target {
                End::Wire(_) => return Ok(Err(Declined(Reason::Member))),
                End::Row(row) if self.may_name_any(row)? => {
                    return Ok(Ok(Err(End::Placeholder)));
                }
                End::Row(row) => self.row_owner(row)?,
                End::Use(env) => format!("the use of `{}`", self.use_pattern_name(env)?),
                End::To | End::Attr(_) | End::Placeholder | End::Value => {
                    return Err(Internal::missed("a `to` path's step"));
                }
            };
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
        if self.to_step(target, entry.sym)?.is_some() {
            self.draw(
                Code::MisplacedTo,
                span,
                format!(
                    "`to.{text}` ends at an instance or a wire; a `to` path ends at an attribute"
                ),
            );
            return Ok(Ok(Err(End::Value)));
        }
        let target = match target {
            End::Row(row) => row,
            End::Wire(wire) => {
                let kind = self
                    .wires
                    .get(wire as usize)
                    .ok_or(Internal::missed("wire row"))?
                    .kind;
                let decl = self
                    .vocabulary
                    .wire_kind(kind)
                    .ok_or(Internal::missed("wire kind"))?;
                let name = self.ir.symbols.text(entry.sym).to_string();
                if decl.attrs.iter().any(|attr| attr.name == name) {
                    return Ok(self.step_wire(node, wire, *last)?.map(Err));
                }
                let owner = decl.name.clone();
                self.draw(
                    missing,
                    span,
                    format!("the destination, the `{owner}` wire, has no attribute `{name}`"),
                );
                return Ok(Ok(Err(End::Value)));
            }
            End::Use(env) => {
                let owner = self.use_pattern_name(env)?;
                let name = self.ir.symbols.text(entry.sym).to_string();
                self.draw(
                    missing,
                    span,
                    format!("the destination, the use of `{owner}`, has no attribute `{name}`"),
                );
                return Ok(Ok(Err(End::Value)));
            }
            End::To | End::Attr(_) | End::Placeholder | End::Value => {
                return Err(Internal::missed("a `to` path's end"));
            }
        };
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
        let read_ty = if let Some(unit) = declared.unit {
            if self.unit_scale(unit.family, unit.default_unit)? != 1.0 {
                return Ok(Err(Declined(Reason::UnitAttribute)));
            }
            super::check::plain_expected(declared)
        } else {
            super::check::expected(declared)
        };
        let Some(ty) = read_ty else {
            return Ok(Err(Declined(super::check::attribute_reason(declared))));
        };
        Ok(Ok(Ok((target, attr, ty))))
    }

    pub fn to_step(
        &self,
        target: End,
        sym: crate::semantics::ir::flat::Sym,
    ) -> Result<Option<End>, Internal> {
        match target {
            End::Row(row) => {
                if let Some(child) = self.child_named(row, sym.0) {
                    return Ok(Some(End::Row(child)));
                }
                if let Some(child) = self.use_child_named(row, sym)? {
                    return Ok(Some(End::Row(child)));
                }
                if let Some(wire) = self.wire_named(row, sym.0) {
                    return Ok(Some(End::Wire(wire)));
                }
                Ok(self.use_named(row, sym.0).map(End::Use))
            }
            End::Use(env) => {
                CountedWalk::CorePathStep.step();
                self.use_item_named(env, sym)
            }
            End::To | End::Attr(_) | End::Placeholder | End::Value | End::Wire(_) => Ok(None),
        }
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
                ItemRec::When(_) => items.extend(self.when_items(item)),
                ItemRec::Paren(inner) if inner != NONE => items.push(inner),
                ItemRec::Bundle(_) => items.extend(self.bundle_items(item)),
                ItemRec::Wire(_) | ItemRec::For(_) | ItemRec::Paren(_) => {}
            }
        }
        Ok(false)
    }

    pub fn may_name_any(&self, row: u32) -> Result<bool, Internal> {
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

impl Core<'_> {
    fn loop_name_path_end(
        &mut self,
        node: u32,
        receiver: u32,
        position: u32,
        field: u32,
    ) -> Stage<End> {
        let owner = self.binding_owner(receiver)?;
        let insts = self.owner_insts.get(&owner).cloned().unwrap_or_default();
        if self.loop_at(owner).is_none() || insts.is_empty() {
            return Ok(Err(Declined(Reason::Member)));
        }
        self.per_inst_paths.insert(node);
        let mut first = None;
        for inst in insts {
            let Some((held, row)) = self.inst_row.get(&inst).copied() else {
                return Ok(Err(Declined(Reason::Member)));
            };
            if held != position {
                return Ok(Err(Declined(Reason::Member)));
            }
            let end = match self.step_row(node, row, field)? {
                Ok(end) => end,
                Err(declined) => return Ok(Err(declined)),
            };
            match end {
                End::Attr(at) => {
                    self.inst_reads.insert((node, inst), at);
                }
                End::Row(_) | End::Use(_) | End::To | End::Wire(_) => {
                    return Ok(Err(Declined(Reason::Member)));
                }
                End::Value | End::Placeholder => {}
            }
            if first.is_none()
                || matches!(end, End::Attr(_)) && !matches!(first, Some(End::Attr(_)))
            {
                first = Some(end);
            }
        }
        Ok(Ok(first.unwrap_or(End::Placeholder)))
    }
}

impl Core<'_> {
    pub fn read_env(&self, read: &AttrRead) -> Result<u32, Internal> {
        let env = if read.wire {
            self.wires
                .get(read.row as usize)
                .ok_or(Internal::missed("wire row"))?
                .env
        } else {
            self.instances
                .get(read.row as usize)
                .ok_or(Internal::missed("row"))?
                .env
        };
        if env == NONE || read.root == NONE || read.cyclic {
            return Ok(NONE);
        }
        if self
            .body_of
            .get(self.local(read.root)?)
            .copied()
            .unwrap_or(NONE)
            == NONE
        {
            return Ok(NONE);
        }
        Ok(env)
    }

    pub fn typed_in_env(&mut self, read: &AttrRead) -> Stage<Fact> {
        let env = self.read_env(read)?;
        if env == NONE {
            return self.ensure_typed(read.root);
        }
        let saved = self.enter_env(env);
        let typed = self.ensure_typed(read.root);
        self.leave_env(saved);
        typed
    }
}

impl Core<'_> {
    fn use_local(&self, node: u32) -> Result<Option<u32>, Internal> {
        if self.pattern_items.is_empty() {
            return Ok(None);
        }
        let mut current = node;
        while let (Op::Field, receiver, _, _) = self.ir.node(current)? {
            current = receiver;
        }
        if self.ir.node(current)?.0 != Op::Name {
            return Ok(None);
        }
        Ok(match self.bound(current)? {
            DeclRef::Instance(item) => self.pattern_items.get(&item).copied(),
            DeclRef::None
            | DeclRef::Let(_)
            | DeclRef::Cell(_)
            | DeclRef::Param(_)
            | DeclRef::Duplicate
            | DeclRef::Fn(_)
            | DeclRef::Pattern(_) => None,
        })
    }

    fn resolve_use_paths(&mut self) -> Stage<()> {
        let mut nodes = Vec::new();
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Field)
                || !self.reached.get(local).copied().unwrap_or(false)
            {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(u32::MAX);
            if let Some(pattern) = self.use_local(node)? {
                nodes.push((pattern, node));
            }
        }
        if nodes.is_empty() {
            return Ok(Ok(()));
        }
        let rows = self.env_rows.clone();
        let mut patterns: Vec<u32> = nodes.iter().map(|(pattern, _)| *pattern).collect();
        patterns.sort_unstable();
        patterns.dedup();
        for pattern in patterns {
            let owner = self.patterns_start() + pattern;
            let items: Vec<u32> = self
                .pattern_items
                .iter()
                .filter(|(_, at)| **at == pattern)
                .map(|(item, _)| *item)
                .collect();
            for env in 0..u32::try_from(self.envs.len()).unwrap_or(0) {
                let Some(record) = self.envs.get(env as usize).copied() else {
                    continue;
                };
                let owned = self
                    .bodies
                    .get(record.inst as usize)
                    .is_some_and(|body| body.function == owner);
                if record.use_item == NONE || !owned {
                    continue;
                }
                CountedWalk::CorePathStep.step();
                let mut saved = Vec::new();
                for item in &items {
                    let row = rows.get(&(*item, env)).copied().unwrap_or(NONE);
                    if let Some(slot) = self.row_of_item.get_mut(*item as usize) {
                        saved.push((*item, std::mem::replace(slot, row)));
                    }
                }
                let links = self.enter_env(env);
                self.path_env = env;
                let outcome = self.use_paths_in(pattern, env, &nodes);
                self.path_env = NONE;
                self.leave_env(links);
                for (item, row) in saved {
                    if let Some(slot) = self.row_of_item.get_mut(item as usize) {
                        *slot = row;
                    }
                }
                if let Err(declined) = outcome? {
                    return Ok(Err(declined));
                }
            }
        }
        Ok(Ok(()))
    }

    fn use_paths_in(&mut self, pattern: u32, env: u32, nodes: &[(u32, u32)]) -> Stage<()> {
        let mut ends: HashMap<u32, End> = HashMap::new();
        for (at, node) in nodes {
            if *at != pattern {
                continue;
            }
            let end = match self.path_end(*node, &ends)? {
                Ok(end) => end,
                Err(declined) => return Ok(Err(declined)),
            };
            if !self.is_field_receiver(*node) {
                match end {
                    End::Row(row) => {
                        self.env_path_rows.insert((*node, env), row);
                    }
                    End::Wire(_) => return Ok(Err(Declined(Reason::Member))),
                    End::Use(_) => return Ok(Err(Declined(Reason::PatternUse))),
                    End::Attr(read) => {
                        self.env_reads.insert((*node, env), read);
                    }
                    End::Placeholder | End::Value | End::To => {}
                }
            }
            ends.insert(*node, end);
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    fn use_child_named(
        &self,
        row: u32,
        sym: crate::semantics::ir::flat::Sym,
    ) -> Result<Option<u32>, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        if record.env == NONE || self.pattern_items.is_empty() {
            return Ok(None);
        }
        for child in record.children.0..record.children.1 {
            let found = *self
                .instances
                .get(child as usize)
                .ok_or(Internal::missed("row"))?;
            if found.env != NONE
                && self.ir.named(found.name)
                && self.ir.name(found.name).is_some_and(|name| name.sym == sym)
            {
                return Ok(Some(child));
            }
        }
        Ok(None)
    }
}

impl Core<'_> {
    fn use_named(&self, row: u32, sym: u32) -> Option<u32> {
        if self.use_index.is_empty() {
            return None;
        }
        let at = self
            .use_index
            .partition_point(|entry| (entry.0, entry.1) < (row, sym));
        self.use_index
            .get(at)
            .filter(|entry| (entry.0, entry.1) == (row, sym))
            .map(|entry| entry.2)
    }

    fn use_pattern(&self, env: u32) -> Result<u32, Internal> {
        let record = *self
            .envs
            .get(env as usize)
            .ok_or(Internal::missed("use env"))?;
        self.bodies
            .get(record.inst as usize)
            .and_then(|body| self.pattern_at(body.function))
            .ok_or(Internal::missed("a use's pattern"))
    }

    fn use_pattern_name(&self, env: u32) -> Result<String, Internal> {
        let pattern = self.use_pattern(env)?;
        let name = self
            .ir
            .patterns
            .get(pattern as usize)
            .ok_or(Internal::missed("IR pattern"))?
            .name;
        Ok(self.ir.name_text(name).unwrap_or_default().to_string())
    }

    fn use_item_named(
        &self,
        env: u32,
        sym: crate::semantics::ir::flat::Sym,
    ) -> Result<Option<End>, Internal> {
        let body = self.pattern_parts(self.use_pattern(env)?)?.1;
        let block = *self
            .ir
            .blocks
            .get(body as usize)
            .ok_or(Internal::missed("IR block"))?;
        let mut items: Vec<u32> = self
            .ir
            .block_items
            .get(block.items.0 as usize..block.items.1 as usize)
            .unwrap_or_default()
            .to_vec();
        while let Some(item) = items.pop() {
            match *self.ir.item(item)? {
                ItemRec::Placement(child) => {
                    if !self.ir.named(child.as_name)
                        || self
                            .ir
                            .name(child.as_name)
                            .is_none_or(|name| name.sym != sym)
                    {
                        continue;
                    }
                    if let Some(row) = self.env_rows.get(&(item, env)) {
                        return Ok(Some(End::Row(*row)));
                    }
                    if let Some(inner) = self.use_names.get(&(item, env)) {
                        return Ok(Some(End::Use(*inner)));
                    }
                    return Ok(Some(End::Placeholder));
                }
                ItemRec::Line(members) => items.extend(
                    self.ir
                        .members
                        .get(members.0 as usize..members.1 as usize)
                        .unwrap_or_default(),
                ),
                ItemRec::When(_) => items.extend(self.when_items(item)),
                ItemRec::Paren(inner) if inner != NONE => items.push(inner),
                ItemRec::Bundle(_) => items.extend(self.bundle_items(item)),
                ItemRec::Wire(_) | ItemRec::For(_) | ItemRec::Paren(_) => {}
            }
        }
        Ok(None)
    }

    fn step_use(&mut self, node: u32, env: u32, field: u32) -> Stage<End> {
        CountedWalk::CorePathStep.step();
        let entry = *self.ir.name(field).ok_or(Internal::missed("IR name"))?;
        if let Some(end) = self.use_item_named(env, entry.sym)? {
            return Ok(Ok(end));
        }
        let owner = self.use_pattern_name(env)?;
        let text = self.ir.symbols.text(entry.sym).to_string();
        let span = self.span(self.file_of(node), entry.lo, entry.hi);
        self.draw(
            Code::UnknownAttribute,
            span,
            format!("the use of `{owner}` has no child, wire or attribute `{text}`"),
        );
        Ok(Ok(End::Value))
    }

    pub fn group_use_named(&self) -> Result<bool, Internal> {
        if self.group_items.is_empty() {
            return Ok(false);
        }
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Name)
                || !self.reached.get(local).copied().unwrap_or(false)
            {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(u32::MAX);
            if let Some(DeclRef::Instance(item)) = self.bind.get(local)
                && self.group_items.contains(item)
                && !self.is_field_receiver(node)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[derive(Clone, Copy)]
enum Written {
    Item(u32, u32),
    Use(u32),
    Attr,
    Value,
}

impl Core<'_> {
    pub fn written_paths(&mut self, pattern: u32) -> Result<(), Internal> {
        let mut ends: HashMap<u32, Written> = HashMap::new();
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Field) {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(u32::MAX);
            if self.use_local(node)? != Some(pattern) {
                continue;
            }
            CountedWalk::CorePathStep.step();
            let (_, receiver, field, missing) = self.ir.node(node)?;
            let start = if missing == 1 {
                Written::Value
            } else if self.ir.node(receiver)?.0 == Op::Field {
                ends.get(&receiver).copied().unwrap_or(Written::Value)
            } else if let DeclRef::Instance(item) = self.bound(receiver)? {
                self.written_item(item)?
            } else {
                Written::Value
            };
            let end = match start {
                Written::Item(named, kind_item) => {
                    self.written_step(node, field, named, kind_item)?
                }
                Written::Use(used) => self.written_use_step(node, field, used)?,
                Written::Attr => {
                    let text = self.ir.name_text(field).unwrap_or_default().to_string();
                    self.draw_at(
                        Code::UnknownAttribute,
                        node,
                        format!(
                            "`.{text}` reads an attribute of an instance or a wire, and this is a value"
                        ),
                    )?;
                    Written::Value
                }
                Written::Value => Written::Value,
            };
            ends.insert(node, end);
        }
        Ok(())
    }

    fn written_item(&self, item: u32) -> Result<Written, Internal> {
        let kind = self.placement(item)?.kind;
        if let Some(used) = self.used_pattern(kind) {
            if self.cyclic_pattern(used) {
                return Ok(Written::Value);
            }
            return Ok(match self.stand_in(used)? {
                Ok(Some(inner)) if !self.duplicated_kind(inner)? => Written::Item(item, inner),
                Ok(Some(_)) | Err(_) => Written::Value,
                Ok(None) => Written::Use(used),
            });
        }
        Ok(if self.written_kind(item)?.is_some() {
            Written::Item(item, item)
        } else {
            Written::Value
        })
    }

    fn written_kind(&self, item: u32) -> Result<Option<symphonyscript_core::NodeKind>, Internal> {
        let kind = self.placement(item)?.kind;
        if !self.ir.named(kind) {
            return Ok(None);
        }
        let entry = *self.ir.name(kind).ok_or(Internal::missed("IR name"))?;
        if self
            .decl
            .get(entry.sym.0 as usize)
            .is_some_and(|decl| *decl != DeclRef::None)
        {
            return Ok(None);
        }
        Ok(self
            .vocabulary
            .node_kind_named(self.ir.symbols.text(entry.sym)))
    }

    fn written_child(&self, block: u32, sym: u32) -> Result<Option<u32>, Internal> {
        let Some(found) = self.ir.blocks.get(block as usize) else {
            return Ok(None);
        };
        let mut items: Vec<u32> = self
            .ir
            .block_items
            .get(found.items.0 as usize..found.items.1 as usize)
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
                        return Ok(Some(item));
                    }
                }
                ItemRec::Line(members) => items.extend(
                    self.ir
                        .members
                        .get(members.0 as usize..members.1 as usize)
                        .unwrap_or_default(),
                ),
                ItemRec::When(_) => items.extend(self.when_items(item)),
                ItemRec::Paren(inner) if inner != NONE => items.push(inner),
                ItemRec::Bundle(_) => items.extend(self.bundle_items(item)),
                ItemRec::Wire(_) | ItemRec::For(_) | ItemRec::Paren(_) => {}
            }
        }
        Ok(None)
    }

    fn written_step(
        &mut self,
        node: u32,
        field: u32,
        named: u32,
        kind_item: u32,
    ) -> Result<Written, Internal> {
        let entry = *self.ir.name(field).ok_or(Internal::missed("IR name"))?;
        let body = self.placement(kind_item)?.body;
        if let Some(child) = self.written_child(body, entry.sym.0)? {
            return self.written_item(child);
        }
        let Some(kind) = self.written_kind(kind_item)? else {
            return Ok(Written::Value);
        };
        let decl = self
            .vocabulary
            .node_kind(kind)
            .ok_or(Internal::missed("node kind"))?;
        let text = self.ir.symbols.text(entry.sym).to_string();
        if decl.attrs.iter().any(|attr| attr.name == text) {
            return Ok(Written::Attr);
        }
        if self.skipped_top().binary_search(&entry.sym.0).is_ok() {
            return Ok(Written::Value);
        }
        let kind_name = decl.name.clone();
        let name = self
            .ir
            .name_text(self.placement(named)?.as_name)
            .unwrap_or_default()
            .to_string();
        let span = self.span(self.file_of(node), entry.lo, entry.hi);
        self.draw(
            Code::UnknownAttribute,
            span,
            format!("the `{kind_name}` named `{name}` has no child, wire or attribute `{text}`"),
        );
        Ok(Written::Value)
    }

    fn written_use_step(&mut self, node: u32, field: u32, used: u32) -> Result<Written, Internal> {
        let entry = *self.ir.name(field).ok_or(Internal::missed("IR name"))?;
        let body = self.pattern_parts(used)?.1;
        if let Some(child) = self.written_child(body, entry.sym.0)? {
            return self.written_item(child);
        }
        let name = self
            .ir
            .patterns
            .get(used as usize)
            .ok_or(Internal::missed("IR pattern"))?
            .name;
        let owner = self.ir.name_text(name).unwrap_or_default().to_string();
        let text = self.ir.symbols.text(entry.sym).to_string();
        let span = self.span(self.file_of(node), entry.lo, entry.hi);
        self.draw(
            Code::UnknownAttribute,
            span,
            format!("the use of `{owner}` has no child, wire or attribute `{text}`"),
        );
        Ok(Written::Value)
    }
}

impl Core<'_> {
    pub fn path_row(&self, node: u32) -> Option<u32> {
        if !self.env_path_rows.is_empty() {
            for (env, _, _) in self.own_calls.iter().rev() {
                if let Some(row) = self.env_path_rows.get(&(node, *env)) {
                    return Some(*row);
                }
            }
        }
        self.path_rows.get(&node).copied()
    }
}

impl Core<'_> {
    fn use_param_path_end(
        &mut self,
        node: u32,
        owner: u32,
        position: u32,
        field: u32,
    ) -> Stage<End> {
        if self.is_field_receiver(node) {
            return Ok(Err(Declined(Reason::Member)));
        }
        let mut first = None;
        for env in 0..u32::try_from(self.envs.len()).unwrap_or(0) {
            let Some(record) = self.envs.get(env as usize).copied() else {
                continue;
            };
            let owned = self
                .bodies
                .get(record.inst as usize)
                .is_some_and(|body| body.function == owner);
            if record.use_item == NONE || !owned {
                continue;
            }
            CountedWalk::CorePathStep.step();
            let value = self
                .env_vals
                .get((record.values.0 + position) as usize)
                .cloned()
                .unwrap_or(CoreValue::Error);
            let row = match value {
                CoreValue::Node(row) if row != NONE => row,
                CoreValue::Error | CoreValue::Node(_) => continue,
                CoreValue::Int(_)
                | CoreValue::F32(_)
                | CoreValue::F64(_)
                | CoreValue::Bool(_)
                | CoreValue::Enum(..)
                | CoreValue::Str(_)
                | CoreValue::Array(_) => return Ok(Err(Declined(Reason::Member))),
            };
            let links = self.enter_env(env);
            let end = self.step_row(node, row, field);
            self.leave_env(links);
            let end = match end? {
                Ok(end) => end,
                Err(declined) => return Ok(Err(declined)),
            };
            match end {
                End::Attr(at) => {
                    self.env_reads.insert((node, env), at);
                    self.env_param_reads
                        .entry(env)
                        .or_default()
                        .push((node, at));
                }
                End::Row(_) | End::Use(_) | End::To | End::Wire(_) => {
                    return Ok(Err(Declined(Reason::Member)));
                }
                End::Value | End::Placeholder => {}
            }
            if first.is_none()
                || matches!(end, End::Attr(_)) && !matches!(first, Some(End::Attr(_)))
            {
                first = Some(end);
            }
        }
        Ok(Ok(first.unwrap_or(End::Placeholder)))
    }
}

impl Core<'_> {
    fn let_reference_row(&mut self, at: u32, node: u32, field: u32) -> Stage<End> {
        let root = self.let_root(at)?;
        if self.body_of.get(self.local(root)?).copied().unwrap_or(NONE) != NONE {
            let Some(count) = self.literal_array_length(root, field)? else {
                return Ok(Err(Declined(Reason::Member)));
            };
            self.array_members.insert(node, i128::from(count));
            return Ok(Ok(End::Value));
        }
        if self.let_reads_cell(at)? {
            return Ok(Ok(End::Placeholder));
        }
        if !self.path_free_let(at)? {
            return Ok(Err(Declined(Reason::Member)));
        }
        let fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if matches!(fact.leaf, Leaf::Placeholder) {
            return Ok(Ok(End::Placeholder));
        }
        if fact.leaf == Leaf::Error && !fact.node {
            return Ok(Ok(End::Placeholder));
        }
        if !fact.node {
            return self.let_member(node, field, &fact);
        }
        if !fact.dims.is_empty() {
            return Ok(Err(Declined(Reason::Member)));
        }
        let (drawn, stop) = (self.diagnostics.len(), self.stop.clone());
        let value = self.evaluate(root, EvalTy::Exact)?;
        self.diagnostics.truncate(drawn);
        self.stop = stop;
        Ok(Ok(match value {
            Ok(CoreValue::Node(row)) if row != NONE => End::Row(row),
            Ok(_) => End::Placeholder,
            Err(declined) => return Ok(Err(declined)),
        }))
    }
}

impl Core<'_> {
    fn let_member(&mut self, node: u32, field: u32, fact: &Fact) -> Stage<End> {
        let text = self.ir.name_text(field).unwrap_or_default().to_string();
        let member = matches!(text.as_str(), "size" | "capacity");
        let length = fact.dims.first().copied();
        let numeric = matches!(
            fact.leaf,
            Leaf::Int | Leaf::Float | Leaf::Bool | Leaf::Enum(_)
        );
        if member && numeric && !fact.dims.is_empty() && length != Some(ANY_LENGTH) {
            self.array_members
                .insert(node, i128::from(length.unwrap_or(0)));
            return Ok(Ok(End::Value));
        }
        let declared = self
            .ir
            .name(field)
            .and_then(|entry| self.decl.get(entry.sym.0 as usize))
            .is_some_and(|decl| *decl != DeclRef::None);
        let special = text == "stepped";
        if numeric && text == "dropped" {
            self.draw_at(
                Code::UnbuiltConstruct,
                node,
                "a cell's `dropped` (SPEC §12.1) is a construct this compiler does not build",
            )?;
            return Ok(Ok(End::Placeholder));
        }
        if (member || !special) && numeric && fact.dims.is_empty() && !declared {
            self.draw_at(
                Code::UnknownAttribute,
                node,
                format!(
                    "`.{text}` reads an attribute of an instance or a wire, and this is a value"
                ),
            )?;
            return Ok(Ok(End::Value));
        }
        if numeric && !fact.dims.is_empty() && text == "map" {
            self.draw_at(
                Code::UnbuiltConstruct,
                node,
                "an array's `map` (SPEC §5.7) is a construct this compiler does not build",
            )?;
            return Ok(Ok(End::Placeholder));
        }
        let function = self
            .ir
            .name(field)
            .and_then(|entry| self.decl.get(entry.sym.0 as usize))
            .is_some_and(|decl| matches!(decl, DeclRef::Fn(_)));
        if !member
            && !function
            && !matches!(text.as_str(), "bytes" | "dropped")
            && numeric
            && !fact.dims.is_empty()
        {
            self.draw_at(
                Code::UnknownAttribute,
                node,
                "an array has no attributes; index it first",
            )?;
            return Ok(Ok(End::Value));
        }
        Ok(Err(Declined(Reason::Member)))
    }
}

impl Core<'_> {
    fn path_free_let(&mut self, at: u32) -> Result<bool, Internal> {
        if let Some(known) = self.path_free.get(&at) {
            return Ok(*known);
        }
        let mut lets = vec![at];
        let mut seen = std::collections::HashSet::new();
        let found = self.path_free_walk(&mut lets, &mut seen);
        let free = found?;
        for at in seen {
            if free {
                self.path_free.insert(at, true);
            }
        }
        self.path_free.insert(at, free);
        Ok(free)
    }

    fn path_free_walk(
        &self,
        lets: &mut Vec<u32>,
        seen: &mut std::collections::HashSet<u32>,
    ) -> Result<bool, Internal> {
        while let Some(at) = lets.pop() {
            if !seen.insert(at) {
                continue;
            }
            if self.path_free.get(&at) == Some(&true) {
                continue;
            }
            for read in self.postorder(self.let_root(at)?)? {
                let op = self.ir.node(read)?.0;
                if matches!(
                    op,
                    Op::Field
                        | Op::To
                        | Op::Call
                        | Op::NamedCall
                        | Op::MethodCall
                        | Op::NamedMethodCall
                ) {
                    return Ok(false);
                }
                if op == Op::Name
                    && let DeclRef::Let(other) = self.bound(read)?
                {
                    lets.push(other);
                }
            }
        }
        Ok(true)
    }
}

impl Core<'_> {
    fn cell_member(&mut self, at: u32, node: u32, field: u32) -> Stage<End> {
        let text = self.ir.name_text(field).unwrap_or_default().to_string();
        let member = matches!(text.as_str(), "size" | "capacity");
        let annotation = self.ir.binder(self.cell_binder(at)?)?.annotation;
        if annotation == NONE {
            if !member {
                return Ok(Err(Declined(Reason::Member)));
            }
            self.cell_sizes.insert(node, at);
            return Ok(Ok(End::Value));
        }
        let (op, _, held, levels) = self.ir.node(annotation)?;
        let variable = op == Op::TyVar && text == "capacity";
        let fixed = op == Op::TyArray
            && levels == 1
            && held != ANY_LENGTH
            && matches!(text.as_str(), "size" | "capacity");
        if !(variable || fixed) {
            if member && (op == Op::TyScalar || op == Op::TyVar && levels == 1) {
                self.cell_sizes.insert(node, at);
                return Ok(Ok(End::Value));
            }
            return Ok(Err(Declined(Reason::Member)));
        }
        self.array_members.insert(node, i128::from(held));
        Ok(Ok(End::Value))
    }
}

impl Core<'_> {
    fn literal_array_length(&self, receiver: u32, field: u32) -> Result<Option<u32>, Internal> {
        let text = self.ir.name_text(field).unwrap_or_default();
        if !matches!(text, "size" | "capacity") {
            return Ok(None);
        }
        let (op, start, count, _) = self.ir.node(receiver)?;
        if op != Op::Array || count == 0 {
            return Ok(None);
        }
        let mut kinds = Vec::new();
        for at in start..start + count {
            let element = self.ir.node(self.kid(at)?)?.0;
            if !matches!(element, Op::Int | Op::Float) {
                return Ok(None);
            }
            kinds.push(element);
        }
        Ok((!kinds.is_empty()).then_some(count))
    }
}

impl Core<'_> {
    fn element_array_length(&mut self, receiver: u32, field: u32) -> Result<Option<u32>, Internal> {
        let text = self.ir.name_text(field).unwrap_or_default();
        if !matches!(text, "size" | "capacity") {
            return Ok(None);
        }
        let (op, base, index, _) = self.ir.node(receiver)?;
        if op != Op::Index || self.ir.node(base)?.0 != Op::Name || self.ir.node(index)?.0 != Op::Int
        {
            return Ok(None);
        }
        let DeclRef::Let(at) = self.bound(base)? else {
            return Ok(None);
        };
        let root = self.let_root(at)?;
        if self.body_of.get(self.local(root)?).copied().unwrap_or(NONE) != NONE
            || self.let_reads_cell(at)?
            || !self.path_free_let(at)?
        {
            return Ok(None);
        }
        let Ok(fact) = self.ensure_typed(root)? else {
            return Ok(None);
        };
        let numeric = matches!(fact.leaf, Leaf::Int | Leaf::Float);
        let (Some(outer), Some(inner)) = (fact.dims.first(), fact.dims.get(1)) else {
            return Ok(None);
        };
        let Ok(Ok(CoreValue::Int(position))) = self.evaluate(index, EvalTy::Exact) else {
            return Ok(None);
        };
        let inside = (0..i128::from(*outer)).contains(&position);
        Ok((numeric && inside && *inner != ANY_LENGTH && *outer != ANY_LENGTH).then_some(*inner))
    }
}
