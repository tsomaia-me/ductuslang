use symphonyscript_core::Span;

use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::reason::Reason;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::collection::BuiltinName;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{ItemRec, NONE, Op, TopRec};
use crate::semantics::reporting::grouped;

impl Core<'_> {
    pub fn vocabulary_clash(&self, text: &str) -> Option<&'static str> {
        if BuiltinName::all().any(|builtin| builtin.name() == text) {
            return Some("a built-in name");
        }
        if self.vocabulary.node_kind_named(text).is_some() {
            return Some("a node kind of the vocabulary");
        }
        if self.vocabulary.wire_kind_named(text).is_some() {
            return Some("a wire kind of the vocabulary");
        }
        if self.vocabulary.enum_named(text).is_some() {
            return Some("an enum of the vocabulary");
        }
        None
    }

    fn top_name(&self, top: TopRec) -> Result<Option<(u32, DeclRef)>, Internal> {
        Ok(match top {
            TopRec::Let(at) => {
                let rec = self
                    .ir
                    .lets
                    .get(at as usize)
                    .ok_or(Internal::missed("IR let"))?;
                Some((self.ir.binder(rec.binder)?.name, DeclRef::Let(at)))
            }
            TopRec::Placement(item) => match self.ir.item(item)? {
                ItemRec::Placement(placement) if placement.as_name != NONE => {
                    Some((placement.as_name, DeclRef::Instance(item)))
                }
                ItemRec::Placement(_)
                | ItemRec::Wire(_)
                | ItemRec::Bundle(_)
                | ItemRec::For(_)
                | ItemRec::When(_)
                | ItemRec::Line(_)
                | ItemRec::Paren(_) => None,
            },
            TopRec::Cell(at) => {
                let rec = self
                    .ir
                    .cells
                    .get(at as usize)
                    .ok_or(Internal::missed("IR cell"))?;
                Some((self.ir.binder(rec.binder)?.name, DeclRef::Cell(at)))
            }
            TopRec::Fn(at) => {
                let rec = self
                    .ir
                    .fns
                    .get(at as usize)
                    .ok_or(Internal::missed("IR function"))?;
                Some((rec.name, DeclRef::Fn(at)))
            }
            TopRec::Pattern(_) | TopRec::Placeholder { .. } | TopRec::Drawn(_) | TopRec::Skip => {
                None
            }
        })
    }

    fn faulty_line(&self, top: TopRec) -> bool {
        match top {
            TopRec::Let(at) => self
                .ir
                .lets
                .get(at as usize)
                .is_some_and(|rec| rec.faulty_line),
            TopRec::Cell(at) => self
                .ir
                .cells
                .get(at as usize)
                .is_some_and(|rec| rec.faulty_line),
            TopRec::Fn(at) => self
                .ir
                .fns
                .get(at as usize)
                .is_some_and(|rec| rec.faulty_line),
            TopRec::Pattern(_)
            | TopRec::Placement(_)
            | TopRec::Placeholder { .. }
            | TopRec::Drawn(_)
            | TopRec::Skip => false,
        }
    }

    fn declared_name(&self, top: TopRec) -> Result<Option<(u32, DeclRef)>, Internal> {
        if let TopRec::Placeholder { name, .. } = top {
            return Ok(self.ir.named(name).then_some((name, DeclRef::Duplicate)));
        }
        let Some((name, decl)) = self.top_name(top)? else {
            return Ok(None);
        };
        if !self.ir.named(name) {
            return Ok(None);
        }
        if self.faulty_declaration(top)? {
            return Ok(Some((name, DeclRef::Duplicate)));
        }
        Ok(Some((name, decl)))
    }

    pub fn faulty_declaration(&self, top: TopRec) -> Result<bool, Internal> {
        Ok(self.faulty_line(top) || self.faulty_cell(top)?)
    }

    fn faulty_cell(&self, top: TopRec) -> Result<bool, Internal> {
        let TopRec::Cell(at) = top else {
            return Ok(false);
        };
        let root = self.cell_root(at)?;
        Ok(root != NONE && self.ir.node(root)?.0 == Op::Bad)
    }

    pub fn name_span(&self, file: u32, name: u32) -> Result<Span, Internal> {
        let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        Ok(self.span(file, entry.lo, entry.hi))
    }

    pub fn collect(&mut self) -> Stage<()> {
        self.mark_bodies()?;
        self.decl = vec![DeclRef::None; self.ir.symbols.end() as usize];
        let mut first: Vec<Option<Span>> = vec![None; self.decl.len()];
        for (file, rec) in self.ir.files.iter().enumerate() {
            let file = u32::try_from(file).unwrap_or(0);
            for at in rec.top.0..rec.top.1 {
                let top = *self
                    .ir
                    .top
                    .get(at as usize)
                    .ok_or(Internal::missed("IR top item"))?;
                if let TopRec::Let(index) = top
                    && let Some(slot) = self.let_file.get_mut(index as usize)
                {
                    *slot = file;
                }
                let Some((name, decl)) = self.declared_name(top)? else {
                    continue;
                };
                let entry = *self.ir.name(name).ok_or(Internal::missed("IR name"))?;
                let text = self.ir.symbols.text(entry.sym).to_string();
                let span = self.name_span(file, name)?;
                let slot = entry.sym.0 as usize;
                if let Some(owner) = self.variants.owner(&text) {
                    let owner = self
                        .vocabulary
                        .enum_decl(owner)
                        .ok_or(Internal::missed("enum"))?
                        .name
                        .clone();
                    self.draw(
                        Code::VariantName,
                        span,
                        format!(
                            "`{text}` is a variant of the enum `{owner}` and is never a declared name"
                        ),
                    );
                    if let Some(decl) = self.decl.get_mut(slot) {
                        *decl = DeclRef::Duplicate;
                    }
                    continue;
                }
                if let Some(what) = self.vocabulary_clash(&text) {
                    self.draw(Code::DuplicateName, span, format!("`{text}` is {what}"));
                    if let Some(decl) = self.decl.get_mut(slot) {
                        *decl = DeclRef::Duplicate;
                    }
                    continue;
                }
                if let Some(earlier) = first.get(slot).cloned().flatten() {
                    self.draw_related(
                        Code::DuplicateName,
                        span,
                        vec![(earlier, "first declared here".to_string())],
                        format!("`{text}` is declared twice"),
                    );
                    if let Some(decl) = self.decl.get_mut(slot) {
                        *decl = DeclRef::Duplicate;
                    }
                    continue;
                }
                if let Some(seen) = first.get_mut(slot) {
                    *seen = Some(span);
                }
                if let Some(slot) = self.decl.get_mut(slot) {
                    *slot = decl;
                }
            }
        }
        if let Err(declined) = self.bind_names()? {
            return Ok(Err(declined));
        }
        if self.calls_a_non_function()? {
            return Ok(Err(Declined(Reason::FunctionValue)));
        }
        if self.recursive()? {
            return Ok(Err(Declined(Reason::Recursion)));
        }
        if let Some(reason) = self.local_faults()? {
            return Ok(Err(Declined(reason)));
        }
        let mut inlined = 0u64;
        let mut warned = false;
        for (call, callee, count) in self.inline_charges()? {
            inlined = inlined.saturating_add(count);
            let name = self.function_name(callee)?;
            if inlined > self.limits.inlined_calls {
                let message = format!(
                    "this call of `{name}` makes the compilation inline more than {} calls",
                    grouped(self.limits.inlined_calls)
                );
                self.stop_at(Code::InlineLimit, call, message)?;
                break;
            }
            if !warned && inlined > self.limits.inlined_calls_warning {
                warned = true;
                let counted = format!(
                    "this call of `{name}` makes the compilation inline more than {} calls",
                    grouped(self.limits.inlined_calls_warning)
                );
                self.warn_at(call, "inlined_calls", counted, self.limits.inlined_calls)?;
            }
        }
        Ok(Ok(()))
    }

    fn bind_names(&mut self) -> Stage<()> {
        let skipped_top = self.skipped_top();
        self.skipped_top.clone_from(&skipped_top);
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Name)
                || !self.reached.get(local).copied().unwrap_or(false)
            {
                continue;
            }
            CountedWalk::BindLookup.step();
            let node = self.ir.base + u32::try_from(local).unwrap_or(u32::MAX);
            let name = *self.ir.a.get(local).ok_or(Internal::missed("IR name"))?;
            let entry = *self.ir.name(name).ok_or(Internal::missed("IR name"))?;
            if self.enclosing_hit(node, entry.sym.0)? && self.scoped(node, entry.sym.0)?.is_none() {
                return Ok(Err(Declined(Reason::NestedScope)));
            }
            let mut decl = match self.scoped(node, entry.sym.0)? {
                Some(decl) => decl,
                None => self
                    .decl
                    .get(entry.sym.0 as usize)
                    .copied()
                    .unwrap_or(DeclRef::None),
            };
            if decl == DeclRef::None
                && !self.names_the_vocabulary(entry.sym)
                && self.may_declare(node, entry.sym.0, &skipped_top)?
            {
                decl = DeclRef::Duplicate;
            }
            match decl {
                DeclRef::Let(_) | DeclRef::Cell(_) | DeclRef::Param(_) | DeclRef::Fn(_) => {}
                DeclRef::Duplicate => self.placeholders.push(node),
                DeclRef::Instance(_) => return Ok(Err(Declined(Reason::NameOfInstance))),
                DeclRef::None => {
                    let text = self.ir.symbols.text(entry.sym).to_string();
                    if self.vocabulary_clash(&text).is_some()
                        || self.vocabulary.native_named(&text).is_some()
                        || self.variants.owner(&text).is_some()
                    {
                        let reason = if self.variants.owner(&text).is_some() {
                            Reason::BareVariant
                        } else {
                            Reason::NameOfVocabulary
                        };
                        return Ok(Err(Declined(reason)));
                    }
                    let span = self.node_span(node)?;
                    self.draw(
                        Code::UnknownName,
                        span,
                        format!("`{text}` names nothing in scope"),
                    );
                    self.placeholders.push(node);
                }
            }
            if let Some(slot) = self.bind.get_mut(local) {
                *slot = decl;
            }
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    fn region_syms(&self, file: u32, regions: (u32, u32)) -> Vec<u32> {
        let Some(rec) = self.ir.files.get(file as usize) else {
            return Vec::new();
        };
        let order = self
            .ir
            .name_order
            .get(rec.names.0 as usize..rec.names.1 as usize)
            .unwrap_or_default();
        let lo_of = |index: &u32| self.ir.names.get(*index as usize).map_or(0, |name| name.lo);
        let mut syms = Vec::new();
        for at in regions.0..regions.1 {
            let Some(region) = self.ir.regions.get(at as usize) else {
                continue;
            };
            let from = order.partition_point(|index| lo_of(index) < region.0);
            for index in order.get(from..).unwrap_or_default() {
                if lo_of(index) >= region.1 {
                    break;
                }
                if let Some(name) = self.ir.names.get(*index as usize) {
                    syms.push(name.sym.0);
                }
            }
        }
        syms
    }

    pub fn skipped_top(&self) -> Vec<u32> {
        let mut syms = Vec::new();
        for (file, rec) in self.ir.files.iter().enumerate() {
            syms.extend(self.region_syms(u32::try_from(file).unwrap_or(0), rec.regions));
        }
        syms.sort_unstable();
        syms.dedup();
        syms
    }

    fn may_declare(&self, node: u32, sym: u32, skipped_top: &[u32]) -> Result<bool, Internal> {
        let mut owner = self.body_of.get(self.local(node)?).copied().unwrap_or(NONE);
        while owner != NONE {
            if owner < self.functions() {
                let rec = self
                    .ir
                    .fns
                    .get(owner as usize)
                    .ok_or(Internal::missed("IR function"))?;
                if self
                    .region_syms(self.file_of(rec.value), rec.regions)
                    .contains(&sym)
                {
                    return Ok(true);
                }
                break;
            }
            let fold = self
                .fold_nodes
                .get((owner - self.functions()) as usize)
                .copied()
                .unwrap_or(NONE);
            if fold == NONE {
                break;
            }
            owner = self.body_of.get(self.local(fold)?).copied().unwrap_or(NONE);
        }
        Ok(skipped_top.binary_search(&sym).is_ok())
    }
}

impl Core<'_> {
    fn names_the_vocabulary(&self, sym: crate::semantics::ir::flat::Sym) -> bool {
        let text = self.ir.symbols.text(sym);
        self.vocabulary_clash(text).is_some()
            || self.vocabulary.native_named(text).is_some()
            || self.variants.owner(text).is_some()
    }
}
