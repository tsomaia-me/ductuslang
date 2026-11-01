use std::collections::{BTreeMap, BTreeSet};

use super::kept::Kept;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::{
    AttrValue, BlockId, BodyItem, ExprId, ExprKind, Hir, Iterable, Placement, Top,
};

pub(super) struct TwinProgram {
    pub hir: Hir,
    pub placeholders: Vec<ExprId>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Item {
    Top(usize),
    Body(BlockId, usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shape {
    Container,
    Whole,
}

struct Entry {
    parent: Option<Item>,
    shape: Shape,
    declared: Vec<String>,
    nested: Vec<BlockId>,
    refs: BTreeSet<String>,
}

#[derive(Default)]
struct Index {
    entries: BTreeMap<Item, Entry>,
    declarers: BTreeMap<String, Vec<Item>>,
    referrers: BTreeMap<String, Vec<Item>>,
    holding: BTreeSet<Item>,
    owned: BTreeSet<ExprId>,
    field_base: bool,
}

impl TwinProgram {
    pub(super) fn of(
        hir: &Hir,
        placeholders: &[ExprId],
        referenced: &[String],
    ) -> Result<Option<TwinProgram>, Internal> {
        let wanted: BTreeSet<ExprId> = placeholders.iter().copied().collect();
        let mut index = Index::default();
        for (position, top) in hir.top.iter().enumerate() {
            if !hir.is_prelude_top(position) {
                index.top_item(hir, position, *top, &wanted)?;
            }
        }
        if index.owned.len() != wanted.len() {
            return Ok(None);
        }
        let kept = index.closure(hir, referenced)?;
        kept.compacted(hir, placeholders).map(Some)
    }
}

impl Index {
    fn top_item(
        &mut self,
        hir: &Hir,
        position: usize,
        top: Top,
        wanted: &BTreeSet<ExprId>,
    ) -> Result<(), Internal> {
        let item = Item::Top(position);
        let (declares, shape, exprs, nested, kind) = match top {
            Top::Input(id) => {
                let decl = hir.input(id)?;
                (
                    decl.name.clone(),
                    Shape::Whole,
                    vec![decl.value],
                    Vec::new(),
                    None,
                )
            }
            Top::Let(id) => {
                let decl = hir.let_decl(id)?;
                (
                    decl.name.clone(),
                    Shape::Whole,
                    vec![decl.value],
                    Vec::new(),
                    None,
                )
            }
            Top::State(id) => {
                let decl = hir.state(id)?;
                let mut exprs: Vec<ExprId> = decl.init.iter().copied().collect();
                exprs.extend(decl.clock.iter().copied());
                exprs.push(decl.value);
                (decl.name.clone(), Shape::Whole, exprs, Vec::new(), None)
            }
            Top::Fn(id) => {
                let decl = hir.fn_decl(id)?;
                let mut exprs: Vec<ExprId> = decl
                    .params
                    .iter()
                    .filter_map(|param| param.default)
                    .collect();
                exprs.extend(decl.lets.iter().map(|local| local.value));
                exprs.push(decl.value);
                (decl.name.clone(), Shape::Whole, exprs, Vec::new(), None)
            }
            Top::Pattern(id) => {
                let decl = hir.pattern(id)?;
                let mut exprs: Vec<ExprId> = decl
                    .params
                    .iter()
                    .filter_map(|param| param.default)
                    .collect();
                exprs.extend(decl.lets.iter().map(|local| local.value));
                (
                    decl.name.clone(),
                    Shape::Whole,
                    exprs,
                    vec![decl.body],
                    None,
                )
            }
            Top::Placement(id) => {
                let placement = hir.placement(id)?;
                (
                    placement.name.clone(),
                    Shape::Container,
                    head(placement),
                    placement.body.iter().copied().collect(),
                    placement.kind.clone(),
                )
            }
            Top::Placeholder(_) => (None, Shape::Whole, Vec::new(), Vec::new(), None),
        };
        let declared: Vec<String> = declares.into_iter().map(|name| name.text).collect();
        let nested = self.enter(
            hir,
            item,
            (None, shape),
            (
                exprs,
                nested,
                declared,
                kind.into_iter().map(|kind| kind.text).collect(),
            ),
            wanted,
        )?;
        self.blocks(hir, nested, item, wanted)
    }

    fn enter(
        &mut self,
        hir: &Hir,
        item: Item,
        (parent, shape): (Option<Item>, Shape),
        (exprs, nested, declared, kinds): (Vec<ExprId>, Vec<BlockId>, Vec<String>, Vec<String>),
        wanted: &BTreeSet<ExprId>,
    ) -> Result<Vec<BlockId>, Internal> {
        let mut refs: BTreeSet<String> = kinds.into_iter().collect();
        let mut stack = exprs;
        while let Some(id) = stack.pop() {
            CountedWalk::NameTwinsScan.step();
            if wanted.contains(&id) && self.owned.insert(id) {
                self.holding.insert(item);
            }
            let expr = hir.expr(id)?;
            match &expr.kind {
                ExprKind::Name(name) => {
                    refs.insert(name.clone());
                }
                ExprKind::Field(base, member) => {
                    self.field_base |= wanted.contains(base);
                    refs.insert(member.text.clone());
                }
                ExprKind::History { target, .. } | ExprKind::Stepped(target) => {
                    refs.insert(target.text.clone());
                }
                ExprKind::Error
                | ExprKind::Number(..)
                | ExprKind::Bool(_)
                | ExprKind::Str(_)
                | ExprKind::Variant { .. }
                | ExprKind::To
                | ExprKind::Convert(..)
                | ExprKind::Array(_)
                | ExprKind::Index(..)
                | ExprKind::IndexFromEnd(..)
                | ExprKind::Slice { .. }
                | ExprKind::Call(..)
                | ExprKind::Unary(..)
                | ExprKind::Binary(..)
                | ExprKind::If(..)
                | ExprKind::Match { .. }
                | ExprKind::For(_) => {}
            }
            stack.extend(expr.kind.children());
        }
        for name in &declared {
            self.declarers.entry(name.clone()).or_default().push(item);
        }
        for name in &refs {
            self.referrers.entry(name.clone()).or_default().push(item);
        }
        self.entries.insert(
            item,
            Entry {
                parent,
                shape,
                declared,
                nested: nested.clone(),
                refs,
            },
        );
        Ok(nested)
    }

    fn blocks(
        &mut self,
        hir: &Hir,
        nested: Vec<BlockId>,
        parent: Item,
        wanted: &BTreeSet<ExprId>,
    ) -> Result<(), Internal> {
        let mut pending: Vec<(BlockId, usize, Item)> = nested
            .into_iter()
            .rev()
            .map(|block| (block, 0, parent))
            .collect();
        while let Some((block, position, parent)) = pending.pop() {
            CountedWalk::NameTwinsIndexedBlocks.step();
            let Some(body_item) = hir.block(block)?.items.get(position) else {
                continue;
            };
            pending.push((block, position + 1, parent));
            let item = Item::Body(block, position);
            let (shape, exprs, nested, declared, kinds) = described(hir, body_item)?;
            let inner = self.enter(
                hir,
                item,
                (Some(parent), shape),
                (exprs, nested, declared, kinds),
                wanted,
            )?;
            pending.extend(inner.into_iter().rev().map(|block| (block, 0, item)));
        }
        Ok(())
    }

    fn closure(&self, hir: &Hir, referenced: &[String]) -> Result<Kept, Internal> {
        let mut kept = Kept {
            tops: vec![None; hir.top.len()],
            bodies: BTreeMap::new(),
        };
        for (position, top) in hir.top.iter().enumerate() {
            if (hir.is_prelude_top(position) || matches!(top, Top::Placeholder(_)))
                && let Some(slot) = kept.tops.get_mut(position)
            {
                *slot = Some(true);
            }
        }
        let mut work: Vec<(Item, bool, bool)> = self
            .holding
            .iter()
            .map(|item| (*item, false, true))
            .collect();
        for name in referenced {
            for item in self.declarers.get(name).into_iter().flatten() {
                if matches!(item, Item::Top(_)) {
                    work.push((*item, self.field_base, false));
                }
            }
        }
        let mut region: BTreeSet<Item> = BTreeSet::new();
        let mut read: BTreeSet<String> = BTreeSet::new();
        while let Some((item, whole, held)) = work.pop() {
            CountedWalk::NameTwinsRegion.step();
            let entry = self
                .entries
                .get(&item)
                .ok_or(Internal::missed("twin item"))?;
            let whole = whole || entry.shape != Shape::Container;
            let raised = raise(&mut kept, item, whole);
            let entered = held && region.insert(item);
            if raised {
                if let Some(parent) = entry.parent {
                    work.push((parent, false, false));
                }
                for name in &entry.refs {
                    for declarer in self.declarers.get(name).into_iter().flatten() {
                        work.push((*declarer, true, false));
                    }
                }
                if whole {
                    for block in &entry.nested {
                        let count = hir.block(*block)?.items.len();
                        work.extend(
                            (0..count).map(|position| (Item::Body(*block, position), true, false)),
                        );
                    }
                }
            }
            if !entered {
                continue;
            }
            let mut holder = Some(item);
            while let Some(at) = holder {
                let record = self.entries.get(&at).ok_or(Internal::missed("twin item"))?;
                for name in &record.declared {
                    if !read.insert(name.clone()) {
                        continue;
                    }
                    for reader in self.referrers.get(name).into_iter().flatten() {
                        work.push((*reader, false, true));
                    }
                }
                holder = record.parent;
            }
        }
        Ok(kept)
    }
}

fn head(placement: &Placement) -> Vec<ExprId> {
    let mut exprs: Vec<ExprId> = placement.default.iter().copied().collect();
    exprs.extend(placement.gate.iter().copied());
    exprs.extend(attr_exprs(&placement.attrs));
    exprs
}

fn attr_exprs(attrs: &[crate::semantics::ir::Attr]) -> impl Iterator<Item = ExprId> + '_ {
    attrs.iter().filter_map(|attr| match attr.value {
        AttrValue::Expr(expr) => Some(expr),
        AttrValue::Set(_) => None,
    })
}

fn iterable_exprs(iterable: Iterable) -> Vec<ExprId> {
    match iterable {
        Iterable::Range(start, end) => vec![start, end],
        Iterable::Values(values) => vec![values],
    }
}

fn raise(kept: &mut Kept, item: Item, whole: bool) -> bool {
    let current = match item {
        Item::Top(position) => kept.tops.get(position).copied().flatten(),
        Item::Body(block, position) => kept.bodies.get(&(block, position)).copied(),
    };
    if current == Some(true) || (current == Some(false) && !whole) {
        return false;
    }
    match item {
        Item::Top(position) => {
            if let Some(slot) = kept.tops.get_mut(position) {
                *slot = Some(whole);
            }
        }
        Item::Body(block, position) => {
            kept.bodies.insert((block, position), whole);
        }
    }
    true
}

type Described = (Shape, Vec<ExprId>, Vec<BlockId>, Vec<String>, Vec<String>);

fn described(hir: &Hir, body_item: &BodyItem) -> Result<Described, Internal> {
    Ok(match body_item {
        BodyItem::Placement(id) => {
            let placement = hir.placement(*id)?;
            (
                Shape::Container,
                head(placement),
                placement.body.iter().copied().collect(),
                placement
                    .name
                    .iter()
                    .map(|name| name.text.clone())
                    .collect(),
                placement
                    .kind
                    .iter()
                    .map(|kind| kind.text.clone())
                    .collect(),
            )
        }
        BodyItem::Wire(id) => {
            let wire = hir.wire(*id)?;
            let mut exprs: Vec<ExprId> = wire.default.iter().copied().collect();
            exprs.extend(wire.gate.iter().copied());
            exprs.extend(attr_exprs(&wire.attrs));
            exprs.push(wire.destination);
            (
                Shape::Whole,
                exprs,
                Vec::new(),
                wire.name.iter().map(|name| name.text.clone()).collect(),
                vec![wire.kind.text.clone()],
            )
        }
        BodyItem::Bundle { members, each, .. } => {
            let mut placements = members.clone();
            let mut exprs = Vec::new();
            if let Some(each) = each {
                placements.extend(each.members.iter().copied());
                exprs.extend(iterable_exprs(each.iterable));
            }
            let mut nested = Vec::new();
            let mut declared = Vec::new();
            let mut kinds = Vec::new();
            for id in placements {
                let placement = hir.placement(id)?;
                exprs.extend(head(placement));
                nested.extend(placement.body.iter().copied());
                declared.extend(placement.name.iter().map(|name| name.text.clone()));
                kinds.extend(placement.kind.iter().map(|kind| kind.text.clone()));
            }
            (Shape::Whole, exprs, nested, declared, kinds)
        }
        BodyItem::For { iterable, body, .. } => (
            Shape::Container,
            iterable_exprs(*iterable),
            vec![*body],
            Vec::new(),
            Vec::new(),
        ),
        BodyItem::When {
            arms, otherwise, ..
        } => (
            Shape::Container,
            arms.iter().map(|arm| arm.guard).collect(),
            arms.iter()
                .map(|arm| arm.body)
                .chain(otherwise.iter().copied())
                .collect(),
            Vec::new(),
            Vec::new(),
        ),
    })
}
