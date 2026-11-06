use super::core::Core;
use super::loops::iterable_roots;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{AttrForm, ItemRec, NONE, Op, TopRec};

impl Core<'_> {
    pub fn front_end_drawn(&mut self) -> Result<(), Internal> {
        let mut roots = Vec::new();
        let mut drawn = Vec::new();
        for (file, rec) in self.ir.files.iter().enumerate() {
            let file = u32::try_from(file).unwrap_or(0);
            for at in rec.top.0..rec.top.1 {
                let top = *self
                    .ir
                    .top
                    .get(at as usize)
                    .ok_or(Internal::missed("IR top item"))?;
                match top {
                    TopRec::Placeholder { drawn: range, .. } | TopRec::Drawn(range) => {
                        drawn.push((file, range));
                    }
                    TopRec::Let(at) => {
                        roots.push(self.let_root(at)?);
                    }
                    TopRec::Cell(at) => {
                        let rec = *self
                            .ir
                            .cells
                            .get(at as usize)
                            .ok_or(Internal::missed("IR cell"))?;
                        drawn.push((file, rec.drawn));
                        roots.extend([rec.init, rec.rule]);
                        for kid in rec.clock.0..rec.clock.1 {
                            roots.push(self.kid(kid)?);
                        }
                    }
                    TopRec::Fn(at) => {
                        let rec = *self
                            .ir
                            .fns
                            .get(at as usize)
                            .ok_or(Internal::missed("IR function"))?;
                        for binder in rec.params.0..rec.params.1 {
                            roots.push(self.ir.binder(binder)?.default);
                        }
                        roots.extend(self.function_roots(at)?);
                    }
                    TopRec::Placement(item) => self.placement_roots(item, &mut roots)?,
                    TopRec::Pattern(at) => {
                        let (params, body) = self.pattern_parts(at)?;
                        for binder in params.0..params.1 {
                            roots.push(self.ir.binder(binder)?.default);
                        }
                        let mut items = Vec::new();
                        self.block_children(body, &mut items)?;
                        for item in items {
                            self.placement_roots(item, &mut roots)?;
                        }
                    }
                    TopRec::Skip => {}
                }
            }
        }
        let mut stack: Vec<u32> = roots.into_iter().filter(|root| *root != NONE).collect();
        while let Some(node) = stack.pop() {
            let local = self.local(node)?;
            if let Some(seen) = self.reached.get_mut(local) {
                if *seen {
                    continue;
                }
                *seen = true;
            }
            let (op, a, b, _) = self.ir.node(node)?;
            if op == Op::Bad {
                drawn.push((self.file_of(node), (a, b)));
            } else if op == Op::For {
                stack.extend(self.function_roots(self.functions() + a)?);
            } else if matches!(op, Op::Call | Op::NamedCall | Op::Field) {
                stack.push(a);
            }
            stack.extend(self.operands(node)?.into_iter().filter(|at| *at != NONE));
        }
        for (file, range) in drawn {
            self.emit_drawn(file, range)?;
        }
        Ok(())
    }

    fn placement_roots(&self, item: u32, roots: &mut Vec<u32>) -> Result<(), Internal> {
        let mut items = vec![item];
        while let Some(item) = items.pop() {
            let (placement, body) = match *self.ir.item(item)? {
                ItemRec::Placement(placement) => (placement, placement.body),
                ItemRec::Wire(wire) => {
                    roots.extend([wire.default, wire.gate, wire.destination]);
                    self.entry_roots(wire.attrs, roots);
                    continue;
                }
                ItemRec::For(rec) => {
                    roots.extend(iterable_roots(rec.head));
                    self.block_children(rec.body, &mut items)?;
                    continue;
                }
                ItemRec::Line(members) => {
                    items.extend(
                        self.ir
                            .members
                            .get(members.0 as usize..members.1 as usize)
                            .unwrap_or_default(),
                    );
                    continue;
                }
                ItemRec::When(_) => {
                    roots.extend(self.when_guards(item));
                    items.extend(self.when_items(item));
                    continue;
                }
                ItemRec::Paren(inner) if inner != NONE => {
                    items.push(inner);
                    continue;
                }
                ItemRec::Bundle(_) | ItemRec::Paren(_) => continue,
            };
            roots.extend([placement.default, placement.gate]);
            self.entry_roots(placement.attrs, roots);
            self.block_children(body, &mut items)?;
        }
        Ok(())
    }

    fn block_children(&self, body: u32, items: &mut Vec<u32>) -> Result<(), Internal> {
        if body == NONE {
            return Ok(());
        }
        let Some(block) = self.ir.blocks.get(body as usize) else {
            return Ok(());
        };
        for at in block.items.0..block.items.1 {
            let child = *self
                .ir
                .block_items
                .get(at as usize)
                .ok_or(Internal::missed("IR block item"))?;
            if !self.ir.dropped(child) {
                items.push(child);
            }
        }
        Ok(())
    }

    fn entry_roots(&self, attrs: (u32, u32), roots: &mut Vec<u32>) {
        let entries = self
            .ir
            .attr_entries
            .get(attrs.0 as usize..attrs.1 as usize)
            .unwrap_or_default();
        for entry in entries {
            if let AttrForm::Value(value) = entry.form {
                roots.push(value);
            }
        }
    }

    fn emit_drawn(&mut self, file: u32, range: (u32, u32)) -> Result<(), Internal> {
        for at in range.0..range.1 {
            let entry = *self
                .ir
                .drawn
                .get(at as usize)
                .ok_or(Internal::missed("IR diagnostic"))?;
            let summary = self.ir.text(entry.summary).to_string();
            let span = self.span(file, entry.lo, entry.hi);
            self.draw(entry.code, span, summary);
        }
        Ok(())
    }
}
