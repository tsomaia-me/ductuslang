use symphonyscript_core::Span;

use super::attr_read::AttrRead;
use super::core::{Core, Stage};
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{ABSENT, AttrForm, ItemRec, NONE, Op};
use crate::semantics::operators::EvalTy;

impl Core<'_> {
    pub fn index_wire_names(&mut self) -> Result<(), Internal> {
        let mut index = Vec::new();
        for (at, row) in self.wires.iter().enumerate() {
            CountedWalk::CorePathStep.step();
            let rec = self.wire_rec(row.item)?;
            if !self.ir.named(rec.as_name) {
                continue;
            }
            let entry = self
                .ir
                .name(rec.as_name)
                .ok_or(Internal::missed("IR name"))?;
            index.push((
                row.source,
                entry.sym.0,
                u32::try_from(at).unwrap_or(u32::MAX),
            ));
        }
        index.sort_unstable();
        self.wire_index = index;
        Ok(())
    }

    pub fn wire_name_span(&self, wire: u32) -> Result<Span, Internal> {
        let row = *self
            .wires
            .get(wire as usize)
            .ok_or(Internal::missed("wire row"))?;
        let rec = self.wire_rec(row.item)?;
        self.name_span(row.file, rec.as_name)
    }

    pub fn wires_named(&self, parent: u32, sym: u32) -> Vec<u32> {
        if parent == NONE {
            return Vec::new();
        }
        CountedWalk::CorePathStep.step();
        let at = self
            .wire_index
            .partition_point(|entry| (entry.0, entry.1) < (parent, sym));
        self.wire_index
            .get(at..)
            .unwrap_or_default()
            .iter()
            .take_while(|entry| (entry.0, entry.1) == (parent, sym))
            .map(|entry| entry.2)
            .collect()
    }

    pub fn wire_named(&self, parent: u32, sym: u32) -> Option<u32> {
        self.wires_named(parent, sym).first().copied()
    }

    pub fn wire_named_before(
        &self,
        parent: u32,
        sym: u32,
        at: &Span,
    ) -> Result<Option<Span>, Internal> {
        let mut found: Option<Span> = None;
        for wire in self.wires_named(parent, sym) {
            let span = self.wire_name_span(wire)?;
            if span.file == at.file && span.range.start < at.range.start {
                found = Some(match found {
                    Some(earlier) if earlier.range.start <= span.range.start => earlier,
                    Some(_) | None => span,
                });
            }
        }
        Ok(found)
    }

    pub fn wire_clashes(&mut self, parent: &[u32]) -> Result<(), Internal> {
        let entries = self.wire_index.clone();
        let mut order: Vec<(u32, u32, u32)> = entries;
        order.sort_unstable_by_key(|entry| entry.2);
        for (body, sym, wire) in order {
            let span = self.wire_name_span(wire)?;
            let text = self
                .ir
                .symbols
                .text(crate::semantics::ir::flat::Sym(sym))
                .to_string();
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
                if decl.attrs.iter().any(|attr| attr.name == text) {
                    let message = format!(
                        "`{text}` is an attribute of the parent's kind `{}`",
                        decl.name
                    );
                    self.draw(Code::ChildNameClash, span.clone(), message);
                }
            }
            let mut earlier = self.wire_named_before(body, sym, &span)?;
            if let Some(child) = self.child_named(body, sym) {
                let child_span = self.row_name_span(child)?;
                if child_span.file == span.file && child_span.range.start < span.range.start {
                    earlier = Some(match earlier {
                        Some(other) if other.range.start <= child_span.range.start => other,
                        Some(_) | None => child_span,
                    });
                }
            }
            let mut up = body;
            while earlier.is_none() && up != NONE {
                up = parent.get(up as usize).copied().unwrap_or(NONE);
                if up == NONE {
                    break;
                }
                if let Some(child) = self.child_named(up, sym) {
                    earlier = Some(self.row_name_span(child)?);
                } else if let Some(other) = self.wire_named(up, sym) {
                    earlier = Some(self.wire_name_span(other)?);
                }
            }
            if let Some(earlier) = earlier {
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
}

impl Core<'_> {
    pub fn wire_attr_source(&self, wire: u32, attr: usize) -> Result<(u32, u32, u32), Internal> {
        let row = *self
            .wires
            .get(wire as usize)
            .ok_or(Internal::missed("wire row"))?;
        let rec = self.wire_rec(row.item)?;
        let decl = self
            .vocabulary
            .wire_kind(row.kind)
            .ok_or(Internal::missed("wire kind"))?;
        if rec.default != NONE && decl.default_attr.map(|at| at.index()) == Some(attr) {
            let (lo, hi) = self.ir.span(rec.default)?;
            return Ok((rec.default, lo, hi));
        }
        let name = decl
            .attrs
            .get(attr)
            .ok_or(Internal::missed("attribute"))?
            .name
            .as_str();
        let flagged = decl.attrs.get(attr).and_then(|attr| attr.flag);
        for at in rec.flags.0..rec.flags.1 {
            let flag = *self
                .ir
                .flags
                .get(at as usize)
                .ok_or(Internal::missed("IR flag"))?;
            if Some(flag.flag) == flagged {
                return Ok((ABSENT, flag.lo, flag.hi));
            }
        }
        for at in rec.attrs.0..rec.attrs.1 {
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

    pub fn wire_attr_read(&mut self, wire: u32, attr: usize, ty: EvalTy) -> Stage<u32> {
        let key = (wire, u32::try_from(attr).unwrap_or(u32::MAX));
        if let Some(found) = self.wire_attr_index.get(&key) {
            return Ok(Ok(*found));
        }
        let row = *self
            .wires
            .get(wire as usize)
            .ok_or(Internal::missed("wire row"))?;
        let (root, lo, hi) = self.wire_attr_source(wire, attr)?;
        let set = if root == ABSENT {
            let rec = self.wire_rec(row.item)?;
            let declared = self
                .vocabulary
                .wire_kind(row.kind)
                .and_then(|decl| decl.attrs.get(attr))
                .ok_or(Internal::missed("attribute"))?;
            Some(self.set_value(rec.flags, rec.attrs, declared)?)
        } else {
            None
        };
        let root = if set.is_some() { NONE } else { root };
        let name = self
            .vocabulary
            .wire_kind(row.kind)
            .and_then(|decl| decl.attrs.get(attr))
            .ok_or(Internal::missed("attribute"))?
            .name
            .clone();
        let rec = self.wire_rec(row.item)?;
        let own = self
            .ir
            .name_text(rec.as_name)
            .unwrap_or_default()
            .to_string();
        let at = u32::try_from(self.attr_reads.len()).unwrap_or(u32::MAX);
        self.attr_reads.push(AttrRead {
            row: wire,
            attr,
            root,
            ty,
            label: format!("{own}.{name}"),
            span: self.span(row.file, lo, hi),
            key: (row.file, lo),
            cyclic: false,
            wire: true,
            set,
        });
        self.wire_attr_index.insert(key, at);
        self.attr_values
            .push([None, None, None, None, None, None, None]);
        Ok(Ok(at))
    }
}

impl Core<'_> {
    pub fn reads_a_wire_name(&self) -> Result<bool, Internal> {
        let mut named = std::collections::HashSet::new();
        for item in &self.ir.items {
            if let ItemRec::Wire(wire) = item
                && self.ir.named(wire.as_name)
                && let Some(entry) = self.ir.name(wire.as_name)
            {
                named.insert(entry.sym.0);
            }
        }
        if named.is_empty() {
            return Ok(false);
        }
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Name)
                || !self.reached.get(local).copied().unwrap_or(false)
            {
                continue;
            }
            let name = *self.ir.a.get(local).ok_or(Internal::missed("IR name"))?;
            let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
            if named.contains(&entry.sym.0) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Core<'_> {
    pub fn read_attr_decl(&self, read: &AttrRead) -> Option<&symphonyscript_core::AttrDecl> {
        if read.wire {
            let kind = self.wires.get(read.row as usize)?.kind;
            self.vocabulary.wire_kind(kind)?.attrs.get(read.attr)
        } else {
            let kind = self.instances.get(read.row as usize)?.kind;
            self.vocabulary.node_kind(kind)?.attrs.get(read.attr)
        }
    }

    pub fn read_family(&self, read: &AttrRead) -> Option<symphonyscript_core::FamilyId> {
        self.read_attr_decl(read)
            .and_then(|attr| attr.unit)
            .map(|unit| unit.family)
    }
}

impl Core<'_> {
    pub fn set_value(
        &self,
        flags: (u32, u32),
        attrs: (u32, u32),
        declared: &symphonyscript_core::AttrDecl,
    ) -> Result<bool, Internal> {
        for at in flags.0..flags.1 {
            let flag = *self
                .ir
                .flags
                .get(at as usize)
                .ok_or(Internal::missed("IR flag"))?;
            if Some(flag.flag) == declared.flag {
                return Ok(true);
            }
        }
        for at in attrs.0..attrs.1 {
            let entry = *self
                .ir
                .attr_entries
                .get(at as usize)
                .ok_or(Internal::missed("IR attribute"))?;
            if self.ir.name_text(entry.name) != Some(declared.name.as_str()) {
                continue;
            }
            if let AttrForm::Set(value) = entry.form {
                return Ok(value);
            }
        }
        Ok(false)
    }
}

impl Core<'_> {
    pub fn in_cell_rule(&self, node: u32) -> Result<bool, Internal> {
        for rec in &self.ir.cells {
            for root in [rec.rule, rec.init] {
                if root != NONE && self.postorder(root)?.contains(&node) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
