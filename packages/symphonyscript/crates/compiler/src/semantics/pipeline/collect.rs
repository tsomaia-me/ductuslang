use symphonyscript_core::Span;

use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::collection::BuiltinName;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{ItemRec, NONE, Op, TopRec};

impl Core<'_> {
    fn vocabulary_clash(&self, text: &str) -> Option<&'static str> {
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
            TopRec::Cell(_)
            | TopRec::Fn(_)
            | TopRec::Pattern(_)
            | TopRec::Placeholder { .. }
            | TopRec::Drawn(_)
            | TopRec::Skip => None,
        })
    }

    fn name_span(&self, file: u32, name: u32) -> Result<Span, Internal> {
        let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        Ok(self.span(file, entry.lo, entry.hi))
    }

    pub fn collect(&mut self) -> Stage<()> {
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
                let Some((name, decl)) = self.top_name(top)? else {
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
        self.bind_names()
    }

    fn bind_names(&mut self) -> Stage<()> {
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Name) {
                continue;
            }
            CountedWalk::BindLookup.step();
            let node = self.ir.base + u32::try_from(local).unwrap_or(u32::MAX);
            let name = *self.ir.a.get(local).ok_or(Internal::missed("IR name"))?;
            let entry = *self.ir.name(name).ok_or(Internal::missed("IR name"))?;
            let decl = self
                .decl
                .get(entry.sym.0 as usize)
                .copied()
                .unwrap_or(DeclRef::None);
            match decl {
                DeclRef::Let(_) => {}
                DeclRef::Duplicate => self.placeholders.push(node),
                DeclRef::Instance(_) => return Ok(Err(Declined)),
                DeclRef::None => {
                    let text = self.ir.symbols.text(entry.sym).to_string();
                    if self.vocabulary_clash(&text).is_some()
                        || self.vocabulary.native_named(&text).is_some()
                        || self.variants.owner(&text).is_some()
                    {
                        return Ok(Err(Declined));
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
