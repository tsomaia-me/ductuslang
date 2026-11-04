use std::collections::BTreeMap;

use symphonyscript_core::{AttrDecl, AttrId, Count, NodeKind, Scalar, Span};

use super::attr_item::AttrItem;
use super::core::{Core, Stage};
use super::declined::Declined;
use super::fact::Fact;
use super::folds::ANY_LENGTH;
use super::leaf::Leaf;
use super::reason::Reason;
use super::values::flexible;
use super::width::widens;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{AttrForm, NONE, Op};
use crate::semantics::operators::{EvalTy, Value, int_range, ty_name};

pub(super) fn describe(count: Count) -> String {
    match count {
        Count::Exactly(n) => format!("exactly {n}"),
        Count::AtLeast(n) => format!("at least {n}"),
        Count::AtMost(n) => format!("at most {n}"),
        Count::Between(n, m) => format!("between {n} and {m}"),
        Count::Any => "any number".to_string(),
    }
}

fn expected_array(attr: &AttrDecl) -> Option<(EvalTy, u32)> {
    let [length] = attr.ty.lengths.as_slice() else {
        return None;
    };
    if attr.ty.capacity.is_some() || attr.unit.is_some() {
        return None;
    }
    let mut scalar = attr.clone();
    scalar.ty.lengths.clear();
    expected(&scalar).map(|ty| (ty, *length))
}

pub(super) fn expected(attr: &AttrDecl) -> Option<EvalTy> {
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

    pub fn row_span(&self, row: u32) -> Result<Span, Internal> {
        let record = *self
            .instances
            .get(row as usize)
            .ok_or(Internal::missed("row"))?;
        let placement = self.placement(record.item)?;
        Ok(self.span(record.file, placement.lo, placement.hi))
    }

    pub fn check_attributes(&mut self) -> Stage<()> {
        let vocabulary = self.vocabulary;
        for row in 0..self.instances.len() {
            CountedWalk::CheckRow.step();
            let record = *self.instances.get(row).ok_or(Internal::missed("row"))?;
            let placement = self.placement(record.item)?;
            let decl = vocabulary
                .node_kind(record.kind)
                .ok_or(Internal::missed("node kind"))?;
            let item = AttrItem {
                file: record.file,
                default: placement.default,
                attrs: placement.attrs,
                lo: placement.lo,
                hi: placement.hi,
            };
            let saved = self.enter_env(record.env);
            let checked = self.item_attributes(item, &decl.name, &decl.attrs, decl.default_attr);
            self.leave_env(saved);
            if let Err(declined) = checked? {
                return Ok(Err(declined));
            }
        }
        for wire in 0..self.wires.len() {
            CountedWalk::CheckWireRow.step();
            let row = *self.wires.get(wire).ok_or(Internal::missed("wire row"))?;
            let rec = self.wire_rec(row.item)?;
            let decl = vocabulary
                .wire_kind(row.kind)
                .ok_or(Internal::missed("wire kind"))?;
            let item = AttrItem {
                file: row.file,
                default: rec.default,
                attrs: rec.attrs,
                lo: rec.lo,
                hi: rec.hi,
            };
            if let Err(declined) =
                self.item_attributes(item, &decl.name, &decl.attrs, decl.default_attr)?
            {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }

    fn item_attributes(
        &mut self,
        item: AttrItem,
        kind_name: &str,
        decl_attrs: &[AttrDecl],
        default_attr: Option<AttrId>,
    ) -> Stage<()> {
        let AttrItem {
            file,
            default,
            attrs,
            lo,
            hi,
        } = item;
        let mut sources: Vec<(usize, Span, u32)> = Vec::new();
        let mut set = |core: &mut Self, attr: usize, span: Span, root: u32| {
            if let Some((_, earlier, _)) = sources.iter().find(|(seen, _, _)| *seen == attr) {
                let name = decl_attrs
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
        if default != NONE {
            let span = self.node_span(default)?;
            match default_attr {
                Some(attr) => set(self, attr.index(), span, default),
                None => self.draw(
                    Code::NoDefaultAttribute,
                    span,
                    format!("`{}` has no default attribute for `/expr`", kind_name),
                ),
            }
        }
        for at in attrs.0..attrs.1 {
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
            let name_span = self.span(file, name.lo, name.hi);
            match decl_attrs.iter().position(|attr| attr.name == text) {
                Some(attr) => {
                    let span = self.span(file, entry.lo, entry.hi);
                    set(self, attr, span, root);
                }
                None => self.draw(
                    Code::UnknownAttribute,
                    name_span,
                    format!("`{}` has no attribute `{text}`", kind_name),
                ),
            }
        }
        for (position, attr) in decl_attrs.iter().enumerate() {
            if attr.default.is_none() && !sources.iter().any(|(seen, _, _)| *seen == position) {
                let span = self.span(file, lo, hi);
                self.draw(
                    Code::MissingAttribute,
                    span,
                    format!(
                        "`{}` needs a value for `{}`, which has no default",
                        kind_name, attr.name
                    ),
                );
            }
        }
        for (attr, _, root) in sources {
            let attr = decl_attrs.get(attr).ok_or(Internal::missed("attribute"))?;
            if let Some((ty, length)) = expected_array(attr) {
                if let Err(declined) = self.bind_array(root, &attr.name, ty, length)? {
                    return Ok(Err(declined));
                }
                continue;
            }
            let Some(ty) = expected(attr) else {
                return Ok(Err(Declined(attribute_reason(attr))));
            };
            if let Err(declined) = self.bind_value(root, &attr.name, ty)? {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }

    fn refuse_fixed(
        &mut self,
        root: u32,
        ty: EvalTy,
        reactive: bool,
    ) -> Result<Vec<u32>, Internal> {
        let mut stack = vec![root];
        let mut refused = Vec::new();
        while let Some(node) = stack.pop() {
            let fact = self.fact(node)?;
            let Some(own) = fact.fixed.filter(|own| !widens(*own, ty)) else {
                continue;
            };
            let (op, a, b, c) = self.ir.node(node)?;
            let soft = reactive && fact.read.is_some();
            let through = match op {
                Op::Neg | Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem | Op::Pow | Op::Array
                    if soft =>
                {
                    self.operands(node)?
                }
                Op::IntDiv if soft && int_range(ty).is_some() => self.operands(node)?,
                Op::IntDiv if soft => continue,
                Op::If if soft => vec![b, c],
                Op::Index | Op::IndexFromEnd if soft => vec![a],
                Op::Neg
                | Op::Add
                | Op::Sub
                | Op::Mul
                | Op::Div
                | Op::IntDiv
                | Op::Rem
                | Op::Pow
                | Op::Array
                | Op::If
                | Op::Index
                | Op::IndexFromEnd
                | Op::Int
                | Op::Float
                | Op::Bool
                | Op::Name
                | Op::Str
                | Op::Variant
                | Op::To
                | Op::Field
                | Op::MethodCall
                | Op::NamedMethodCall
                | Op::Past
                | Op::Stepped
                | Op::Match
                | Op::TyScalar
                | Op::TyName
                | Op::TyArray
                | Op::TyVar
                | Op::Bad
                | Op::And
                | Op::Or
                | Op::Not
                | Op::Lt
                | Op::Le
                | Op::Gt
                | Op::Ge
                | Op::Is
                | Op::IsNot
                | Op::Call
                | Op::NamedCall
                | Op::Convert
                | Op::Slice
                | Op::For => Vec::new(),
            };
            if through.is_empty() {
                self.draw_at_root(
                    Code::TypeMismatch,
                    node,
                    format!(
                        "a `{}` value does not widen into `{}`",
                        ty_name(own),
                        ty_name(ty)
                    ),
                )?;
                refused.push(node);
            }
            stack.extend(through.into_iter().rev());
        }
        Ok(refused)
    }

    fn bind_array(&mut self, root: u32, name: &str, ty: EvalTy, length: u32) -> Stage<()> {
        let fact: Fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return self.own_check(root);
        }
        if fact.dims != [length] {
            self.draw_at(
                Code::TypeMismatch,
                root,
                "the value's array shape differs from the attribute's type",
            )?;
            return self.own_check(root);
        }
        if self.ir.node(root)?.0 != Op::Array {
            return Ok(Err(Declined(Reason::ArrayAttribute)));
        }
        for element in self.operands(root)? {
            if let Err(declined) = self.bind_value(element, name, ty)? {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }

    pub fn own_check(&mut self, root: u32) -> Stage<()> {
        if self.unchecked {
            return Ok(Ok(()));
        }
        self.evaluate_own(root)
    }

    pub fn bind_value(&mut self, root: u32, name: &str, ty: EvalTy) -> Stage<()> {
        let fact: Fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        let integer = int_range(ty).is_some();
        match fact.leaf {
            Leaf::Placeholder | Leaf::Error => return self.detached(root, name, ty),
            _ if !fact.dims.is_empty() => {
                let variable = self.variable_length(root)?;
                self.draw_at(
                    Code::TypeMismatch,
                    root,
                    format!(
                        "`{name}` holds one `{}`, and this value is {}",
                        ty_name(ty),
                        if variable {
                            "a variable array"
                        } else {
                            "an array"
                        }
                    ),
                )?;
                return self.own_check(root);
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
                return self.own_check(root);
            }
            Leaf::Empty | Leaf::Int | Leaf::Float if ty == EvalTy::Bool => {
                self.draw_at_root(
                    Code::TypeMismatch,
                    root,
                    "this value has the wrong type here; `bool` is expected",
                )?;
                return self.own_check(root);
            }
            Leaf::Float if integer && self.reads_float_cell(root)? => {
                self.draw_at_root(
                    Code::FloatToInteger,
                    root,
                    "a float cell is used where an integer is expected; a conversion states the rounding",
                )?;
                return self.own_check(root);
            }
            Leaf::Float if integer => {
                self.draw_at_root(
                    Code::TypeMismatch,
                    root,
                    "a float value stands where an integer is expected; a conversion states the rounding",
                )?;
                return self.own_check(root);
            }
            Leaf::Bool | Leaf::Empty | Leaf::Int | Leaf::Float => {}
        }
        let reactive = self.is_reactive(root)?;
        if let Some(own) = fact.fixed
            && !widens(own, ty)
        {
            let refused = self.refuse_fixed(root, ty, reactive)?;
            if refused.contains(&root) {
                return self.own_check(root);
            }
            for node in refused {
                if let Err(declined) = self.own_check(node)? {
                    return Ok(Err(declined));
                }
            }
        }
        let evaluated = match fact.fixed {
            Some(own) if widens(own, ty) => ty,
            _ if reactive => ty,
            Some(own) => own,
            None => flexible(ty),
        };
        if self.is_kept(root)? || self.unchecked {
            return self.own_check(root);
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
        self.check_wire_counts()?;
        self.check_wire_cycles()?;
        Ok(())
    }

    fn children_counts(&mut self, parent: u32) -> Result<(), Internal> {
        let record = *self
            .instances
            .get(parent as usize)
            .ok_or(Internal::missed("row"))?;
        let vocabulary = self.vocabulary;
        let decl = vocabulary
            .node_kind(record.kind)
            .ok_or(Internal::missed("node kind"))?;
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
            let found = by_kind.get(kind).map_or(&[][..], Vec::as_slice);
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
        if self.unknown_wires
            || self
                .wires
                .iter()
                .any(|wire| wire.candidates.0 == wire.candidates.1)
        {
            return Ok(());
        }
        let edges = self.wire_edges();
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
            if let Some(targets) = edges.get(row as usize) {
                stack.extend(targets.iter().copied());
            }
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

pub(super) fn attribute_reason(attr: &AttrDecl) -> Reason {
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

impl Core<'_> {
    fn variable_length(&self, root: u32) -> Result<bool, Internal> {
        if !self.is_reactive(root)? {
            return Ok(false);
        }
        if self.fact(root)?.dims.first() == Some(&ANY_LENGTH) {
            return Ok(true);
        }
        if self.ir.node(root)?.0 != Op::Slice {
            return Ok(false);
        }
        for bound in self.bounds(root)? {
            if bound != NONE && self.is_reactive(bound)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
