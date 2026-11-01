use std::collections::BTreeMap;

use super::twin_program::TwinProgram;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::{
    Args, Attr, AttrValue, Block, BlockId, BodyItem, BundleFor, Expr, ExprId, ExprKind, FnDecl,
    FnId, ForExpr, Hir, Input, InputId, Iterable, LetDecl, LetId, MatchArm, Param, PatternDecl,
    PatternId, Placement, PlacementId, SliceBound, StateDecl, StateId, StrPart, Top, WhenArm, Wire,
    WireId,
};

pub(super) struct Kept {
    pub tops: Vec<Option<bool>>,
    pub bodies: BTreeMap<(BlockId, usize), bool>,
}

struct Marks {
    exprs: Vec<bool>,
    blocks: BTreeMap<BlockId, Vec<usize>>,
    placements: Vec<bool>,
    wires: Vec<bool>,
    inputs: Vec<bool>,
    lets: Vec<bool>,
    states: Vec<bool>,
    fns: Vec<bool>,
    patterns: Vec<bool>,
    roots: Vec<ExprId>,
}

struct Remap {
    exprs: Vec<Option<u32>>,
    blocks: Vec<Option<u32>>,
    placements: Vec<Option<u32>>,
    wires: Vec<Option<u32>>,
    inputs: Vec<Option<u32>>,
    lets: Vec<Option<u32>>,
    states: Vec<Option<u32>>,
    fns: Vec<Option<u32>>,
    patterns: Vec<Option<u32>>,
}

impl Kept {
    pub(super) fn compacted(
        &self,
        hir: &Hir,
        placeholders: &[ExprId],
    ) -> Result<TwinProgram, Internal> {
        let mut marks = Marks {
            exprs: vec![false; hir.exprs.len()],
            blocks: BTreeMap::new(),
            placements: vec![false; hir.placements.len()],
            wires: vec![false; hir.wires.len()],
            inputs: vec![false; hir.inputs.len()],
            lets: vec![false; hir.lets.len()],
            states: vec![false; hir.states.len()],
            fns: vec![false; hir.fns.len()],
            patterns: vec![false; hir.patterns.len()],
            roots: Vec::new(),
        };
        for (position, top) in hir.top.iter().enumerate() {
            if let Some(Some(whole)) = self.tops.get(position) {
                marks.top(hir, *top, *whole, self)?;
            }
        }
        marks.close(hir)?;
        let remap = Remap {
            exprs: ranks(&marks.exprs),
            blocks: block_ranks(&marks.blocks, hir.blocks.len()),
            placements: ranks(&marks.placements),
            wires: ranks(&marks.wires),
            inputs: ranks(&marks.inputs),
            lets: ranks(&marks.lets),
            states: ranks(&marks.states),
            fns: ranks(&marks.fns),
            patterns: ranks(&marks.patterns),
        };
        let mut twin = Hir {
            files: hir.files.clone(),
            skipped_top: hir.skipped_top.clone(),
            drawn: hir.drawn.clone(),
            placeholders: hir.placeholders.clone(),
            prelude: hir.prelude,
            ..Hir::default()
        };
        for (index, expr) in hir.exprs.iter().enumerate() {
            CountedWalk::NameTwinsScan.step();
            if marks.exprs.get(index).copied().unwrap_or(false) {
                CountedWalk::NameTwinsCompacted.step();
                twin.exprs.push(Expr {
                    kind: remap.kind(&expr.kind)?,
                    loc: expr.loc,
                });
            }
        }
        for (block, items) in &marks.blocks {
            let old = hir.block(*block)?;
            let mut kept = Vec::with_capacity(items.len());
            for position in items {
                let item = old
                    .items
                    .get(*position)
                    .ok_or(Internal::missed("twin body item"))?;
                kept.push(remap.body_item(item)?);
            }
            twin.blocks.push(Block {
                items: kept,
                unknown: old.unknown.clone(),
            });
        }
        for (index, placement) in hir.placements.iter().enumerate() {
            CountedWalk::NameTwinsScan.step();
            if marks.placements.get(index).copied().unwrap_or(false) {
                twin.placements.push(remap.placement(placement)?);
            }
        }
        for (index, wire) in hir.wires.iter().enumerate() {
            CountedWalk::NameTwinsScan.step();
            if marks.wires.get(index).copied().unwrap_or(false) {
                twin.wires.push(remap.wire(wire)?);
            }
        }
        for (index, input) in hir.inputs.iter().enumerate() {
            CountedWalk::NameTwinsScan.step();
            if marks.inputs.get(index).copied().unwrap_or(false) {
                twin.inputs.push(Input {
                    value: remap.expr(input.value)?,
                    ..input.clone()
                });
            }
        }
        for (index, decl) in hir.lets.iter().enumerate() {
            CountedWalk::NameTwinsScan.step();
            if marks.lets.get(index).copied().unwrap_or(false) {
                twin.lets.push(remap.let_decl(decl)?);
            }
        }
        for (index, state) in hir.states.iter().enumerate() {
            CountedWalk::NameTwinsScan.step();
            if marks.states.get(index).copied().unwrap_or(false) {
                twin.states.push(remap.state(state)?);
            }
        }
        for (index, decl) in hir.fns.iter().enumerate() {
            CountedWalk::NameTwinsScan.step();
            if marks.fns.get(index).copied().unwrap_or(false) {
                twin.fns.push(remap.fn_decl(decl)?);
            }
        }
        for (index, decl) in hir.patterns.iter().enumerate() {
            CountedWalk::NameTwinsScan.step();
            if marks.patterns.get(index).copied().unwrap_or(false) {
                twin.patterns.push(remap.pattern(decl)?);
            }
        }
        for (position, top) in hir.top.iter().enumerate() {
            if let Some(Some(_)) = self.tops.get(position) {
                twin.top.push(remap.top(*top)?);
            }
        }
        Ok(TwinProgram {
            hir: twin,
            placeholders: placeholders
                .iter()
                .map(|id| remap.expr(*id))
                .collect::<Result<_, _>>()?,
        })
    }
}

fn ranks(marked: &[bool]) -> Vec<Option<u32>> {
    let mut next = 0;
    marked
        .iter()
        .map(|kept| {
            kept.then(|| {
                let rank = next;
                next += 1;
                rank
            })
        })
        .collect()
}

fn block_ranks(marked: &BTreeMap<BlockId, Vec<usize>>, count: usize) -> Vec<Option<u32>> {
    let mut found = vec![None; count];
    for (rank, block) in marked.keys().enumerate() {
        if let Some(slot) = found.get_mut(block.index()) {
            *slot = u32::try_from(rank).ok();
        }
    }
    found
}

fn look(map: &[Option<u32>], index: usize) -> Result<u32, Internal> {
    map.get(index)
        .copied()
        .flatten()
        .ok_or(Internal::missed("twin program id"))
}

fn set(marks: &mut [bool], index: usize) -> Result<(), Internal> {
    let slot = marks
        .get_mut(index)
        .ok_or(Internal::missed("twin program id"))?;
    *slot = true;
    Ok(())
}

impl Marks {
    fn top(&mut self, hir: &Hir, top: Top, whole: bool, kept: &Kept) -> Result<(), Internal> {
        let mut pending = Vec::new();
        match top {
            Top::Input(id) => {
                set(&mut self.inputs, id.index())?;
                self.roots.push(hir.input(id)?.value);
            }
            Top::Let(id) => {
                set(&mut self.lets, id.index())?;
                self.roots.push(hir.let_decl(id)?.value);
            }
            Top::State(id) => {
                set(&mut self.states, id.index())?;
                let state = hir.state(id)?;
                self.roots.extend(state.init.iter().copied());
                self.roots.extend(state.clock.iter().copied());
                self.roots.push(state.value);
            }
            Top::Fn(id) => {
                set(&mut self.fns, id.index())?;
                let decl = hir.fn_decl(id)?;
                self.params(&decl.params, &decl.lets);
                self.roots.push(decl.value);
            }
            Top::Pattern(id) => {
                set(&mut self.patterns, id.index())?;
                let decl = hir.pattern(id)?;
                self.params(&decl.params, &decl.lets);
                pending.push((decl.body, true));
            }
            Top::Placement(id) => self.placement(hir, id, whole, &mut pending)?,
            Top::Placeholder(_) => {}
        }
        self.blocks(hir, pending, kept)
    }

    fn params(&mut self, params: &[Param], lets: &[LetDecl]) {
        self.roots
            .extend(params.iter().filter_map(|param| param.default));
        self.roots.extend(lets.iter().map(|local| local.value));
    }

    fn attrs(&mut self, attrs: &[Attr]) {
        self.roots
            .extend(attrs.iter().filter_map(|attr| match attr.value {
                AttrValue::Expr(expr) => Some(expr),
                AttrValue::Set(_) => None,
            }));
    }

    fn iterable(&mut self, iterable: Iterable) {
        match iterable {
            Iterable::Range(start, end) => self.roots.extend([start, end]),
            Iterable::Values(values) => self.roots.push(values),
        }
    }

    fn placement(
        &mut self,
        hir: &Hir,
        id: PlacementId,
        whole: bool,
        pending: &mut Vec<(BlockId, bool)>,
    ) -> Result<(), Internal> {
        set(&mut self.placements, id.index())?;
        let placement = hir.placement(id)?;
        self.roots.extend(placement.default.iter().copied());
        self.roots.extend(placement.gate.iter().copied());
        self.attrs(&placement.attrs);
        pending.extend(placement.body.map(|body| (body, whole)));
        Ok(())
    }

    fn blocks(
        &mut self,
        hir: &Hir,
        mut pending: Vec<(BlockId, bool)>,
        kept: &Kept,
    ) -> Result<(), Internal> {
        while let Some((block, whole)) = pending.pop() {
            CountedWalk::NameTwinsMarkedBlocks.step();
            let mut items = Vec::new();
            for (position, item) in hir.block(block)?.items.iter().enumerate() {
                let mode = if whole {
                    Some(true)
                } else {
                    kept.bodies.get(&(block, position)).copied()
                };
                let Some(whole) = mode else {
                    continue;
                };
                items.push(position);
                self.body_item(hir, item, whole, &mut pending)?;
            }
            self.blocks.insert(block, items);
        }
        Ok(())
    }

    fn body_item(
        &mut self,
        hir: &Hir,
        item: &BodyItem,
        whole: bool,
        pending: &mut Vec<(BlockId, bool)>,
    ) -> Result<(), Internal> {
        match item {
            BodyItem::Placement(id) => self.placement(hir, *id, whole, pending),
            BodyItem::Wire(id) => {
                set(&mut self.wires, id.index())?;
                let wire = hir.wire(*id)?;
                self.roots.extend(wire.default.iter().copied());
                self.roots.extend(wire.gate.iter().copied());
                self.attrs(&wire.attrs);
                self.roots.push(wire.destination);
                Ok(())
            }
            BodyItem::Bundle { members, each, .. } => {
                for member in members {
                    self.placement(hir, *member, true, pending)?;
                }
                if let Some(each) = each {
                    self.iterable(each.iterable);
                    for member in &each.members {
                        self.placement(hir, *member, true, pending)?;
                    }
                }
                Ok(())
            }
            BodyItem::For { iterable, body, .. } => {
                self.iterable(*iterable);
                pending.push((*body, whole));
                Ok(())
            }
            BodyItem::When {
                arms, otherwise, ..
            } => {
                for arm in arms {
                    self.roots.push(arm.guard);
                    pending.push((arm.body, whole));
                }
                pending.extend(otherwise.map(|otherwise| (otherwise, whole)));
                Ok(())
            }
        }
    }

    fn close(&mut self, hir: &Hir) -> Result<(), Internal> {
        let mut stack = std::mem::take(&mut self.roots);
        while let Some(id) = stack.pop() {
            let slot = self
                .exprs
                .get_mut(id.index())
                .ok_or(Internal::missed("twin expression"))?;
            if *slot {
                continue;
            }
            CountedWalk::NameTwinsCompacted.step();
            *slot = true;
            stack.extend(hir.expr(id)?.kind.children());
        }
        Ok(())
    }
}

impl Remap {
    fn expr(&self, id: ExprId) -> Result<ExprId, Internal> {
        look(&self.exprs, id.index()).map(ExprId)
    }

    fn exprs(&self, ids: &[ExprId]) -> Result<Vec<ExprId>, Internal> {
        ids.iter().map(|id| self.expr(*id)).collect()
    }

    fn optional(&self, id: Option<ExprId>) -> Result<Option<ExprId>, Internal> {
        id.map(|id| self.expr(id)).transpose()
    }

    fn block(&self, id: BlockId) -> Result<BlockId, Internal> {
        look(&self.blocks, id.index()).map(BlockId)
    }

    fn placement_id(&self, id: PlacementId) -> Result<PlacementId, Internal> {
        look(&self.placements, id.index()).map(PlacementId)
    }

    fn placement_ids(&self, ids: &[PlacementId]) -> Result<Vec<PlacementId>, Internal> {
        ids.iter().map(|id| self.placement_id(*id)).collect()
    }

    fn slice_bound(&self, bound: Option<SliceBound>) -> Result<Option<SliceBound>, Internal> {
        bound
            .map(|bound| {
                Ok(SliceBound {
                    value: self.expr(bound.value)?,
                    from_end: bound.from_end,
                })
            })
            .transpose()
    }

    fn args(&self, args: &Args) -> Result<Args, Internal> {
        Ok(match args {
            Args::Positional(values) => Args::Positional(self.exprs(values)?),
            Args::Named(values) => Args::Named(
                values
                    .iter()
                    .map(|(name, value)| Ok((name.clone(), self.expr(*value)?)))
                    .collect::<Result<_, Internal>>()?,
            ),
            Args::Received(receiver, rest) => {
                Args::Received(self.expr(*receiver)?, Box::new(self.args(rest)?))
            }
        })
    }

    fn iterable(&self, iterable: Iterable) -> Result<Iterable, Internal> {
        Ok(match iterable {
            Iterable::Range(start, end) => Iterable::Range(self.expr(start)?, self.expr(end)?),
            Iterable::Values(values) => Iterable::Values(self.expr(values)?),
        })
    }

    fn let_decl(&self, decl: &LetDecl) -> Result<LetDecl, Internal> {
        Ok(LetDecl {
            value: self.expr(decl.value)?,
            ..decl.clone()
        })
    }

    fn lets(&self, lets: &[LetDecl]) -> Result<Vec<LetDecl>, Internal> {
        lets.iter().map(|decl| self.let_decl(decl)).collect()
    }

    fn params(&self, params: &[Param]) -> Result<Vec<Param>, Internal> {
        params
            .iter()
            .map(|param| {
                Ok(Param {
                    default: self.optional(param.default)?,
                    ..param.clone()
                })
            })
            .collect()
    }

    fn attrs(&self, attrs: &[Attr]) -> Result<Vec<Attr>, Internal> {
        attrs
            .iter()
            .map(|attr| {
                Ok(Attr {
                    value: match attr.value {
                        AttrValue::Expr(expr) => AttrValue::Expr(self.expr(expr)?),
                        AttrValue::Set(set) => AttrValue::Set(set),
                    },
                    ..attr.clone()
                })
            })
            .collect()
    }

    fn kind(&self, kind: &ExprKind) -> Result<ExprKind, Internal> {
        Ok(match kind {
            ExprKind::Error
            | ExprKind::Number(..)
            | ExprKind::Bool(_)
            | ExprKind::Name(_)
            | ExprKind::Variant { .. }
            | ExprKind::To
            | ExprKind::Stepped(_) => kind.clone(),
            ExprKind::Str(parts) => ExprKind::Str(
                parts
                    .iter()
                    .map(|part| match part {
                        StrPart::Text(text) => Ok(StrPart::Text(text.clone())),
                        StrPart::Hole(expr) => Ok(StrPart::Hole(self.expr(*expr)?)),
                    })
                    .collect::<Result<_, Internal>>()?,
            ),
            ExprKind::Convert(scalar, operand) => ExprKind::Convert(*scalar, self.expr(*operand)?),
            ExprKind::Array(elements) => ExprKind::Array(self.exprs(elements)?),
            ExprKind::Field(base, member) => ExprKind::Field(self.expr(*base)?, member.clone()),
            ExprKind::Index(base, index) => ExprKind::Index(self.expr(*base)?, self.expr(*index)?),
            ExprKind::IndexFromEnd(base, index) => {
                ExprKind::IndexFromEnd(self.expr(*base)?, self.expr(*index)?)
            }
            ExprKind::Slice { base, lower, upper } => ExprKind::Slice {
                base: self.expr(*base)?,
                lower: self.slice_bound(*lower)?,
                upper: self.slice_bound(*upper)?,
            },
            ExprKind::History {
                target,
                offset,
                fallback,
            } => ExprKind::History {
                target: target.clone(),
                offset: self.expr(*offset)?,
                fallback: self.optional(*fallback)?,
            },
            ExprKind::Call(callee, args) => ExprKind::Call(self.expr(*callee)?, self.args(args)?),
            ExprKind::Unary(op, operand) => ExprKind::Unary(*op, self.expr(*operand)?),
            ExprKind::Binary(op, lhs, rhs) => {
                ExprKind::Binary(*op, self.expr(*lhs)?, self.expr(*rhs)?)
            }
            ExprKind::If(condition, then, otherwise) => ExprKind::If(
                self.expr(*condition)?,
                self.expr(*then)?,
                self.expr(*otherwise)?,
            ),
            ExprKind::Match {
                scrutinee,
                arms,
                otherwise,
            } => ExprKind::Match {
                scrutinee: self.expr(*scrutinee)?,
                arms: arms
                    .iter()
                    .map(|arm| {
                        Ok(MatchArm {
                            label: self.expr(arm.label)?,
                            value: self.expr(arm.value)?,
                        })
                    })
                    .collect::<Result<_, Internal>>()?,
                otherwise: self.optional(*otherwise)?,
            },
            ExprKind::For(for_expr) => ExprKind::For(Box::new(ForExpr {
                iterable: self.iterable(for_expr.iterable)?,
                start: self.expr(for_expr.start)?,
                lets: self.lets(&for_expr.lets)?,
                body: self.expr(for_expr.body)?,
                ..(**for_expr).clone()
            })),
        })
    }

    fn body_item(&self, item: &BodyItem) -> Result<BodyItem, Internal> {
        Ok(match item {
            BodyItem::Placement(id) => BodyItem::Placement(self.placement_id(*id)?),
            BodyItem::Wire(id) => BodyItem::Wire(look(&self.wires, id.index()).map(WireId)?),
            BodyItem::Bundle {
                members,
                each,
                faulty,
                loc,
            } => BodyItem::Bundle {
                members: self.placement_ids(members)?,
                each: each
                    .as_ref()
                    .map(|each| {
                        Ok(BundleFor {
                            iterable: self.iterable(each.iterable)?,
                            members: self.placement_ids(&each.members)?,
                            ..each.clone()
                        })
                    })
                    .transpose()?,
                faulty: *faulty,
                loc: *loc,
            },
            BodyItem::For {
                names,
                iterable,
                body,
                loc,
            } => BodyItem::For {
                names: names.clone(),
                iterable: self.iterable(*iterable)?,
                body: self.block(*body)?,
                loc: *loc,
            },
            BodyItem::When {
                arms,
                otherwise,
                loc,
            } => BodyItem::When {
                arms: arms
                    .iter()
                    .map(|arm| {
                        Ok(WhenArm {
                            guard: self.expr(arm.guard)?,
                            body: self.block(arm.body)?,
                        })
                    })
                    .collect::<Result<_, Internal>>()?,
                otherwise: otherwise.map(|block| self.block(block)).transpose()?,
                loc: *loc,
            },
        })
    }

    fn placement(&self, placement: &Placement) -> Result<Placement, Internal> {
        Ok(Placement {
            default: self.optional(placement.default)?,
            gate: self.optional(placement.gate)?,
            attrs: self.attrs(&placement.attrs)?,
            body: placement.body.map(|body| self.block(body)).transpose()?,
            ..placement.clone()
        })
    }

    fn wire(&self, wire: &Wire) -> Result<Wire, Internal> {
        Ok(Wire {
            default: self.optional(wire.default)?,
            gate: self.optional(wire.gate)?,
            attrs: self.attrs(&wire.attrs)?,
            destination: self.expr(wire.destination)?,
            ..wire.clone()
        })
    }

    fn state(&self, state: &StateDecl) -> Result<StateDecl, Internal> {
        Ok(StateDecl {
            init: self.optional(state.init)?,
            clock: self.exprs(&state.clock)?,
            value: self.expr(state.value)?,
            ..state.clone()
        })
    }

    fn fn_decl(&self, decl: &FnDecl) -> Result<FnDecl, Internal> {
        Ok(FnDecl {
            params: self.params(&decl.params)?,
            lets: self.lets(&decl.lets)?,
            value: self.expr(decl.value)?,
            ..decl.clone()
        })
    }

    fn pattern(&self, decl: &PatternDecl) -> Result<PatternDecl, Internal> {
        Ok(PatternDecl {
            params: self.params(&decl.params)?,
            lets: self.lets(&decl.lets)?,
            body: self.block(decl.body)?,
            ..decl.clone()
        })
    }

    fn top(&self, top: Top) -> Result<Top, Internal> {
        Ok(match top {
            Top::Input(id) => Top::Input(look(&self.inputs, id.index()).map(InputId)?),
            Top::Let(id) => Top::Let(look(&self.lets, id.index()).map(LetId)?),
            Top::State(id) => Top::State(look(&self.states, id.index()).map(StateId)?),
            Top::Fn(id) => Top::Fn(look(&self.fns, id.index()).map(FnId)?),
            Top::Pattern(id) => Top::Pattern(look(&self.patterns, id.index()).map(PatternId)?),
            Top::Placement(id) => Top::Placement(self.placement_id(id)?),
            Top::Placeholder(index) => Top::Placeholder(index),
        })
    }
}
