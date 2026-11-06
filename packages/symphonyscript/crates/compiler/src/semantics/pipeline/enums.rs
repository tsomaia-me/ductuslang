use symphonyscript_core::EnumId;

use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::leaf::Leaf;
use super::reason::Reason;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{NONE, Op};
use crate::semantics::operators::{EvalTy, Value, ty_name};

impl Core<'_> {
    pub fn ty_text(&self, ty: EvalTy) -> Result<String, Internal> {
        Ok(match ty {
            EvalTy::Enum(id) => self.enum_name(id)?,
            other => ty_name(other).to_string(),
        })
    }

    fn enum_name(&self, id: EnumId) -> Result<String, Internal> {
        Ok(self
            .vocabulary
            .enum_decl(id)
            .ok_or(Internal::missed("enum"))?
            .name
            .clone())
    }

    pub fn is_bare(&self, node: u32) -> bool {
        self.bare.contains(&node)
    }

    pub fn variant_fact(&mut self, node: u32) -> Stage<Fact> {
        let (_, enumeration, variant, _) = self.ir.node(node)?;
        let enum_text = self.name_text(enumeration)?;
        let variant_text = self.name_text(variant)?;
        let Some(id) = self.vocabulary.enum_named(&enum_text) else {
            return Ok(Err(Declined(Reason::EnumFlow)));
        };
        if self.variants.position(id, &variant_text)?.is_none() {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        Ok(Ok(Fact::scalar(Leaf::Enum(Some(id)))))
    }

    pub fn variant_value(&self, node: u32) -> Result<super::core_value::CoreValue, Internal> {
        let (op, a, b, _) = self.ir.node(node)?;
        let (id, text) = if op == Op::Variant {
            let Some(id) = self.vocabulary.enum_named(&self.name_text(a)?) else {
                return Ok(Value::Error);
            };
            (id, self.name_text(b)?)
        } else {
            let Some(id) = self.bare_enums.get(&node).copied() else {
                return Ok(Value::Error);
            };
            (id, self.name_text(a)?)
        };
        Ok(match self.variants.position(id, &text)? {
            Some(position) => Value::Enum(id, position),
            None => Value::Error,
        })
    }

    fn bare_leaves(&self, root: u32) -> Result<Vec<u32>, Internal> {
        let mut found = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let (op, _, b, c) = self.ir.node(node)?;
            if op == Op::If {
                stack.extend([c, b]);
            } else if op == Op::Name && self.is_bare(node) {
                found.push(node);
            }
        }
        Ok(found)
    }

    pub fn resolve_bare(&mut self, root: u32, id: EnumId, draw: bool) -> Result<(), Internal> {
        for node in self.bare_leaves(root)? {
            if self.bare_enums.contains_key(&node) {
                continue;
            }
            let text = self.name_text(self.ir.node(node)?.1)?;
            if self.variants.position(id, &text)?.is_none() {
                if !draw {
                    continue;
                }
                let message = format!("`{text}` is not a variant of `{}`", self.enum_name(id)?);
                self.draw_at(Code::UnknownName, node, message)?;
                continue;
            }
            self.bare_enums.insert(node, id);
        }
        Ok(())
    }

    pub fn enum_gate(&mut self, node: u32, fact: &Fact) -> Stage<()> {
        if !self.enums_seen {
            return Ok(Ok(()));
        }
        let (op, a, b, c) = self.ir.node(node)?;
        for kid in self.operands(node)? {
            let leaf = self.fact(kid)?.leaf;
            let Leaf::Enum(known) = leaf else {
                continue;
            };
            let allowed = match op {
                Op::If => kid != a && (kid == b || kid == c),
                Op::Is | Op::IsNot => true,
                Op::Array | Op::Call | Op::NamedCall | Op::For => known.is_some(),
                Op::Past => known.is_some() && kid == a,
                Op::Index | Op::IndexFromEnd => known.is_some() && kid == a,
                Op::Convert
                | Op::Not
                | Op::Neg
                | Op::Add
                | Op::Sub
                | Op::Mul
                | Op::Div
                | Op::IntDiv
                | Op::Rem
                | Op::Pow
                | Op::Lt
                | Op::Le
                | Op::Gt
                | Op::Ge
                | Op::And
                | Op::Or => known.is_some(),
                Op::Int
                | Op::Float
                | Op::Bool
                | Op::Name
                | Op::Slice
                | Op::TyScalar
                | Op::TyArray
                | Op::TyName
                | Op::TyVar
                | Op::To
                | Op::Field
                | Op::Stepped
                | Op::Variant
                | Op::Bad
                | Op::Str
                | Op::Match
                | Op::MethodCall
                | Op::NamedMethodCall => false,
            };
            if !allowed || (known.is_none() && self.in_body(node)?.is_some()) {
                return Ok(Err(Declined(Reason::EnumFlow)));
            }
        }
        if matches!(op, Op::Is | Op::IsNot) {
            let (left, right) = (self.fact(a)?.leaf, self.fact(b)?.leaf);
            match (left, right) {
                (Leaf::Enum(Some(id)), Leaf::Enum(None)) => self.resolve_bare(b, id, true)?,
                (Leaf::Enum(None), Leaf::Enum(Some(id))) => self.resolve_bare(a, id, true)?,
                (Leaf::Enum(None), _) | (_, Leaf::Enum(None)) => {
                    return Ok(Err(Declined(Reason::BareVariant)));
                }
                _ => {}
            }
        }
        if op == Op::If
            && let Leaf::Enum(Some(id)) = fact.leaf
        {
            self.resolve_bare(node, id, false)?;
        }
        if op == Op::Name
            && let DeclRef::Let(at) = self.bound(node)?
            && self.has_fact(self.let_root(at)?)?
            && self.fact(self.let_root(at)?)?.leaf == Leaf::Enum(None)
        {
            return Ok(Err(Declined(Reason::BareVariant)));
        }
        if fact.leaf == Leaf::Enum(None) && self.in_body(node)?.is_some() {
            return Ok(Err(Declined(Reason::EnumFlow)));
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    pub fn check_variants(&mut self) -> Stage<()> {
        self.enums_seen = !self.bare.is_empty();
        for local in 0..self.ir.op.len() {
            let op = self.ir.op.get(local).copied();
            if matches!(op, Some(Op::Int | Op::Float))
                && let Some(suffix) = self.ir.b.get(local)
                && matches!(
                    self.ir.suffixes.get(*suffix as usize),
                    Some(crate::semantics::ir::Suffix::Unit { .. })
                )
            {
                self.units_seen = true;
            }
            if op != Some(Op::Variant) {
                continue;
            }
            self.enums_seen = true;
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            let (_, enumeration, variant, _) = self.ir.node(node)?;
            let enum_text = self.name_text(enumeration)?;
            let variant_text = self.name_text(variant)?;
            let Some(id) = self.vocabulary.enum_named(&enum_text) else {
                return Ok(Err(Declined(Reason::EnumFlow)));
            };
            if self.variants.position(id, &variant_text)?.is_some() {
                continue;
            }
            let entry = *self.ir.name(variant).ok_or(Internal::missed("IR name"))?;
            let span = self.span(self.file_of(node), entry.lo, entry.hi);
            self.draw(
                Code::UnknownVariant,
                span,
                format!("the enum `{enum_text}` has no variant `{variant_text}`"),
            );
            self.unknown_variants.insert(node);
        }
        for at in 0..self.ir.lets.len() {
            let at = u32::try_from(at).unwrap_or(u32::MAX);
            if let Some(EvalTy::Enum(id)) = self.annotation_ty(at)? {
                let root = self.let_root(at)?;
                self.resolve_bare(root, id, true)?;
            }
        }
        for at in 0..self.ir.cells.len() {
            let at = u32::try_from(at).unwrap_or(u32::MAX);
            if let Some(EvalTy::Enum(id)) = self.cell_annotation_ty(at)?
                && let Some(rec) = self.ir.cells.get(at as usize).copied()
            {
                for root in [rec.init, rec.rule] {
                    if root != NONE {
                        self.resolve_bare(root, id, true)?;
                    }
                }
            }
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    fn bare_valued(
        &self,
        root: u32,
        members: &std::collections::HashSet<u32>,
        aliases: &mut Vec<(u32, u32)>,
    ) -> Result<bool, Internal> {
        let mut stack = vec![root];
        let mut bare = false;
        while let Some(node) = stack.pop() {
            let (op, a, b, c) = self.ir.node(node)?;
            if op == Op::If {
                for read in self.postorder(a)? {
                    if self.ir.node(read)?.0 == Op::Name && !self.is_bare(read) {
                        return Ok(false);
                    }
                }
                stack.extend([c, b]);
            } else if op == Op::Name && self.is_bare(node) {
                bare = true;
            } else if op == Op::Name
                && let DeclRef::Let(at) = self.bound(node)?
                && members.contains(&at)
            {
                aliases.push((node, at));
            } else {
                return Ok(false);
            }
        }
        Ok(bare || !aliases.is_empty())
    }

    fn enum_attribute_roots(
        &self,
    ) -> Result<std::collections::HashMap<u32, Option<EnumId>>, Internal> {
        let mut roots = std::collections::HashMap::new();
        let mut items: Vec<(
            u32,
            u32,
            &[symphonyscript_core::AttrDecl],
            Option<symphonyscript_core::AttrId>,
        )> = Vec::new();
        for record in &self.instances {
            let placement = self.placement(record.item)?;
            let decl = self
                .vocabulary
                .node_kind(record.kind)
                .ok_or(Internal::missed("node kind"))?;
            items.push((
                placement.default,
                placement.attrs.0,
                &decl.attrs,
                decl.default_attr,
            ));
            items.push((NONE, placement.attrs.1, &decl.attrs, None));
        }
        for row in &self.wires {
            let rec = self.wire_rec(row.item)?;
            let decl = self
                .vocabulary
                .wire_kind(row.kind)
                .ok_or(Internal::missed("wire kind"))?;
            items.push((rec.default, rec.attrs.0, &decl.attrs, decl.default_attr));
            items.push((NONE, rec.attrs.1, &decl.attrs, None));
        }
        let enum_of = |attr: Option<&symphonyscript_core::AttrDecl>| match attr {
            Some(attr) if attr.ty.lengths.is_empty() && attr.ty.capacity.is_none() => {
                match attr.ty.scalar {
                    symphonyscript_core::Scalar::Enum(id) => Some(id),
                    _ => None,
                }
            }
            Some(_) | None => None,
        };
        for pair in items.chunks(2) {
            let [(default, start, attrs, default_attr), (_, end, _, _)] = pair else {
                continue;
            };
            if *default != NONE {
                let attr = default_attr.and_then(|attr| attrs.get(attr.index()));
                roots.insert(*default, enum_of(attr));
            }
            for at in *start..*end {
                let entry = *self
                    .ir
                    .attr_entries
                    .get(at as usize)
                    .ok_or(Internal::missed("IR attribute"))?;
                let crate::semantics::ir::flat::AttrForm::Value(root) = entry.form else {
                    continue;
                };
                let name = self
                    .ir
                    .name(entry.name)
                    .ok_or(Internal::missed("IR name"))?;
                let text = self.ir.symbols.text(name.sym);
                roots.insert(root, enum_of(attrs.iter().find(|attr| attr.name == text)));
            }
        }
        Ok(roots)
    }

    pub fn bare_declarations(&mut self) -> Result<(), Internal> {
        if self.bare.is_empty() {
            return Ok(());
        }
        let mut shaped: std::collections::HashMap<u32, Vec<(u32, u32)>> =
            std::collections::HashMap::new();
        let open: std::collections::HashSet<u32> = (0..self.ir.lets.len())
            .filter_map(|at| u32::try_from(at).ok())
            .collect();
        for (at, rec) in self.ir.lets.iter().enumerate() {
            let at = u32::try_from(at).unwrap_or(u32::MAX);
            let mut aliases = Vec::new();
            if rec.owner == NONE
                && !rec.is_match
                && self.annotation_ty(at)?.is_none()
                && self.bare_valued(rec.value, &open, &mut aliases)?
            {
                shaped.insert(at, aliases);
            }
        }
        let mut members: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut refused: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut order: Vec<u32> = shaped.keys().copied().collect();
        order.sort_unstable();
        for start in order {
            let mut stack = vec![(start, false)];
            let mut visiting = std::collections::HashSet::new();
            while let Some((at, done)) = stack.pop() {
                if members.contains(&at) || refused.contains(&at) {
                    continue;
                }
                let Some(aliases) = shaped.get(&at) else {
                    refused.insert(at);
                    continue;
                };
                if done {
                    visiting.remove(&at);
                    if aliases.iter().all(|(_, target)| members.contains(target)) {
                        members.insert(at);
                    } else {
                        refused.insert(at);
                    }
                    continue;
                }
                if !visiting.insert(at) {
                    refused.insert(at);
                    continue;
                }
                stack.push((at, true));
                for (_, target) in aliases {
                    if !members.contains(target) && !refused.contains(target) {
                        if visiting.contains(target) {
                            refused.insert(*target);
                        } else {
                            stack.push((*target, false));
                        }
                    }
                }
            }
        }
        if members.is_empty() {
            return Ok(());
        }
        let mut parent: std::collections::HashMap<u32, u32> =
            members.iter().map(|at| (*at, *at)).collect();
        let find = |parent: &std::collections::HashMap<u32, u32>, mut at: u32| {
            while let Some(up) = parent.get(&at).copied().filter(|up| *up != at) {
                at = up;
            }
            at
        };
        let mut internal = std::collections::HashSet::new();
        for at in &members {
            for (node, target) in shaped.get(at).map(Vec::as_slice).unwrap_or_default() {
                internal.insert(*node);
                let (x, y) = (find(&parent, *at), find(&parent, *target));
                if x != y {
                    parent.insert(x.max(y), x.min(y));
                }
            }
        }
        let mut reads: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
        for (local, op) in self.ir.op.iter().enumerate() {
            if *op != Op::Name {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            if internal.contains(&node) {
                continue;
            }
            if let DeclRef::Let(at) = self.bound(node)?
                && members.contains(&at)
            {
                reads.entry(find(&parent, at)).or_default().push(node);
            }
        }
        let attributes = self.enum_attribute_roots()?;
        let mut groups: Vec<u32> = members.iter().map(|at| find(&parent, *at)).collect();
        groups.sort_unstable();
        groups.dedup();
        for group in groups {
            let Some(found) = reads.get(&group) else {
                continue;
            };
            let mut wanted = None;
            let mut agreed = true;
            for read in found {
                match (attributes.get(read).copied().flatten(), wanted) {
                    (Some(id), None) => wanted = Some(id),
                    (Some(id), Some(seen)) if id == seen => {}
                    (Some(_) | None, _) => agreed = false,
                }
            }
            let Some(id) = wanted.filter(|_| agreed) else {
                continue;
            };
            let mut roots = Vec::new();
            for at in &members {
                if find(&parent, *at) == group {
                    roots.push(self.let_root(*at)?);
                }
            }
            for root in roots {
                self.resolve_bare(root, id, true)?;
            }
        }
        Ok(())
    }
}
