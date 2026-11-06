use symphonyscript_core::Span;

use super::core::Core;
use super::cycles::{shortest_cycle, strongly_connected};
use super::decl_ref::DeclRef;
use super::reason::Reason;
use crate::diagnostics::{Code, Diagnostic, Related};
use crate::semantics::Internal;
use crate::semantics::ir::flat::{ItemRec, NONE, Sym};

struct Bound {
    sym: u32,
    span: Span,
    decl: DeclRef,
}

impl Core<'_> {
    fn binder_span(&self, file: u32, binder: u32) -> Result<Span, Internal> {
        let name = self.ir.binder(binder)?.name;
        let entry = *self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        Ok(self.span(file, entry.lo, entry.hi))
    }

    fn owner_node(&self, owner: u32) -> Result<u32, Internal> {
        if owner >= self.functions() {
            return Ok(self.anchor(owner));
        }
        let rec = self
            .ir
            .fns
            .get(owner as usize)
            .ok_or(Internal::missed("IR function"))?;
        Ok(rec.value)
    }

    fn bound_names(&self, owner: u32) -> Result<Vec<Bound>, Internal> {
        let file = match self.pattern_at(owner) {
            Some(pattern) => self.pattern_file(pattern),
            None => self.file_of(self.owner_node(owner)?),
        };
        let (params, lets) = if self.loop_at(owner).is_some() {
            let head = self.loop_rec(owner)?.head;
            ((head.names.0..head.names.1).collect::<Vec<u32>>(), (0, 0))
        } else if let Some(pattern) = self.pattern_at(owner) {
            let params = self.pattern_parts(pattern)?.0;
            ((params.0..params.1).collect::<Vec<u32>>(), (0, 0))
        } else if owner < self.functions() {
            let rec = self
                .ir
                .fns
                .get(owner as usize)
                .ok_or(Internal::missed("IR function"))?;
            ((rec.params.0..rec.params.1).collect::<Vec<u32>>(), rec.lets)
        } else {
            let rec = self
                .ir
                .fors
                .get((owner - self.functions()) as usize)
                .ok_or(Internal::missed("IR for"))?;
            let mut params: Vec<u32> = (rec.names.0..rec.names.1).collect();
            params.push(rec.accumulator);
            (params, rec.lets)
        };
        let mut bound = Vec::new();
        for (position, binder) in params.into_iter().enumerate() {
            let name = self.ir.binder(binder)?.name;
            if !self.ir.named(name) {
                continue;
            }
            let sym = self.ir.name(name).ok_or(Internal::missed("IR name"))?.sym.0;
            bound.push(Bound {
                sym,
                span: self.binder_span(file, binder)?,
                decl: DeclRef::Param(u32::try_from(position).unwrap_or(0)),
            });
        }
        for at in lets.0..lets.1 {
            let rec = *self
                .ir
                .lets
                .get(at as usize)
                .ok_or(Internal::missed("IR let"))?;
            let name = self.ir.binder(rec.binder)?.name;
            if !self.ir.named(name) {
                continue;
            }
            let sym = self.ir.name(name).ok_or(Internal::missed("IR name"))?.sym.0;
            bound.push(Bound {
                sym,
                span: self.binder_span(self.file_of(rec.value), rec.binder)?,
                decl: DeclRef::Let(at),
            });
        }
        Ok(bound)
    }

    fn visible_span(&self, owner: u32, sym: u32) -> Result<Option<Span>, Internal> {
        let mut current = owner;
        while current != NONE && current >= self.functions() {
            let fold = self.owner_node(current)?;
            if fold == NONE {
                break;
            }
            current = self.body_of.get(self.local(fold)?).copied().unwrap_or(NONE);
            if current != NONE
                && let Some(found) = self
                    .bound_names(current)?
                    .into_iter()
                    .find(|bound| bound.sym == sym)
            {
                return Ok(Some(found.span));
            }
        }
        let binder = match self.decl.get(sym as usize).copied() {
            Some(DeclRef::Let(at)) => self.ir.lets.get(at as usize).map(|rec| rec.binder),
            Some(DeclRef::Cell(at)) => self.ir.cells.get(at as usize).map(|rec| rec.binder),
            Some(DeclRef::Fn(at)) => {
                return self.declared_span(
                    self.ir
                        .fns
                        .get(at as usize)
                        .map(|rec| (rec.name, rec.value)),
                );
            }
            Some(DeclRef::Pattern(at)) => {
                let declared = self
                    .ir
                    .patterns
                    .get(at as usize)
                    .map(|rec| (rec.name, self.ir.base));
                return self.declared_span(declared);
            }
            Some(DeclRef::Instance(item)) => {
                let declared = if let ItemRec::Placement(placement) = self.ir.item(item)? {
                    Some((placement.as_name, self.ir.base))
                } else {
                    None
                };
                return self.declared_span(declared);
            }
            Some(DeclRef::None | DeclRef::Duplicate | DeclRef::Param(_)) | None => return Ok(None),
        };
        let Some(binder) = binder else {
            return Ok(None);
        };
        let name = self.ir.binder(binder)?.name;
        let entry = *self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        let file = match self.decl.get(sym as usize).copied() {
            Some(DeclRef::Let(at)) => self.let_file.get(at as usize).copied().unwrap_or(0),
            Some(DeclRef::Cell(at)) => self.file_of(self.cell_root(at)?),
            Some(_) | None => self.file_of(self.ir.base),
        };
        Ok(Some(self.span(file, entry.lo, entry.hi)))
    }

    fn draw_local(
        &mut self,
        code: Code,
        span: Span,
        summary: String,
        help: &str,
        related: (Span, &str),
    ) {
        let diagnostic = Diagnostic::new(code, span, summary)
            .with_help(help)
            .with_related(vec![Related {
                span: related.0,
                label: related.1.to_string(),
            }]);
        self.diagnostics.push(diagnostic);
    }

    pub fn local_faults(&mut self) -> Result<Option<Reason>, Internal> {
        let enclosed = self.enclosed_names()?;
        for owner in 0..self.owners() {
            let fold = owner >= self.functions() && self.pattern_at(owner).is_none();
            let bound = self.bound_names(owner)?;
            for (position, name) in bound.iter().enumerate() {
                let text = self.ir.symbols.text(Sym(name.sym)).to_string();
                if let Some(owner_enum) = self.variants.owner(&text) {
                    let owner_enum = self
                        .vocabulary
                        .enum_decl(owner_enum)
                        .ok_or(Internal::missed("enum"))?
                        .name
                        .clone();
                    self.draw(
                        Code::VariantName,
                        name.span.clone(),
                        format!(
                            "`{text}` is a variant of the enum `{owner_enum}` and is never a declared name"
                        ),
                    );
                    continue;
                }
                let first = bound
                    .get(..position)
                    .and_then(|before| before.iter().find(|other| other.sym == name.sym));
                if let Some(first) = first {
                    let first_param = matches!(first.decl, DeclRef::Param(_));
                    let this_param = matches!(name.decl, DeclRef::Param(_));
                    let (summary, label) = if fold {
                        (
                            format!("`{text}` is bound twice by this `for`"),
                            "the first name",
                        )
                    } else if first_param && this_param {
                        (
                            format!("`{text}` names two parameters"),
                            "the first parameter",
                        )
                    } else {
                        (
                            format!("`{text}` is declared twice in one body"),
                            "first declared here",
                        )
                    };
                    self.draw_local(
                        Code::DuplicateLocal,
                        name.span.clone(),
                        summary,
                        "rename one of the two",
                        (first.span.clone(), label),
                    );
                    continue;
                }
                if !fold {
                    continue;
                }
                let enclosing = enclosed.contains(&(owner, name.sym));
                if !enclosing && let Some(child) = self.visible_child(owner, name.sym)? {
                    let summary =
                        format!("`{text}` equals a visible name, and a `for` binds only new names");
                    self.draw_local(
                        Code::ForVariableClash,
                        name.span.clone(),
                        summary,
                        "rename the name the `for` binds",
                        (child, "the visible name it equals"),
                    );
                    continue;
                }
                let top = self
                    .decl
                    .get(name.sym as usize)
                    .is_some_and(|decl| *decl != DeclRef::None);
                let vocabulary = self.vocabulary_clash(&text).is_some();
                if !(enclosing || top || vocabulary) {
                    continue;
                }
                let summary =
                    format!("`{text}` equals a visible name, and a `for` binds only new names");
                let duplicate = !enclosing
                    && self.decl.get(name.sym as usize).copied() == Some(DeclRef::Duplicate);
                if (vocabulary && !(enclosing || top)) || duplicate {
                    let diagnostic =
                        Diagnostic::new(Code::ForVariableClash, name.span.clone(), summary)
                            .with_help("rename the name the `for` binds");
                    self.diagnostics.push(diagnostic);
                    continue;
                }
                let Some(visible) = self.visible_span(owner, name.sym)? else {
                    return Ok(Some(Reason::LocalClash));
                };
                self.draw_local(
                    Code::ForVariableClash,
                    name.span.clone(),
                    summary,
                    "rename the name the `for` binds",
                    (visible, "the visible name it equals"),
                );
            }
            self.local_cycles(&bound)?;
        }
        Ok(None)
    }

    fn local_cycles(&mut self, bound: &[Bound]) -> Result<(), Internal> {
        let lets: Vec<u32> = bound
            .iter()
            .filter_map(|name| match name.decl {
                DeclRef::Let(at) => Some(at),
                DeclRef::None
                | DeclRef::Cell(_)
                | DeclRef::Instance(_)
                | DeclRef::Duplicate
                | DeclRef::Param(_)
                | DeclRef::Fn(_)
                | DeclRef::Pattern(_) => None,
            })
            .collect();
        let Some(&first) = lets.first() else {
            return Ok(());
        };
        let mut edges: Vec<Vec<u32>> = Vec::new();
        for at in &lets {
            let mut reads = Vec::new();
            for node in self.postorder(self.let_root(*at)?)? {
                if let DeclRef::Let(other) = self.bound(node)?
                    && lets.contains(&other)
                {
                    reads.push(other - first);
                }
            }
            edges.push(reads);
        }
        for component in strongly_connected(&edges) {
            let Some(&lowest) = component.iter().min() else {
                continue;
            };
            let looped = component.len() > 1
                || edges
                    .get(lowest as usize)
                    .is_some_and(|reads| reads.contains(&lowest));
            if !looped {
                continue;
            }
            for member in &component {
                if let Some(slot) = self.cyclic.get_mut((first + member) as usize) {
                    *slot = true;
                }
            }
            let cycle: Vec<u32> = shortest_cycle(&edges, &component, lowest)
                .into_iter()
                .map(|member| first + member)
                .collect();
            self.draw_cycle(&cycle)?;
        }
        Ok(())
    }
}

impl Core<'_> {
    fn visible_child(&self, owner: u32, sym: u32) -> Result<Option<Span>, Internal> {
        let fold = self.anchor(owner);
        if fold == NONE {
            return Ok(None);
        }
        let scope = self.scope_of.get(&fold).copied();
        let Some(child) = super::collect::child_in_scope(&self.child_scopes, scope, sym) else {
            return Ok(None);
        };
        let ItemRec::Placement(placement) = *self.ir.item(child)? else {
            return Ok(None);
        };
        Ok(Some(self.name_span(self.file_of(fold), placement.as_name)?))
    }
}

impl Core<'_> {
    fn declared_span(&self, declared: Option<(u32, u32)>) -> Result<Option<Span>, Internal> {
        let Some((name, node)) = declared else {
            return Ok(None);
        };
        if self.ir.files.len() != 1 || node == NONE {
            return Ok(None);
        }
        let entry = *self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        Ok(Some(self.span(self.file_of(node), entry.lo, entry.hi)))
    }
}
