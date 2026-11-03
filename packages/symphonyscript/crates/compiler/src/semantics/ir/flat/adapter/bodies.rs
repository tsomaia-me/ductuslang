use super::super::attr_form::AttrForm;
use super::super::bundle_for_rec::BundleForRec;
use super::super::item_rec::ItemRec;
use super::super::iteration::Iteration;
use super::super::{ABSENT, NONE, index};
use super::hir_adapter::Adapter;
use crate::semantics::Internal;
use crate::semantics::ir::{
    Attr, AttrValue, Block, BlockId, BodyItem, BundleFor, Iterable, LoopVar, Placement,
    PlacementId, UnknownNames, WhenArm, Wire, WireId,
};

type Pending = Vec<(u32, BlockId)>;

impl Adapter<'_> {
    fn flags(&self, flags: (u32, u32)) -> Vec<(char, crate::semantics::ir::Loc)> {
        self.ir
            .flags
            .get(flags.0 as usize..flags.1 as usize)
            .unwrap_or_default()
            .iter()
            .map(|flag| (flag.flag, self.loc(flag.lo, flag.hi)))
            .collect()
    }

    fn attrs(&mut self, entries: (u32, u32)) -> Result<Vec<Attr>, Internal> {
        let mut attrs = Vec::new();
        for at in entries.0..entries.1 {
            let entry = *self
                .ir
                .attr_entries
                .get(at as usize)
                .ok_or(Internal::missed("IR attribute"))?;
            let Some(name) = self.ident(entry.name) else {
                continue;
            };
            let value = match entry.form {
                AttrForm::Value(root) => AttrValue::Expr(self.expr(root)?),
                AttrForm::Set(value) => AttrValue::Set(value),
            };
            attrs.push(Attr {
                name,
                value,
                loc: self.loc(entry.lo, entry.hi),
            });
        }
        Ok(attrs)
    }

    fn optional(&mut self, root: u32) -> Result<Option<crate::semantics::ir::ExprId>, Internal> {
        if root == NONE {
            return Ok(None);
        }
        self.expr(root).map(Some)
    }

    fn nested(&mut self, block: u32, pending: &mut Pending) -> BlockId {
        if block == ABSENT {
            return self.absent_block();
        }
        let id = self.empty_block();
        pending.push((block, id));
        id
    }

    pub fn placement(&mut self, item: u32, pending: &mut Pending) -> Result<PlacementId, Internal> {
        let Some(ItemRec::Placement(rec)) = self.ir.items.get(item as usize).copied() else {
            return Err(Internal::missed("IR placement"));
        };
        let loc = self.loc(rec.lo, rec.hi);
        let default = self.optional(rec.default)?;
        let gate = self.optional(rec.gate)?;
        let attrs = self.attrs(rec.attrs)?;
        let body = match rec.body {
            NONE => None,
            block => Some(self.nested(block, pending)),
        };
        let skipped_block = match rec.skipped_block {
            Some(region) => {
                let mut names = Vec::new();
                names.extend(self.ir.names_in(&self.rec, region).map(str::to_string));
                UnknownNames::of(names)
            }
            None => UnknownNames::default(),
        };
        let placement = Placement {
            kind: self.ident(rec.kind),
            flags: self.flags(rec.flags),
            name: self.ident(rec.as_name),
            has_as: rec.has_as,
            default,
            gate,
            attrs,
            body,
            skipped_block,
            inline: rec.inline,
            loc,
        };
        let id = PlacementId(index(self.hir.placements.len()));
        self.hir.placements.push(placement);
        Ok(id)
    }

    fn node_placement(
        &mut self,
        item: u32,
        pending: &mut Pending,
    ) -> Result<Option<PlacementId>, Internal> {
        if self.ir.dropped(item) {
            return Ok(None);
        }
        self.placement(item, pending).map(Some)
    }

    fn wire(&mut self, item: u32) -> Result<Option<WireId>, Internal> {
        let Some(ItemRec::Wire(rec)) = self.ir.items.get(item as usize).copied() else {
            return Err(Internal::missed("IR wire"));
        };
        let Some(kind) = self.ident(rec.kind) else {
            return Ok(None);
        };
        let loc = self.loc(rec.lo, rec.hi);
        let default = self.optional(rec.default)?;
        let gate = self.optional(rec.gate)?;
        let attrs = self.attrs(rec.attrs)?;
        let destination = self.expr(rec.destination)?;
        let wire = Wire {
            kind,
            flags: self.flags(rec.flags),
            name: self.ident(rec.as_name),
            default,
            gate,
            attrs,
            destination,
            loc,
        };
        let id = WireId(index(self.hir.wires.len()));
        self.hir.wires.push(wire);
        Ok(Some(id))
    }

    pub fn fill(&mut self, mut pending: Pending) -> Result<(), Internal> {
        while let Some((block, id)) = pending.pop() {
            let rec = *self
                .ir
                .blocks
                .get(block as usize)
                .ok_or(Internal::missed("IR block"))?;
            let mut items = Vec::new();
            for at in rec.items.0..rec.items.1 {
                let item = *self
                    .ir
                    .block_items
                    .get(at as usize)
                    .ok_or(Internal::missed("IR body item"))?;
                self.body_item(item, &mut items, &mut pending)?;
            }
            for at in rec.members.0..rec.members.1 {
                let member = *self
                    .ir
                    .members
                    .get(at as usize)
                    .ok_or(Internal::missed("IR member"))?;
                self.member(member, &mut items, &mut pending)?;
            }
            let mut unknown = self.region_names(rec.regions);
            unknown.every = items.is_empty();
            let slot = self
                .hir
                .blocks
                .get_mut(id.index())
                .ok_or(Internal::missed("block"))?;
            *slot = Block { items, unknown };
        }
        Ok(())
    }

    fn body_item(
        &mut self,
        item: u32,
        items: &mut Vec<BodyItem>,
        pending: &mut Pending,
    ) -> Result<(), Internal> {
        let rec = *self.ir.item(item)?;
        match rec {
            ItemRec::Placement(_) => {
                if let Some(id) = self.node_placement(item, pending)? {
                    items.push(BodyItem::Placement(id));
                }
            }
            ItemRec::Wire(_) => {
                if let Some(id) = self.wire(item)? {
                    items.push(BodyItem::Wire(id));
                }
            }
            ItemRec::Line(members) => {
                for at in members.0..members.1 {
                    let member = self.ir.members.get(at as usize).copied().unwrap_or(NONE);
                    self.member(member, items, pending)?;
                }
            }
            ItemRec::For(rec) => {
                let loc = self.loc(rec.lo, rec.hi);
                let (names, iterable) = self.loop_head(rec.head)?;
                let body = self.nested(rec.body, pending);
                items.push(BodyItem::For {
                    names,
                    iterable,
                    body,
                    loc,
                });
            }
            ItemRec::When(rec) => {
                let loc = self.loc(rec.lo, rec.hi);
                let mut arms = Vec::new();
                for at in rec.arms.0..rec.arms.1 {
                    let (guard, block) = *self
                        .ir
                        .arms
                        .get(at as usize)
                        .ok_or(Internal::missed("IR when arm"))?;
                    let guard = self.expr(guard)?;
                    let body = self.nested(block, pending);
                    arms.push(WhenArm { guard, body });
                }
                let otherwise = match rec.otherwise {
                    NONE => None,
                    block => Some(self.nested(block, pending)),
                };
                items.push(BodyItem::When {
                    arms,
                    otherwise,
                    loc,
                });
            }
            ItemRec::Bundle(_) | ItemRec::Paren(_) => self.member(item, items, pending)?,
        }
        Ok(())
    }

    fn member(
        &mut self,
        item: u32,
        items: &mut Vec<BodyItem>,
        pending: &mut Pending,
    ) -> Result<(), Internal> {
        match self.ir.items.get(item as usize).copied() {
            Some(ItemRec::Placement(_)) => {
                if let Some(id) = self.node_placement(item, pending)? {
                    items.push(BodyItem::Placement(id));
                }
            }
            Some(ItemRec::Paren(inner)) => {
                if inner != NONE
                    && let Some(id) = self.node_placement(inner, pending)?
                {
                    items.push(BodyItem::Placement(id));
                }
            }
            Some(ItemRec::Bundle(rec)) => {
                if rec.start_touched {
                    return Ok(());
                }
                let loc = self.loc(rec.lo, rec.hi);
                let Some(members) = self.bundle_members(rec.members, pending)? else {
                    return Ok(());
                };
                let each = match rec.each {
                    Some(BundleForRec {
                        head,
                        members,
                        lo,
                        hi,
                    }) => {
                        let each_loc = self.loc(lo, hi);
                        let Some(members) = self.bundle_members(members, pending)? else {
                            return Ok(());
                        };
                        let (names, iterable) = self.loop_head(head)?;
                        Some(BundleFor {
                            names,
                            iterable,
                            members,
                            loc: each_loc,
                        })
                    }
                    None => None,
                };
                items.push(BodyItem::Bundle {
                    members,
                    each,
                    faulty: rec.faulty,
                    loc,
                });
            }
            Some(_) | None => {}
        }
        Ok(())
    }

    fn bundle_members(
        &mut self,
        members: (u32, u32),
        pending: &mut Pending,
    ) -> Result<Option<Vec<PlacementId>>, Internal> {
        let mut ids = Vec::new();
        for at in members.0..members.1 {
            let member = self.ir.members.get(at as usize).copied().unwrap_or(NONE);
            let placement = match self.ir.items.get(member as usize) {
                Some(ItemRec::Placement(_)) => member,
                Some(ItemRec::Paren(inner)) if *inner != NONE => *inner,
                Some(_) | None => return Ok(None),
            };
            match self.node_placement(placement, pending)? {
                Some(id) => ids.push(id),
                None => return Ok(None),
            }
        }
        Ok(Some(ids))
    }

    fn loop_head(&mut self, head: Iteration) -> Result<(Vec<LoopVar>, Iterable), Internal> {
        let mut names = Vec::new();
        for at in head.names.0..head.names.1 {
            let binder = *self.ir.binder(at)?;
            let ty = self.annotation(binder.annotation)?;
            names.push(LoopVar {
                name: self.ident(binder.name),
                ty,
            });
        }
        let iterable = if head.range {
            let start = self.expr(head.iterable.0)?;
            Iterable::Range(start, self.expr(head.iterable.1)?)
        } else {
            Iterable::Values(self.expr(head.iterable.0)?)
        };
        Ok((names, iterable))
    }
}
