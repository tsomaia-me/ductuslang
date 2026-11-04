use std::collections::HashMap;

use symphonyscript_core::{AttrDefault, Value};

use super::attr_read::AttrRead;
use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::reason::Reason;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{AttrForm, NONE, Op};
use crate::semantics::operators::EvalTy;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum End {
    Row(u32),
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
            let span = self.span(self.file_of(node), entry.lo, entry.hi);
            let owner = format!("the `{}` named `{}`", decl.name, self.named_row_text(row)?);
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
        if self.body_of.get(self.local(node)?).copied().unwrap_or(NONE) != NONE {
            return Ok(Err(Declined(Reason::PathInBody)));
        }
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
        } else if op == Op::Field {
            *ends
                .get(&receiver)
                .ok_or(Internal::missed("a path's receiver"))?
        } else {
            return Ok(Err(Declined(Reason::Member)));
        };
        match start {
            End::Row(row) => self.step_row(node, row, field),
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
        let mut ends: HashMap<u32, End> = HashMap::new();
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Field)
                || !self.reached.get(local).copied().unwrap_or(false)
            {
                continue;
            }
            CountedWalk::CorePathStep.step();
            let node = self.ir.base + u32::try_from(local).unwrap_or(u32::MAX);
            let end = match self.path_end(node, &ends)? {
                Ok(end) => end,
                Err(declined) => return Ok(Err(declined)),
            };
            if !self.is_field_receiver(node) {
                match end {
                    End::Row(_) => return Ok(Err(Declined(Reason::NameOfInstance))),
                    End::Attr(at) => {
                        if let Some(slot) = self.paths.get_mut(local) {
                            *slot = at;
                        }
                    }
                    End::Placeholder | End::Value => {}
                }
            }
            ends.insert(node, end);
        }
        let roots: Vec<u32> = self
            .attr_reads
            .iter()
            .map(|read| read.root)
            .filter(|root| *root != NONE)
            .collect();
        if self.roots_read_cell(roots)? {
            return Ok(Err(Declined(Reason::ReactivePath)));
        }
        Ok(Ok(()))
    }

    fn roots_read_cell(&mut self, roots: Vec<u32>) -> Result<bool, Internal> {
        let mut seen_lets = vec![false; self.ir.lets.len()];
        let mut seen_reads = vec![false; self.attr_reads.len()];
        let mut stack = roots;
        while let Some(current) = stack.pop() {
            for node in self.deep_postorder(current)? {
                let (op, _, _, _) = self.ir.node(node)?;
                if op == Op::Field {
                    let read = self.paths.get(self.local(node)?).copied().unwrap_or(NONE);
                    if let Some(slot) = seen_reads.get_mut(read as usize)
                        && !*slot
                    {
                        *slot = true;
                        let inner = self
                            .attr_reads
                            .get(read as usize)
                            .map_or(NONE, |at| at.root);
                        if inner != NONE {
                            stack.push(inner);
                        }
                    }
                    continue;
                }
                if op != Op::Name {
                    continue;
                }
                match self.bound(node)? {
                    DeclRef::Cell(_) => return Ok(true),
                    DeclRef::Let(at) => {
                        if let Some(slot) = seen_lets.get_mut(at as usize)
                            && !*slot
                        {
                            *slot = true;
                            stack.push(self.let_root(at)?);
                        }
                    }
                    DeclRef::None
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate
                    | DeclRef::Param(_)
                    | DeclRef::Fn(_) => {}
                }
            }
        }
        Ok(false)
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
