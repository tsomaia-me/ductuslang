use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::instance_row::InstanceRow;
use super::reason::Reason;
use super::work::Work;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{ItemRec, NONE, PlacementRec, TopRec};

impl Core<'_> {
    pub fn placement(&self, item: u32) -> Result<PlacementRec, Internal> {
        match self.ir.item(item)? {
            ItemRec::Placement(placement) => Ok(*placement),
            ItemRec::Wire(_)
            | ItemRec::Bundle(_)
            | ItemRec::For(_)
            | ItemRec::When(_)
            | ItemRec::Line(_)
            | ItemRec::Paren(_) => Err(Internal::missed("IR placement")),
        }
    }

    fn row(&mut self, item: u32, file: u32, parent: u32, env: u32) -> Stage<u32> {
        CountedWalk::CoreExpandRow.step();
        let placement = self.placement(item)?;
        if self.used_pattern(placement.kind).is_some() {
            return Ok(Err(Declined(Reason::PatternUse)));
        }
        let Some(kind_name) = self
            .ir
            .name_text(placement.kind)
            .filter(|_| self.ir.named(placement.kind))
        else {
            return Ok(Ok(NONE));
        };
        let duplicated = self
            .ir
            .name(placement.kind)
            .is_some_and(|entry| self.decl.get(entry.sym.0 as usize) == Some(&DeclRef::Duplicate));
        let found = self
            .vocabulary
            .node_kind_named(kind_name)
            .filter(|_| !duplicated);
        let Some(kind) = found else {
            let reason = self.placement_kind_reason(kind_name);
            if let Some(reason) = reason.or(placement.inline.then_some(Reason::BodyItem)) {
                return Ok(Err(Declined(reason)));
            }
            if parent != NONE {
                self.unknown_children.insert(parent);
                self.unknown_kind = true;
                self.unknown_wires = true;
            } else if self.ir.named(placement.as_name) {
                self.unknown_top = true;
                self.unknown_kind = true;
                self.unknown_wires = true;
            }
            self.unknown_kind_drawn(file, placement.kind)?;
            if placement.body != NONE
                && let Err(declined) = self.unknown_body(placement.body, file)?
            {
                return Ok(Err(declined));
            }
            return Ok(Ok(NONE));
        };
        if parent == NONE && !self.ir.named(placement.as_name) {
            return Ok(Ok(NONE));
        }
        if !self.charge_item(file, placement.lo, placement.hi)? {
            return Ok(Ok(NONE));
        }
        let row = u32::try_from(self.instances.len()).map_err(|_| Internal::missed("row"))?;
        self.instances.push(InstanceRow {
            kind,
            item,
            file,
            children: (0, 0),
            env,
        });
        if let Some(slot) = self.row_of_item.get_mut(item as usize) {
            *slot = row;
        }
        Ok(Ok(row))
    }

    pub fn expand(&mut self) -> Stage<()> {
        self.named_in_loops()?;
        let mut pending: Vec<u32> = Vec::new();
        for (file, rec) in self.ir.files.iter().enumerate() {
            let file = u32::try_from(file).unwrap_or(0);
            for at in rec.top.0..rec.top.1 {
                let top = *self
                    .ir
                    .top
                    .get(at as usize)
                    .ok_or(Internal::missed("IR top item"))?;
                if let TopRec::Placement(item) = top {
                    let row = match self.row(item, file, NONE, NONE)? {
                        Ok(row) => row,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    if row == NONE {
                        continue;
                    }
                    self.top_instances.push(row);
                    pending.push(row);
                }
            }
        }
        let mut next = 0;
        while let Some(&parent) = pending.get(next) {
            next += 1;
            let record = *self
                .instances
                .get(parent as usize)
                .ok_or(Internal::missed("row"))?;
            let placement = self.placement(record.item)?;
            if placement.body == NONE {
                continue;
            }
            let first = u32::try_from(self.instances.len()).unwrap_or(u32::MAX);
            if let Err(declined) =
                self.expand_block(placement.body, record, parent, &mut pending)?
            {
                return Ok(Err(declined));
            }
            if self.stop.is_some() {
                return Ok(Ok(()));
            }
            let end = u32::try_from(self.instances.len()).unwrap_or(u32::MAX);
            if let Some(row) = self.instances.get_mut(parent as usize) {
                row.children = (first, end);
            }
        }
        Ok(Ok(()))
    }

    fn expand_block(
        &mut self,
        body: u32,
        record: InstanceRow,
        parent: u32,
        pending: &mut Vec<u32>,
    ) -> Stage<()> {
        let block = *self
            .ir
            .blocks
            .get(body as usize)
            .ok_or(Internal::missed("IR block"))?;
        let mut work = vec![Work::Items(block.items.0, block.items.1, record.env)];
        while let Some(job) = work.pop() {
            if self.stop.is_some() {
                return Ok(Ok(()));
            }
            let (item, env) = match job {
                Work::Items(at, end, env) => {
                    if at >= end {
                        continue;
                    }
                    work.push(Work::Items(at + 1, end, env));
                    let item = *self
                        .ir
                        .block_items
                        .get(at as usize)
                        .ok_or(Internal::missed("IR block item"))?;
                    (item, env)
                }
                Work::Members(at, end, env) => {
                    if at >= end {
                        continue;
                    }
                    work.push(Work::Members(at + 1, end, env));
                    let item = *self
                        .ir
                        .members
                        .get(at as usize)
                        .ok_or(Internal::missed("IR member"))?;
                    (item, env)
                }
                Work::Loop(mut run) => {
                    if run.next >= run.count {
                        continue;
                    }
                    if !self.loop_step(run.item)? {
                        return Ok(Ok(()));
                    }
                    let env = self.loop_env(&mut run)?;
                    let ItemRec::For(rec) = *self.ir.item(run.item)? else {
                        return Err(Internal::missed("IR for block"));
                    };
                    run.next += 1;
                    work.push(Work::Loop(run));
                    let block = *self
                        .ir
                        .blocks
                        .get(rec.body as usize)
                        .ok_or(Internal::missed("IR block"))?;
                    work.push(Work::Items(block.items.0, block.items.1, env));
                    continue;
                }
            };
            match *self.ir.item(item)? {
                ItemRec::Wire(_) => {
                    if let Err(declined) = self.wire_row(item, record.file, parent, env)? {
                        return Ok(Err(declined));
                    }
                }
                ItemRec::For(_) => {
                    let run = match self.loop_head(item, env)? {
                        Ok(run) => run,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    match run {
                        Some(run) if run.count > 0 => work.push(Work::Loop(run)),
                        run => {
                            if let Err(declined) = self.unrun_names(item, record.file)? {
                                return Ok(Err(declined));
                            }
                            if run.is_none_or(|run| run.open) {
                                self.open_body(item, parent)?;
                            }
                        }
                    }
                }
                ItemRec::Line(members) => work.push(Work::Members(members.0, members.1, env)),
                ItemRec::Placement(placement) => {
                    if let Some(pattern) = self.used_pattern(placement.kind) {
                        match self.use_env(item, pattern, record.file, env)? {
                            Ok(Some(made)) => {
                                let body = self.pattern_parts(pattern)?.1;
                                let block = *self
                                    .ir
                                    .blocks
                                    .get(body as usize)
                                    .ok_or(Internal::missed("IR block"))?;
                                work.push(Work::Items(block.items.0, block.items.1, made));
                            }
                            Ok(None) => {}
                            Err(declined) => return Ok(Err(declined)),
                        }
                        continue;
                    }
                    let row = match self.row(item, record.file, parent, env)? {
                        Ok(row) => row,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    if row != NONE {
                        pending.push(row);
                    }
                }
                ItemRec::Bundle(_) | ItemRec::When(_) | ItemRec::Paren(_) => {
                    return Ok(Err(Declined(Reason::BodyItem)));
                }
            }
        }
        Ok(Ok(()))
    }
}
impl Core<'_> {
    fn placement_kind_reason(&self, kind: &str) -> Option<Reason> {
        if self.vocabulary.wire_kind_named(kind).is_some() {
            Some(Reason::PatternOrWirePlacement)
        } else {
            None
        }
    }
}

impl Core<'_> {
    pub fn unknown_kind_drawn(&mut self, file: u32, name: u32) -> Result<(), Internal> {
        let entry = *self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        let found = self
            .decl
            .get(entry.sym.0 as usize)
            .copied()
            .unwrap_or(DeclRef::None);
        let silent = match found {
            DeclRef::Duplicate => true,
            DeclRef::None => self.skipped_top.binary_search(&entry.sym.0).is_ok(),
            DeclRef::Let(_)
            | DeclRef::Cell(_)
            | DeclRef::Instance(_)
            | DeclRef::Param(_)
            | DeclRef::Fn(_)
            | DeclRef::Pattern(_) => false,
        };
        if silent {
            return Ok(());
        }
        let text = self.ir.symbols.text(entry.sym).to_string();
        let known = found != DeclRef::None
            || self.vocabulary_clash(&text).is_some()
            || self.vocabulary.native_named(&text).is_some();
        let message = if known {
            format!("`{text}` is not a kind")
        } else {
            format!("`{text}` is not a node kind or a pattern")
        };
        let span = self.name_span(file, name)?;
        self.draw(crate::diagnostics::Code::UnknownKind, span, message);
        Ok(())
    }
}
