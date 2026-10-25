use std::collections::{BTreeMap, BTreeSet};

use symphonyscript_core::Vocabulary;

use super::frame::Frame;
use super::frame_kind::FrameKind;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::collection::{Decl, Namespace, Variants};
use crate::semantics::ir::{
    AttrValue, BlockId, BodyItem, ExprId, ExprKind, FnId, Hir, Ident, Iterable, LetDecl, Loc,
    PatternId, PlacementId, TypeBase, TypeRef, UnknownNames,
};
use crate::semantics::reporting::Report;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    New,
    Open,
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Body,
    Arm,
    Anonymous,
}

pub(super) struct Checker<'a> {
    hir: &'a Hir,
    vocabulary: &'a Vocabulary,
    variants: &'a Variants,
    namespace: &'a Namespace,
    frames: Vec<Frame>,
    unknown_bodies: BTreeSet<BlockId>,
    report: &'a mut Report,
}

impl<'a> Checker<'a> {
    pub fn new(
        hir: &'a Hir,
        vocabulary: &'a Vocabulary,
        variants: &'a Variants,
        namespace: &'a Namespace,
        report: &'a mut Report,
    ) -> Self {
        Self {
            hir,
            vocabulary,
            variants,
            namespace,
            frames: Vec::new(),
            unknown_bodies: BTreeSet::new(),
            report,
        }
    }

    pub fn function(&mut self, id: FnId) -> Result<(), Internal> {
        let decl = self.hir.fn_decl(id)?;
        let mut earlier: Option<usize> = None;
        let mut seen_default = false;
        for param in &decl.params {
            if let Some(default) = param.default {
                self.expr(default, earlier, false)?;
                seen_default = true;
            } else if seen_default {
                self.report.error(
                    self.hir,
                    Code::ParameterOrder,
                    param.loc,
                    "a parameter with a default follows every parameter without one",
                );
            }
            if let Some(name) = &param.name {
                self.declared(name)?;
                let mut names = BTreeMap::new();
                names.insert(name.text.clone(), name.loc);
                earlier = Some(self.frame(names, earlier, FrameKind::Locals));
            }
        }
        let locals = self.locals(
            decl.params.iter().filter_map(|param| param.name.as_ref()),
            decl.lets.iter().filter_map(|local| local.name.as_ref()),
        )?;
        self.leave_unknown(locals, decl.skipped_params.clone())?;
        for local in &decl.lets {
            self.expr(local.value, Some(locals), false)?;
        }
        self.expr(decl.value, Some(locals), false)?;
        self.let_cycles(&decl.lets)?;
        Ok(())
    }

    pub fn pattern(&mut self, id: PatternId) -> Result<(), Internal> {
        let decl = self.hir.pattern(id)?;
        let mut defaults = decl.params.iter().filter(|param| param.is_default);
        defaults.next();
        for extra in defaults {
            self.report.error(
                self.hir,
                Code::DuplicateDefaultParameter,
                extra.loc,
                "a pattern has at most one parameter marked `default`",
            );
        }
        let mut earlier: Option<usize> = None;
        for param in &decl.params {
            if let Some(default) = param.default {
                self.expr(default, earlier, false)?;
            }
            if let Some(name) = &param.name {
                self.declared(name)?;
                let mut names = BTreeMap::new();
                names.insert(name.text.clone(), name.loc);
                earlier = Some(self.frame(names, earlier, FrameKind::Locals));
            }
        }
        let locals = self.locals(
            decl.params.iter().filter_map(|param| param.name.as_ref()),
            decl.lets.iter().filter_map(|local| local.name.as_ref()),
        )?;
        self.leave_unknown(locals, decl.skipped_params.clone())?;
        for local in &decl.lets {
            self.expr(local.value, Some(locals), false)?;
        }
        self.blocks(vec![(decl.body, Some(locals), Mode::Body)])?;
        self.let_cycles(&decl.lets)?;
        Ok(())
    }

    fn let_cycles(&mut self, lets: &[LetDecl]) -> Result<(), Internal> {
        let mut positions: BTreeMap<&str, usize> = BTreeMap::new();
        for (position, local) in lets.iter().enumerate() {
            if let Some(name) = &local.name {
                positions.entry(name.text.as_str()).or_insert(position);
            }
        }
        let mut reads: Vec<Vec<usize>> = Vec::with_capacity(lets.len());
        for local in lets {
            let mut found = Vec::new();
            let mut stack = vec![local.value];
            while let Some(id) = stack.pop() {
                let kind = &self.hir.expr(id)?.kind;
                if let ExprKind::Name(name) = kind
                    && let Some(position) = positions.get(name.as_str())
                {
                    found.push(*position);
                }
                stack.extend(kind.children());
            }
            found.sort_unstable();
            found.dedup();
            reads.push(found);
        }
        let missed = Internal::missed("local let");
        let mut state = vec![Visit::New; lets.len()];
        for start in 0..lets.len() {
            let visit = state.get_mut(start).ok_or(missed)?;
            if *visit != Visit::New {
                continue;
            }
            *visit = Visit::Open;
            let mut path: Vec<(usize, usize)> = vec![(start, 0)];
            while let Some((node, next)) = path.last().copied() {
                let Some(&target) = reads.get(node).ok_or(missed)?.get(next) else {
                    *state.get_mut(node).ok_or(missed)? = Visit::Done;
                    path.pop();
                    continue;
                };
                if let Some(top) = path.last_mut() {
                    top.1 += 1;
                }
                let visit = state.get_mut(target).ok_or(missed)?;
                match *visit {
                    Visit::New => {
                        *visit = Visit::Open;
                        path.push((target, 0));
                    }
                    Visit::Open => {
                        let from = path
                            .iter()
                            .position(|(position, _)| *position == target)
                            .ok_or(missed)?;
                        let mut cycle: Vec<&Ident> = Vec::new();
                        for (position, _) in path.iter().skip(from) {
                            if let Some(name) = lets.get(*position).ok_or(missed)?.name.as_ref() {
                                cycle.push(name);
                            }
                        }
                        self.cycle(&cycle);
                    }
                    Visit::Done => {}
                }
            }
        }
        Ok(())
    }

    fn cycle(&mut self, names: &[&Ident]) {
        let Some(first) = names.first() else {
            return;
        };
        let mut text: Vec<&str> = names.iter().map(|name| name.text.as_str()).collect();
        text.push(first.text.as_str());
        let secondary: Vec<Loc> = names.iter().skip(1).map(|name| name.loc).collect();
        self.report.error_with(
            self.hir,
            Code::DeclarationCycle,
            first.loc,
            &secondary,
            format!("these declarations reach themselves: {}", text.join(" → ")),
        );
    }

    pub fn top_placement(&mut self, id: PlacementId) -> Result<(), Internal> {
        let placement = self.hir.placement(id)?;
        self.placement_exprs(id, None)?;
        self.kind(id, None, None, false)?;
        if let Some(body) = placement.body {
            self.blocks(vec![(body, None, Mode::Body)])?;
        }
        Ok(())
    }

    pub fn type_ref(&mut self, ty: &TypeRef) {
        if let TypeBase::Enum(name) = &ty.base
            && self.vocabulary.enum_named(&name.text).is_none()
        {
            self.report.error(
                self.hir,
                Code::UnknownType,
                name.loc,
                format!("`{}` is not an enum of the vocabulary", name.text),
            );
        }
        for (length, loc) in &ty.lengths {
            if *length == Some(0) || length.is_none() {
                self.report.error(
                    self.hir,
                    Code::ZeroLength,
                    *loc,
                    "an array length in a type is at least 1",
                );
            }
        }
    }

    fn locals<'n>(
        &mut self,
        params: impl Iterator<Item = &'n Ident>,
        lets: impl Iterator<Item = &'n Ident>,
    ) -> Result<usize, Internal> {
        let mut names: BTreeMap<String, Loc> = BTreeMap::new();
        for param in params {
            if let Some(earlier) = names.get(&param.text) {
                let earlier = *earlier;
                self.report.error_with(
                    self.hir,
                    Code::DuplicateLocal,
                    param.loc,
                    &[earlier],
                    format!("`{}` names two parameters", param.text),
                );
                continue;
            }
            names.insert(param.text.clone(), param.loc);
        }
        let mut seen: BTreeMap<String, Loc> = BTreeMap::new();
        for local in lets {
            self.declared(local)?;
            if let Some(earlier) = seen.get(&local.text).or_else(|| names.get(&local.text)) {
                let earlier = *earlier;
                self.report.error_with(
                    self.hir,
                    Code::DuplicateLocal,
                    local.loc,
                    &[earlier],
                    format!("`{}` is declared twice in one body", local.text),
                );
                continue;
            }
            seen.insert(local.text.clone(), local.loc);
        }
        names.extend(seen);
        Ok(self.frame(names, None, FrameKind::Locals))
    }

    fn frame(
        &mut self,
        names: BTreeMap<String, Loc>,
        parent: Option<usize>,
        kind: FrameKind,
    ) -> usize {
        self.frames.push(Frame {
            names,
            parent,
            kind,
            unknown: UnknownNames::default(),
        });
        self.frames.len() - 1
    }

    fn declared(&mut self, name: &Ident) -> Result<(), Internal> {
        if let Some(id) = self.variants.owner(&name.text) {
            let owner = self
                .vocabulary
                .enum_decl(id)
                .ok_or(Internal::missed("enum"))?
                .name
                .as_str();
            self.report.error(
                self.hir,
                Code::VariantName,
                name.loc,
                format!(
                    "`{}` is a variant of the enum `{owner}` and is never a declared name",
                    name.text
                ),
            );
        }
        Ok(())
    }

    fn leave_unknown(&mut self, frame: usize, unknown: UnknownNames) -> Result<(), Internal> {
        self.frames
            .get_mut(frame)
            .ok_or(Internal::missed("scope frame"))?
            .unknown = unknown;
        Ok(())
    }

    fn may_declare(&self, frame: Option<usize>, name: &str) -> Result<bool, Internal> {
        let mut current = frame;
        while let Some(index) = current {
            let frame = self
                .frames
                .get(index)
                .ok_or(Internal::missed("scope frame"))?;
            if frame.unknown.holds(name) {
                return Ok(true);
            }
            current = frame.parent;
        }
        Ok(self.hir.skipped_top.holds(name))
    }

    fn visible(&self, frame: Option<usize>, name: &str) -> Result<bool, Internal> {
        let mut current = frame;
        while let Some(index) = current {
            let frame = self
                .frames
                .get(index)
                .ok_or(Internal::missed("scope frame"))?;
            if frame.names.contains_key(name) {
                return Ok(true);
            }
            current = frame.parent;
        }
        Ok(self.namespace.get(name).is_some())
    }

    fn enclosing_body_name(
        &self,
        frame: Option<usize>,
        name: &str,
    ) -> Result<Option<Loc>, Internal> {
        let mut current = frame;
        while let Some(index) = current {
            let frame = self
                .frames
                .get(index)
                .ok_or(Internal::missed("scope frame"))?;
            if frame.kind == FrameKind::Locals {
                return Ok(None);
            }
            if frame.kind == FrameKind::Body
                && let Some(loc) = frame.names.get(name)
            {
                return Ok(Some(*loc));
            }
            current = frame.parent;
        }
        Ok(None)
    }

    pub fn expr(
        &mut self,
        root: ExprId,
        frame: Option<usize>,
        wire_clause: bool,
    ) -> Result<(), Internal> {
        let mut stack = vec![(root, false)];
        while let Some((id, field_base)) = stack.pop() {
            let expr = self.hir.expr(id)?;
            match &expr.kind {
                ExprKind::Name(name) => {
                    if !self.visible(frame, name)?
                        && self.variants.owner(name).is_none()
                        && !self.may_declare(frame, name)?
                    {
                        self.report.error(
                            self.hir,
                            Code::UnknownName,
                            expr.loc,
                            format!("`{name}` names nothing in scope"),
                        );
                    }
                }
                ExprKind::To => {
                    if !wire_clause {
                        self.report.error(
                            self.hir,
                            Code::MisplacedTo,
                            expr.loc,
                            "`to` is written only in a wire placement's `/expr`, gate and attribute clause",
                        );
                    } else if !field_base {
                        self.report.error(
                            self.hir,
                            Code::MisplacedTo,
                            expr.loc,
                            "`to` takes exactly one name, as in `to.gain`",
                        );
                    }
                }
                ExprKind::Field(base, _) => {
                    let base_expr = &self.hir.expr(*base)?.kind;
                    if let ExprKind::Field(inner, _) = base_expr
                        && matches!(self.hir.expr(*inner)?.kind, ExprKind::To)
                        && wire_clause
                    {
                        self.report.error(
                            self.hir,
                            Code::MisplacedTo,
                            expr.loc,
                            "`to` takes exactly one name, as in `to.gain`",
                        );
                        continue;
                    }
                    stack.push((*base, true));
                }
                ExprKind::Match {
                    scrutinee,
                    arms,
                    otherwise,
                } => {
                    stack.push((*scrutinee, false));
                    for arm in arms {
                        if !matches!(self.hir.expr(arm.label)?.kind, ExprKind::Name(_)) {
                            stack.push((arm.label, false));
                        }
                        stack.push((arm.value, false));
                    }
                    stack.extend(otherwise.iter().map(|value| (*value, false)));
                }
                other => stack.extend(other.children().into_iter().map(|child| (child, false))),
            }
        }
        Ok(())
    }

    fn placement_exprs(&mut self, id: PlacementId, frame: Option<usize>) -> Result<(), Internal> {
        let placement = self.hir.placement(id)?;
        for expr in placement.default.iter().chain(placement.gate.iter()) {
            self.expr(*expr, frame, false)?;
        }
        for attr in &placement.attrs {
            if let AttrValue::Expr(value) = attr.value {
                self.expr(value, frame, false)?;
            }
        }
        Ok(())
    }

    fn kind(
        &mut self,
        id: PlacementId,
        previous: Option<PlacementId>,
        frame: Option<usize>,
        in_unknown: bool,
    ) -> Result<(), Internal> {
        let placement = self.hir.placement(id)?;
        let Some(kind) = &placement.kind else {
            return Ok(());
        };
        match self.namespace.get(&kind.text) {
            Some(Decl::Node(_) | Decl::Pattern(_)) => {}
            found => {
                if let Some(body) = placement.body {
                    self.unknown_bodies.insert(body);
                }
                if in_unknown && self.visible(frame, &kind.text)? {
                    return Ok(());
                }
                let message = match found {
                    None => format!("`{}` is not a node kind or a pattern", kind.text),
                    Some(_) => format!("`{}` is not a kind", kind.text),
                };
                let previous = match previous.filter(|_| placement.inline) {
                    Some(previous) => Some(self.hir.placement(previous)?),
                    None => None,
                };
                let suggestion = previous
                    .filter(|previous| previous.name.is_none())
                    .and_then(|previous| previous.kind.as_ref())
                    .map(|previous| {
                        format!(
                            "to name the placement, write `{} as {}`",
                            previous.text, kind.text
                        )
                    });
                match suggestion {
                    Some(hint) => self.report.hinted(
                        self.hir,
                        Code::UnknownKind,
                        kind.loc,
                        &[],
                        message,
                        hint,
                    ),
                    None => self
                        .report
                        .error(self.hir, Code::UnknownKind, kind.loc, message),
                }
            }
        }
        Ok(())
    }

    fn blocks(&mut self, mut pending: Vec<(BlockId, Option<usize>, Mode)>) -> Result<(), Internal> {
        while let Some((block, parent, mode)) = pending.pop() {
            let frame = match mode {
                Mode::Body => {
                    let names = self.body_names(block, parent)?;
                    let frame = self.frame(names, parent, FrameKind::Body);
                    let unknown = self.hir.body_unknown(block)?;
                    self.leave_unknown(frame, unknown)?;
                    Some(frame)
                }
                Mode::Arm => parent,
                Mode::Anonymous => {
                    self.anonymous_names(block)?;
                    parent
                }
            };
            let mut previous: Option<PlacementId> = None;
            let in_unknown = self.unknown_bodies.contains(&block);
            for item in &self.hir.block(block)?.items {
                match item {
                    BodyItem::Placement(id) => {
                        self.placement_exprs(*id, frame)?;
                        self.kind(*id, previous, frame, in_unknown)?;
                        previous = Some(*id);
                        if let Some(body) = self.hir.placement(*id)?.body {
                            pending.push((body, frame, Mode::Body));
                        }
                    }
                    BodyItem::Wire(id) => {
                        previous = None;
                        let wire = self.hir.wire(*id)?;
                        for expr in wire.default.iter().chain(wire.gate.iter()) {
                            self.expr(*expr, frame, true)?;
                        }
                        for attr in &wire.attrs {
                            if let AttrValue::Expr(value) = attr.value {
                                self.expr(value, frame, true)?;
                            }
                        }
                        self.expr(wire.destination, frame, false)?;
                    }
                    BodyItem::Bundle { members, each, .. } => {
                        previous = None;
                        for member in members {
                            self.placement_exprs(*member, frame)?;
                            self.kind(*member, None, frame, in_unknown)?;
                        }
                        if let Some(each) = each {
                            self.iterable(each.iterable, frame)?;
                            let var_frame = self.for_var(each.var.as_ref(), frame)?;
                            for member in &each.members {
                                if let Some(name) = &self.hir.placement(*member)?.name {
                                    self.named_in_for(name);
                                }
                                self.placement_exprs(*member, var_frame)?;
                                self.kind(*member, None, var_frame, in_unknown)?;
                            }
                        }
                    }
                    BodyItem::For {
                        var,
                        iterable,
                        body,
                        ..
                    } => {
                        previous = None;
                        self.iterable(*iterable, frame)?;
                        let var_frame = self.for_var(var.as_ref(), frame)?;
                        pending.push((*body, var_frame, Mode::Anonymous));
                    }
                    BodyItem::When {
                        arms, otherwise, ..
                    } => {
                        previous = None;
                        let arm_mode = if mode == Mode::Anonymous {
                            Mode::Anonymous
                        } else {
                            Mode::Arm
                        };
                        for arm in arms {
                            self.expr(arm.guard, frame, false)?;
                            pending.push((arm.body, frame, arm_mode));
                        }
                        if let Some(otherwise) = otherwise {
                            pending.push((*otherwise, frame, arm_mode));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn iterable(&mut self, iterable: Iterable, frame: Option<usize>) -> Result<(), Internal> {
        match iterable {
            Iterable::Range(start, end) => {
                self.expr(start, frame, false)?;
                self.expr(end, frame, false)?;
            }
            Iterable::Values(values) => self.expr(values, frame, false)?,
        }
        Ok(())
    }

    fn for_var(
        &mut self,
        var: Option<&Ident>,
        frame: Option<usize>,
    ) -> Result<Option<usize>, Internal> {
        let Some(var) = var else {
            return Ok(frame);
        };
        self.declared(var)?;
        if self.visible(frame, &var.text)? {
            self.report.error(
                self.hir,
                Code::ForVariableClash,
                var.loc,
                format!("the loop variable `{}` equals a visible name", var.text),
            );
        }
        let mut names = BTreeMap::new();
        names.insert(var.text.clone(), var.loc);
        Ok(Some(self.frame(names, frame, FrameKind::ForVar)))
    }

    fn named_in_for(&mut self, name: &Ident) {
        self.report.error(
            self.hir,
            Code::NamedInFor,
            name.loc,
            "placements in a `for` body are anonymous",
        );
    }

    fn anonymous_names(&mut self, block: BlockId) -> Result<(), Internal> {
        let mut blocks = vec![block];
        while let Some(block) = blocks.pop() {
            for item in &self.hir.block(block)?.items {
                match item {
                    BodyItem::Placement(id) => {
                        if let Some(name) = &self.hir.placement(*id)?.name {
                            self.named_in_for(name);
                        }
                    }
                    BodyItem::Wire(id) => {
                        if let Some(name) = &self.hir.wire(*id)?.name {
                            self.named_in_for(name);
                        }
                    }
                    BodyItem::Bundle { members, .. } => {
                        for member in members {
                            if let Some(name) = &self.hir.placement(*member)?.name {
                                self.named_in_for(name);
                            }
                        }
                    }
                    BodyItem::For { .. } | BodyItem::When { .. } => {}
                }
            }
        }
        Ok(())
    }

    fn body_names(
        &mut self,
        block: BlockId,
        parent: Option<usize>,
    ) -> Result<BTreeMap<String, Loc>, Internal> {
        let mut names: BTreeMap<String, Loc> = BTreeMap::new();
        let mut blocks = vec![block];
        let mut found: Vec<Ident> = Vec::new();
        while let Some(block) = blocks.pop() {
            for item in &self.hir.block(block)?.items {
                match item {
                    BodyItem::Placement(id) => found.extend(self.hir.placement(*id)?.name.clone()),
                    BodyItem::Wire(id) => found.extend(self.hir.wire(*id)?.name.clone()),
                    BodyItem::Bundle { members, .. } => {
                        for member in members {
                            found.extend(self.hir.placement(*member)?.name.clone());
                        }
                    }
                    BodyItem::For { .. } => {}
                    BodyItem::When {
                        arms, otherwise, ..
                    } => {
                        blocks.extend(arms.iter().map(|arm| arm.body));
                        blocks.extend(otherwise.iter().copied());
                    }
                }
            }
        }
        found.sort_by_key(|name| name.loc);
        for name in found {
            self.declared(&name)?;
            let clash = match names.get(&name.text) {
                Some(loc) => Some(*loc),
                None => self.enclosing_body_name(parent, &name.text)?,
            };
            if let Some(earlier) = clash {
                self.report.error_with(
                    self.hir,
                    Code::ChildNameClash,
                    name.loc,
                    &[earlier],
                    format!(
                        "`{}` names another child or wire of this body or of an enclosing one",
                        name.text
                    ),
                );
                continue;
            }
            names.insert(name.text, name.loc);
        }
        Ok(names)
    }
}
