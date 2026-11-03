use std::collections::BTreeMap;

use symphonyscript_core::{AttrDecl, Count, NodeKind, Scalar, Span};

use super::core::{Core, Stage};
use super::declined::Declined;
use super::fact::Fact;
use super::leaf::Leaf;
use super::reason::Reason;
use super::values::flexible;
use super::width::widens;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{AttrForm, NONE};
use crate::semantics::operators::{EvalTy, Value, int_range, ty_name};

fn describe(count: Count) -> String {
    match count {
        Count::Exactly(n) => format!("exactly {n}"),
        Count::AtLeast(n) => format!("at least {n}"),
        Count::AtMost(n) => format!("at most {n}"),
        Count::Between(n, m) => format!("between {n} and {m}"),
        Count::Any => "any number".to_string(),
    }
}

fn expected(attr: &AttrDecl) -> Option<EvalTy> {
    if !attr.ty.lengths.is_empty() || attr.ty.capacity.is_some() || attr.unit.is_some() {
        return None;
    }
    Some(match attr.ty.scalar {
        Scalar::U8 => EvalTy::U8,
        Scalar::I32 => EvalTy::I32,
        Scalar::I64 => EvalTy::I64,
        Scalar::F32 => EvalTy::F32,
        Scalar::F64 => EvalTy::F64,
        Scalar::Bool => EvalTy::Bool,
        Scalar::Enum(_) | Scalar::Str => return None,
    })
}

impl Core<'_> {
    fn kind_name(&self, kind: NodeKind) -> Result<String, Internal> {
        Ok(self
            .vocabulary
            .node_kind(kind)
            .ok_or(Internal::missed("node kind"))?
            .name
            .clone())
    }

    fn row_span(&self, row: u32) -> Result<Span, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let placement = self.placement(record.item)?;
        Ok(self.span(record.file, placement.lo, placement.hi))
    }

    pub fn check_attributes(&mut self) -> Stage<()> {
        for row in 0..self.instances.len() {
            CountedWalk::CheckRow.step();
            let record = *self.instances.get(row).ok_or(Internal::missed("row"))?;
            let placement = self.placement(record.item)?;
            let decl = self
                .vocabulary
                .node_kind(record.kind)
                .ok_or(Internal::missed("node kind"))?
                .clone();
            let mut sources: Vec<(usize, Span, u32)> = Vec::new();
            let mut set = |core: &mut Self, attr: usize, span: Span, root: u32| {
                if let Some((_, earlier, _)) = sources.iter().find(|(seen, _, _)| *seen == attr) {
                    let name = decl
                        .attrs
                        .get(attr)
                        .map(|attr| attr.name.clone())
                        .unwrap_or_default();
                    core.draw_related(
                        Code::DuplicateAttribute,
                        span,
                        vec![(earlier.clone(), "first set here".to_string())],
                        format!("the attribute `{name}` is set twice"),
                    );
                    return;
                }
                sources.push((attr, span, root));
            };
            if placement.default != NONE {
                let span = self.node_span(placement.default)?;
                match decl.default_attr {
                    Some(attr) => set(self, attr.index(), span, placement.default),
                    None => self.draw(
                        Code::NoDefaultAttribute,
                        span,
                        format!("`{}` has no default attribute for `/expr`", decl.name),
                    ),
                }
            }
            for at in placement.attrs.0..placement.attrs.1 {
                let entry = *self
                    .ir
                    .attr_entries
                    .get(at as usize)
                    .ok_or(Internal::missed("IR attribute"))?;
                let AttrForm::Value(root) = entry.form else {
                    return Err(Internal::missed("a flag the coverage table leaves out"));
                };
                let name = self
                    .ir
                    .name(entry.name)
                    .ok_or(Internal::missed("IR name"))?;
                let text = self.ir.symbols.text(name.sym).to_string();
                let name_span = self.span(record.file, name.lo, name.hi);
                match decl.attrs.iter().position(|attr| attr.name == text) {
                    Some(attr) => {
                        let span = self.span(record.file, entry.lo, entry.hi);
                        set(self, attr, span, root);
                    }
                    None => self.draw(
                        Code::UnknownAttribute,
                        name_span,
                        format!("`{}` has no attribute `{text}`", decl.name),
                    ),
                }
            }
            for (position, attr) in decl.attrs.iter().enumerate() {
                if attr.default.is_none() && !sources.iter().any(|(seen, _, _)| *seen == position) {
                    let span = self.span(record.file, placement.lo, placement.hi);
                    self.draw(
                        Code::MissingAttribute,
                        span,
                        format!(
                            "`{}` needs a value for `{}`, which has no default",
                            decl.name, attr.name
                        ),
                    );
                }
            }
            for (attr, _, root) in sources {
                let attr = decl.attrs.get(attr).ok_or(Internal::missed("attribute"))?;
                let Some(ty) = expected(attr) else {
                    return Ok(Err(Declined(attribute_reason(attr))));
                };
                if let Err(declined) = self.bind_value(root, &attr.name, ty)? {
                    return Ok(Err(declined));
                }
            }
        }
        Ok(Ok(()))
    }

    fn bind_value(&mut self, root: u32, name: &str, ty: EvalTy) -> Stage<()> {
        let fact: Fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        let integer = int_range(ty).is_some();
        match fact.leaf {
            Leaf::Placeholder | Leaf::Error => return self.evaluate_own(root),
            _ if !fact.dims.is_empty() => {
                self.draw_at(
                    Code::TypeMismatch,
                    root,
                    format!(
                        "`{name}` holds one `{}`, and this value is an array",
                        ty_name(ty)
                    ),
                )?;
                return self.evaluate_own(root);
            }
            Leaf::Bool if ty != EvalTy::Bool => {
                self.draw_at_root(
                    Code::TypeMismatch,
                    root,
                    format!(
                        "this value has the wrong type here; `{}` is expected",
                        ty_name(ty)
                    ),
                )?;
                return self.evaluate_own(root);
            }
            Leaf::Empty | Leaf::Int | Leaf::Float if ty == EvalTy::Bool => {
                self.draw_at_root(
                    Code::TypeMismatch,
                    root,
                    "this value has the wrong type here; `bool` is expected",
                )?;
                return self.evaluate_own(root);
            }
            Leaf::Float if integer && self.reads_float_cell(root)? => {
                self.draw_at_root(
                    Code::FloatToInteger,
                    root,
                    "a float cell is used where an integer is expected; a conversion states the rounding",
                )?;
                return self.evaluate_own(root);
            }
            Leaf::Float if integer => {
                self.draw_at_root(
                    Code::TypeMismatch,
                    root,
                    "a float value stands where an integer is expected; a conversion states the rounding",
                )?;
                return self.evaluate_own(root);
            }
            Leaf::Bool | Leaf::Empty | Leaf::Int | Leaf::Float => {}
        }
        if let Some(own) = fact.fixed
            && !widens(own, ty)
        {
            self.draw_at_root(
                Code::TypeMismatch,
                root,
                format!(
                    "a `{}` value does not widen into `{}`",
                    ty_name(own),
                    ty_name(ty)
                ),
            )?;
            return self.evaluate_own(root);
        }
        let evaluated = match fact.fixed {
            Some(own) if widens(own, ty) => ty,
            Some(own) => own,
            None if self.is_reactive(root)? => ty,
            None => flexible(ty),
        };
        if self.is_kept(root)? {
            return self.evaluate_own(root);
        }
        let value = match self.evaluate(root, evaluated)? {
            Ok(value) => value,
            Err(declined) => return Ok(Err(declined)),
        };
        let fits = match (&value, int_range(ty)) {
            (Value::Int(value), Some(range)) => range.contains(value),
            (Value::F32(value), _) => value.is_finite() || fact.fixed.is_some(),
            (Value::F64(value), _) => value.is_finite() || fact.fixed.is_some(),
            _ => true,
        };
        if !fits {
            self.draw_at_root(
                Code::DoesNotFit,
                root,
                format!("this value does not fit `{}`", ty_name(ty)),
            )?;
        }
        Ok(Ok(()))
    }

    pub fn check_counts(&mut self) -> Result<(), Internal> {
        let mut by_kind: BTreeMap<NodeKind, Vec<u32>> = BTreeMap::new();
        for row in self.top_instances.clone() {
            let kind = self
                .instances
                .get(row as usize)
                .ok_or(Internal::missed("row"))?
                .kind;
            let decl = self
                .vocabulary
                .node_kind(kind)
                .ok_or(Internal::missed("node kind"))?;
            if decl.top_level.is_none() {
                let span = self.row_span(row)?;
                let message = format!("a `{}` does not stand at top level", decl.name);
                self.draw(Code::NotTopLevel, span, message);
                continue;
            }
            by_kind.entry(kind).or_default().push(row);
        }
        for index in 0..self.vocabulary.node_kinds().len() {
            let kind = self
                .vocabulary
                .node_kind_at(index)
                .ok_or(Internal::missed("node kind"))?;
            let decl = self
                .vocabulary
                .node_kind(kind)
                .ok_or(Internal::missed("node kind"))?;
            let Some(count) = decl.top_level else {
                continue;
            };
            let found = by_kind.get(&kind).cloned().unwrap_or_default();
            let name = decl.name.clone();
            if let Some(max) = count.max()
                && let Some(extra) = found.get(max as usize)
            {
                let span = self.row_span(*extra)?;
                self.draw(
                    Code::CountExceeded,
                    span,
                    format!(
                        "the program holds more top-level `{name}` instances than {}",
                        describe(count)
                    ),
                );
            }
            if found.len() < count.min() as usize && !self.unknown_top {
                let span = self.span(0, 0, 0);
                self.draw(
                    Code::CountBelowMinimum,
                    span,
                    format!(
                        "the program holds {} top-level `{name}`, and it needs {}",
                        found.len(),
                        describe(count)
                    ),
                );
            }
        }
        for parent in 0..self.instances.len() {
            self.children_counts(u32::try_from(parent).unwrap_or(u32::MAX))?;
        }
        for row in 0..self.instances.len() {
            self.wire_minimums(u32::try_from(row).unwrap_or(u32::MAX))?;
        }
        Ok(())
    }

    fn wire_minimums(&mut self, row: u32) -> Result<(), Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let decl = self
            .vocabulary
            .node_kind(record.kind)
            .ok_or(Internal::missed("node kind"))?
            .clone();
        let directed = decl
            .outgoing
            .iter()
            .map(|entry| (entry, "outgoing"))
            .chain(decl.incoming.iter().map(|entry| (entry, "incoming")));
        for ((wire_kind, count), direction) in directed {
            let unknown = if direction == "outgoing" {
                self.unknown_children.contains(&row)
            } else {
                self.unknown_wires
            };
            if count.min() == 0 || unknown {
                continue;
            }
            let name = self
                .vocabulary
                .wire_kind(*wire_kind)
                .ok_or(Internal::missed("wire kind"))?
                .name
                .clone();
            let span = self.row_span(row)?;
            self.draw(
                Code::CountBelowMinimum,
                span,
                format!(
                    "this `{}` has 0 {direction} `{name}` wires, and it needs {}",
                    decl.name,
                    describe(*count)
                ),
            );
        }
        Ok(())
    }

    fn children_counts(&mut self, parent: u32) -> Result<(), Internal> {
        let record = *self
            .instances
            .get(parent as usize)
            .ok_or(Internal::missed("row"))?;
        let decl = self
            .vocabulary
            .node_kind(record.kind)
            .ok_or(Internal::missed("node kind"))?
            .clone();
        let mut by_kind: BTreeMap<NodeKind, Vec<u32>> = BTreeMap::new();
        for child in record.children.0..record.children.1 {
            let kind = self
                .instances
                .get(child as usize)
                .ok_or(Internal::missed("row"))?
                .kind;
            if !decl.children.iter().any(|(accepted, _)| *accepted == kind) {
                let span = self.row_span(child)?;
                let message = if decl.children.is_empty() {
                    format!(
                        "`{}` accepts no children, and its body holds only wires",
                        decl.name
                    )
                } else {
                    format!(
                        "`{}` does not accept a `{}` child",
                        decl.name,
                        self.kind_name(kind)?
                    )
                };
                self.draw(Code::ChildNotAccepted, span, message);
                continue;
            }
            by_kind.entry(kind).or_default().push(child);
        }
        for (kind, count) in &decl.children {
            let found = by_kind.get(kind).cloned().unwrap_or_default();
            let name = self.kind_name(*kind)?;
            if let Some(max) = count.max()
                && let Some(extra) = found.get(max as usize)
            {
                let span = self.row_span(*extra)?;
                self.draw(
                    Code::CountExceeded,
                    span,
                    format!(
                        "`{}` holds {} `{name}` children",
                        decl.name,
                        describe(*count)
                    ),
                );
            }
            if found.len() < count.min() as usize && !self.unknown_children.contains(&parent) {
                let span = self.row_span(parent)?;
                self.draw(
                    Code::CountBelowMinimum,
                    span,
                    format!(
                        "this `{}` holds {} `{name}` children, and it needs {}",
                        decl.name,
                        found.len(),
                        describe(*count)
                    ),
                );
            }
        }
        Ok(())
    }

    pub fn check_reachability(&mut self) -> Result<(), Internal> {
        if !self.vocabulary.node_kinds().iter().any(|decl| decl.entry) {
            return Ok(());
        }
        let mut entries = Vec::new();
        for (row, record) in self.instances.iter().enumerate() {
            let decl = self
                .vocabulary
                .node_kind(record.kind)
                .ok_or(Internal::missed("node kind"))?;
            if decl.entry {
                entries.push(u32::try_from(row).unwrap_or(u32::MAX));
            }
        }
        if entries.is_empty() {
            let span = self.span(0, 0, 0);
            self.draw(
                Code::NoEntryInstance,
                span,
                "the program places no instance of an entry kind, so nothing reaches its instances",
            );
            return Ok(());
        }
        let mut reached = vec![false; self.instances.len()];
        let mut stack = entries;
        while let Some(row) = stack.pop() {
            let Some(seen) = reached.get_mut(row as usize) else {
                continue;
            };
            if *seen {
                continue;
            }
            *seen = true;
            let record = *self
                .instances
                .get(row as usize)
                .ok_or(Internal::missed("row"))?;
            stack.extend(record.children.0..record.children.1);
        }
        for row in self.top_instances.clone() {
            if reached.get(row as usize).copied().unwrap_or(true) {
                continue;
            }
            let record = *self
                .instances
                .get(row as usize)
                .ok_or(Internal::missed("row"))?;
            let placement = self.placement(record.item)?;
            let name = self
                .ir
                .name(placement.as_name)
                .ok_or(Internal::missed("IR name"))?;
            let label = self.ir.symbols.text(name.sym).to_string();
            let span = self.span(record.file, name.lo, name.hi);
            self.draw(
                Code::UnreachableInstance,
                span,
                format!("no entry-kind instance reaches `{label}` through children and wires"),
            );
        }
        Ok(())
    }
}

fn attribute_reason(attr: &AttrDecl) -> Reason {
    if attr.unit.is_some() {
        return Reason::UnitAttribute;
    }
    if !attr.ty.lengths.is_empty() || attr.ty.capacity.is_some() {
        return Reason::ArrayAttribute;
    }
    match attr.ty.scalar {
        Scalar::Enum(_) => Reason::EnumAttribute,
        Scalar::Str
        | Scalar::U8
        | Scalar::I32
        | Scalar::I64
        | Scalar::F32
        | Scalar::F64
        | Scalar::Bool => Reason::StringAttribute,
    }
}
