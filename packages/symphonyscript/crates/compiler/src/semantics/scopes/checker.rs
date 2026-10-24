use std::collections::BTreeMap;

use symphonyscript_core::Vocabulary;

use super::frame::{Frame, FrameKind};
use crate::diagnostics::Code;
use crate::semantics::collection::{Decl, Namespace, Variants};
use crate::semantics::ir::{
    AttrValue, BlockId, BodyItem, ExprId, ExprKind, FnId, Hir, Ident, Iterable, LetDecl, Loc,
    PatternId, PlacementId, TypeBase, TypeRef,
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
            report,
        }
    }

    pub fn function(&mut self, id: FnId) {
        let decl = self.hir.fn_decl(id);
        let mut earlier: Option<usize> = None;
        let mut seen_default = false;
        for param in &decl.params {
            if let Some(default) = param.default {
                self.expr(default, earlier, false);
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
                self.declared(name);
                let mut names = BTreeMap::new();
                names.insert(name.text.clone(), name.loc);
                earlier = Some(self.frame(names, earlier, FrameKind::Locals));
            }
        }
        let locals = self.locals(
            decl.params.iter().filter_map(|param| param.name.as_ref()),
            decl.lets.iter().filter_map(|local| local.name.as_ref()),
        );
        for local in &decl.lets {
            self.expr(local.value, Some(locals), false);
        }
        self.expr(decl.value, Some(locals), false);
        self.let_cycles(&decl.lets);
    }

    pub fn pattern(&mut self, id: PatternId) {
        let decl = self.hir.pattern(id);
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
                self.expr(default, earlier, false);
            }
            if let Some(name) = &param.name {
                self.declared(name);
                let mut names = BTreeMap::new();
                names.insert(name.text.clone(), name.loc);
                earlier = Some(self.frame(names, earlier, FrameKind::Locals));
            }
        }
        let locals = self.locals(
            decl.params.iter().filter_map(|param| param.name.as_ref()),
            decl.lets.iter().filter_map(|local| local.name.as_ref()),
        );
        for local in &decl.lets {
            self.expr(local.value, Some(locals), false);
        }
        self.blocks(vec![(decl.body, Some(locals), Mode::Body)]);
        self.let_cycles(&decl.lets);
    }

    fn let_cycles(&mut self, lets: &[LetDecl]) {
        let mut positions: BTreeMap<&str, usize> = BTreeMap::new();
        for (position, local) in lets.iter().enumerate() {
            if let Some(name) = &local.name {
                positions.entry(name.text.as_str()).or_insert(position);
            }
        }
        let reads: Vec<Vec<usize>> = lets
            .iter()
            .map(|local| {
                let mut found = Vec::new();
                let mut stack = vec![local.value];
                while let Some(id) = stack.pop() {
                    let kind = &self.hir.expr(id).kind;
                    if let ExprKind::Name(name) = kind
                        && let Some(position) = positions.get(name.as_str())
                    {
                        found.push(*position);
                    }
                    stack.extend(kind.children());
                }
                found.sort_unstable();
                found.dedup();
                found
            })
            .collect();
        let mut state = vec![Visit::New; lets.len()];
        for start in 0..lets.len() {
            if state.get(start) != Some(&Visit::New) {
                continue;
            }
            let mut path: Vec<(usize, usize)> = vec![(start, 0)];
            if let Some(slot) = state.get_mut(start) {
                *slot = Visit::Open;
            }
            while let Some((node, next)) = path.last().copied() {
                let Some(&target) = reads.get(node).and_then(|targets| targets.get(next)) else {
                    if let Some(slot) = state.get_mut(node) {
                        *slot = Visit::Done;
                    }
                    path.pop();
                    continue;
                };
                if let Some(top) = path.last_mut() {
                    top.1 += 1;
                }
                match state.get(target) {
                    Some(Visit::New) => {
                        if let Some(slot) = state.get_mut(target) {
                            *slot = Visit::Open;
                        }
                        path.push((target, 0));
                    }
                    Some(Visit::Open) => {
                        let from = path
                            .iter()
                            .position(|(position, _)| *position == target)
                            .unwrap_or(0);
                        let cycle: Vec<&Ident> = path
                            .get(from..)
                            .unwrap_or_default()
                            .iter()
                            .filter_map(|(position, _)| lets.get(*position)?.name.as_ref())
                            .collect();
                        self.cycle(&cycle);
                    }
                    _ => {}
                }
            }
        }
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

    pub fn top_placement(&mut self, id: PlacementId) {
        let placement = self.hir.placement(id);
        self.placement_exprs(id, None);
        self.kind(id, None);
        if let Some(body) = placement.body {
            self.blocks(vec![(body, None, Mode::Body)]);
        }
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
    ) -> usize {
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
            self.declared(local);
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
        self.frame(names, None, FrameKind::Locals)
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
        });
        self.frames.len() - 1
    }

    fn declared(&mut self, name: &Ident) {
        if let Some(owner) = self
            .variants
            .owner(&name.text)
            .and_then(|id| self.vocabulary.enum_decl(id))
            .map(|decl| decl.name.as_str())
        {
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
    }

    fn visible(&self, frame: Option<usize>, name: &str) -> bool {
        let mut current = frame;
        while let Some(index) = current {
            let Some(frame) = self.frames.get(index) else {
                break;
            };
            if frame.names.contains_key(name) {
                return true;
            }
            current = frame.parent;
        }
        self.namespace.get(name).is_some()
    }

    fn enclosing_body_name(&self, frame: Option<usize>, name: &str) -> Option<Loc> {
        let mut current = frame;
        while let Some(index) = current {
            let frame = self.frames.get(index)?;
            if frame.kind == FrameKind::Locals {
                return None;
            }
            if frame.kind == FrameKind::Body
                && let Some(loc) = frame.names.get(name)
            {
                return Some(*loc);
            }
            current = frame.parent;
        }
        None
    }

    pub fn expr(&mut self, root: ExprId, frame: Option<usize>, wire_clause: bool) {
        let mut stack = vec![(root, false)];
        while let Some((id, field_base)) = stack.pop() {
            let expr = self.hir.expr(id);
            match &expr.kind {
                ExprKind::Name(name) => {
                    if !self.visible(frame, name) && self.variants.owner(name).is_none() {
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
                    let base_expr = &self.hir.expr(*base).kind;
                    if let ExprKind::Field(inner, _) = base_expr
                        && matches!(self.hir.expr(*inner).kind, ExprKind::To)
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
                        if !matches!(self.hir.expr(arm.label).kind, ExprKind::Name(_)) {
                            stack.push((arm.label, false));
                        }
                        stack.push((arm.value, false));
                    }
                    stack.extend(otherwise.iter().map(|value| (*value, false)));
                }
                other => stack.extend(other.children().into_iter().map(|child| (child, false))),
            }
        }
    }

    fn placement_exprs(&mut self, id: PlacementId, frame: Option<usize>) {
        let placement = self.hir.placement(id);
        for expr in placement.default.iter().chain(placement.gate.iter()) {
            self.expr(*expr, frame, false);
        }
        for attr in &placement.attrs {
            if let AttrValue::Expr(value) = attr.value {
                self.expr(value, frame, false);
            }
        }
    }

    fn kind(&mut self, id: PlacementId, previous: Option<PlacementId>) {
        let placement = self.hir.placement(id);
        let Some(kind) = &placement.kind else {
            return;
        };
        match self.namespace.get(&kind.text) {
            Some(Decl::Node(_) | Decl::Pattern(_)) => {}
            found => {
                let message = match found {
                    None => format!("`{}` is not a node kind or a pattern", kind.text),
                    Some(_) => format!("`{}` is not a kind", kind.text),
                };
                let suggestion = previous
                    .filter(|_| placement.inline)
                    .map(|previous| self.hir.placement(previous))
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
    }

    fn blocks(&mut self, mut pending: Vec<(BlockId, Option<usize>, Mode)>) {
        while let Some((block, parent, mode)) = pending.pop() {
            let frame = match mode {
                Mode::Body => {
                    let names = self.body_names(block, parent);
                    Some(self.frame(names, parent, FrameKind::Body))
                }
                Mode::Arm => parent,
                Mode::Anonymous => {
                    self.anonymous_names(block);
                    parent
                }
            };
            let mut previous: Option<PlacementId> = None;
            for item in &self.hir.block(block).items {
                match item {
                    BodyItem::Placement(id) => {
                        self.placement_exprs(*id, frame);
                        self.kind(*id, previous);
                        previous = Some(*id);
                        if let Some(body) = self.hir.placement(*id).body {
                            pending.push((body, frame, Mode::Body));
                        }
                    }
                    BodyItem::Wire(id) => {
                        previous = None;
                        let wire = self.hir.wire(*id);
                        for expr in wire.default.iter().chain(wire.gate.iter()) {
                            self.expr(*expr, frame, true);
                        }
                        for attr in &wire.attrs {
                            if let AttrValue::Expr(value) = attr.value {
                                self.expr(value, frame, true);
                            }
                        }
                        self.expr(wire.destination, frame, false);
                    }
                    BodyItem::Bundle { members, each, .. } => {
                        previous = None;
                        for member in members {
                            self.placement_exprs(*member, frame);
                            self.kind(*member, None);
                        }
                        if let Some(each) = each {
                            self.iterable(each.iterable, frame);
                            let var_frame = self.for_var(each.var.as_ref(), frame);
                            for member in &each.members {
                                if let Some(name) = &self.hir.placement(*member).name {
                                    self.named_in_for(name);
                                }
                                self.placement_exprs(*member, var_frame);
                                self.kind(*member, None);
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
                        self.iterable(*iterable, frame);
                        let var_frame = self.for_var(var.as_ref(), frame);
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
                            self.expr(arm.guard, frame, false);
                            pending.push((arm.body, frame, arm_mode));
                        }
                        if let Some(otherwise) = otherwise {
                            pending.push((*otherwise, frame, arm_mode));
                        }
                    }
                }
            }
        }
    }

    fn iterable(&mut self, iterable: Iterable, frame: Option<usize>) {
        match iterable {
            Iterable::Range(start, end) => {
                self.expr(start, frame, false);
                self.expr(end, frame, false);
            }
            Iterable::Values(values) => self.expr(values, frame, false),
        }
    }

    fn for_var(&mut self, var: Option<&Ident>, frame: Option<usize>) -> Option<usize> {
        let Some(var) = var else {
            return frame;
        };
        self.declared(var);
        if self.visible(frame, &var.text) {
            self.report.error(
                self.hir,
                Code::ForVariableClash,
                var.loc,
                format!("the loop variable `{}` equals a visible name", var.text),
            );
        }
        let mut names = BTreeMap::new();
        names.insert(var.text.clone(), var.loc);
        Some(self.frame(names, frame, FrameKind::ForVar))
    }

    fn named_in_for(&mut self, name: &Ident) {
        self.report.error(
            self.hir,
            Code::NamedInFor,
            name.loc,
            "placements in a `for` body are anonymous",
        );
    }

    fn anonymous_names(&mut self, block: BlockId) {
        let mut blocks = vec![block];
        while let Some(block) = blocks.pop() {
            for item in &self.hir.block(block).items {
                match item {
                    BodyItem::Placement(id) => {
                        if let Some(name) = &self.hir.placement(*id).name {
                            self.named_in_for(name);
                        }
                    }
                    BodyItem::Wire(id) => {
                        if let Some(name) = &self.hir.wire(*id).name {
                            self.named_in_for(name);
                        }
                    }
                    BodyItem::Bundle { members, .. } => {
                        for member in members {
                            if let Some(name) = &self.hir.placement(*member).name {
                                self.named_in_for(name);
                            }
                        }
                    }
                    BodyItem::For { .. } | BodyItem::When { .. } => {}
                }
            }
        }
    }

    fn body_names(&mut self, block: BlockId, parent: Option<usize>) -> BTreeMap<String, Loc> {
        let mut names: BTreeMap<String, Loc> = BTreeMap::new();
        let mut blocks = vec![block];
        let mut found: Vec<Ident> = Vec::new();
        while let Some(block) = blocks.pop() {
            for item in &self.hir.block(block).items {
                match item {
                    BodyItem::Placement(id) => found.extend(self.hir.placement(*id).name.clone()),
                    BodyItem::Wire(id) => found.extend(self.hir.wire(*id).name.clone()),
                    BodyItem::Bundle { members, .. } => {
                        for member in members {
                            found.extend(self.hir.placement(*member).name.clone());
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
            self.declared(&name);
            let clash = names
                .get(&name.text)
                .copied()
                .or_else(|| self.enclosing_body_name(parent, &name.text));
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
        names
    }
}
