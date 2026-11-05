use super::super::binder::Binder;
use super::super::binder_kind::BinderKind;
use super::super::cell_rec::CellRec;
use super::super::field_rec::FieldRec;
use super::super::fn_rec::FnRec;
use super::super::iteration::Iteration;
use super::super::let_rec::LetRec;
use super::super::name_kind::NameKind;
use super::super::op::Op;
use super::super::pattern_rec::PatternRec;
use super::super::top_rec::TopRec;
use super::super::type_rec::TypeRec;
use super::super::{ABSENT, NONE, digits_value, index};
use super::constructs::{GENERIC, LAMBDA, LOCAL_ANNOTATION};
use super::ir_builder::{IrBuilder, span};
use super::marks::Marks;
use super::ranges::{annotation_end, brackets, line_end, trimmed};
use super::value::{Local, Value};
use crate::diagnostics::Code;
use crate::semantics::analysis::LENGTH_LIMIT;
use crate::semantics::ir::CellWord;
use crate::syntax::ast::{self, AstNode};
use crate::syntax::{NodeView, SyntaxKind};

impl<'p> IrBuilder<'p, '_> {
    pub(super) fn binder(
        &mut self,
        name: u32,
        kind: BinderKind,
        annotation: u32,
        at: (u32, u32),
    ) -> u32 {
        self.ir.binders.push(Binder {
            name,
            lo: at.0,
            hi: at.1,
            kind,
            annotation,
            owner: NONE,
            default: NONE,
            is_default: false,
        });
        index(self.ir.binders.len() - 1)
    }

    pub(super) fn local_let(&mut self, local: Local, owner: u32, fallback: (u32, u32)) -> u32 {
        let binder = self.binder(
            local.name,
            BinderKind::LocalLet,
            local.ty,
            (local.lo, local.hi),
        );
        if let Some(entry) = self.ir.binders.get_mut(binder as usize) {
            entry.owner = owner;
        }
        let value = self.or_bad(local.value, fallback);
        self.ir.lets.push(LetRec {
            binder,
            value,
            owner,
            lo: local.lo,
            hi: local.hi,
            is_match: local.is_match,
            faulty_line: false,
            line_fault: false,
        });
        index(self.ir.lets.len() - 1)
    }

    pub(super) fn loop_names(
        &mut self,
        head: &ast::LoopHead<'_>,
        base: usize,
        kind: BinderKind,
    ) -> (u32, u32) {
        let start = index(self.ir.binders.len());
        for loop_name in head.names() {
            let at = span_of(loop_name.name.range());
            let name = self.name_of(Some(loop_name.name), NameKind::Binder);
            let ty = loop_name.ty.and_then(|ty| self.value_at(base, ty.syntax()));
            let ty = self.commit_type(ty);
            self.binder(name, kind, ty, at);
        }
        (start, index(self.ir.binders.len()))
    }

    pub(super) fn iterable_of(
        &mut self,
        head: &ast::LoopHead<'_>,
        base: usize,
    ) -> (bool, (u32, u32)) {
        match head.iterable() {
            Some(ast::Iterable::Range(range)) => match self.value_at(base, range.syntax()) {
                Some(Value::Range { start, end }) => (true, (start, end)),
                _ => (true, (NONE, NONE)),
            },
            Some(ast::Iterable::Expr(expr)) => {
                let value = self.value_at(base, expr.syntax());
                (false, (self.force(value).unwrap_or(NONE), NONE))
            }
            None => (false, (NONE, NONE)),
        }
    }

    pub(super) fn iteration(
        &mut self,
        head: &ast::LoopHead<'_>,
        base: usize,
        kind: BinderKind,
        fallback: (u32, u32),
    ) -> Iteration {
        let names = self.loop_names(head, base, kind);
        let (range, iterable) = self.iterable_of(head, base);
        let iterable = if range {
            let range_at = match head.iterable() {
                Some(ast::Iterable::Range(range)) => span(range.syntax()),
                _ => fallback,
            };
            (
                self.or_bad(iterable.0, range_at),
                self.or_bad(iterable.1, range_at),
            )
        } else {
            (self.or_bad(iterable.0, fallback), NONE)
        };
        Iteration {
            names,
            range,
            iterable,
        }
    }

    pub(super) fn faulty_line(&self, view: &NodeView<'_>) -> bool {
        let start = view.range().start;
        self.first_in(start + 1..line_end(view)).is_some()
    }

    fn line_fault(&self, view: &NodeView<'_>) -> bool {
        let text = self.parsed.text();
        let start = view.range().start as usize;
        let first = text
            .get(..start)
            .and_then(|before| before.rfind('\n'))
            .map_or(0, |at| at + 1);
        let end = line_end(view) as usize;
        let last = text
            .get(end..)
            .and_then(|after| after.find('\n'))
            .map_or(text.len(), |at| end + at);
        self.fault_reach(u32::try_from(last).unwrap_or(u32::MAX))
            > u32::try_from(first).unwrap_or(u32::MAX)
    }

    pub(super) fn let_decl(
        &mut self,
        view: NodeView<'p>,
        base: usize,
        marks: Marks,
    ) -> Option<Value> {
        let node = ast::LetDecl::cast(view)?;
        let whole = span(&view);
        let name = self.name_of(node.name(), NameKind::Declaration);
        if let Some(lambda) = node.lambda() {
            marks.truncate(self.ir);
            let from = self.drawn_mark();
            self.draw_unbuilt(span(lambda.syntax()), LAMBDA);
            return Some(Value::Top(TopRec::Placeholder {
                name,
                drawn: (from, self.drawn_mark()),
            }));
        }
        let (value, is_match) = match node.match_block() {
            Some(block) => (self.expr_of(base, Some(block), whole), true),
            None => (self.expr_of(base, node.value(), whole), false),
        };
        let ty = node.ty().and_then(|ty| self.value_at(base, ty.syntax()));
        let ty = self.commit_type(ty);
        let line_fault = self.line_fault(&view);
        let faulty_line = self.faulty_line(&view);
        let binder = self.binder(name, BinderKind::Let, ty, whole);
        self.ir.lets.push(LetRec {
            binder,
            value,
            owner: NONE,
            lo: whole.0,
            hi: whole.1,
            is_match,
            faulty_line,
            line_fault,
        });
        Some(Value::Top(TopRec::Let(index(self.ir.lets.len() - 1))))
    }

    pub(super) fn cell_decl(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::CellDecl::cast(view)?;
        let whole = span(&view);
        let rule = node.has_rule();
        let word = match node.word().map(|token| token.kind()) {
            Some(SyntaxKind::StreamKw) => CellWord::Stream,
            Some(SyntaxKind::ChannelKw) => CellWord::Channel,
            Some(_) | None => CellWord::State,
        };
        let from = self.drawn_mark();
        if let Some(clock) = node.clock()
            && rule
        {
            self.draw(
                Code::UnbuiltConstruct,
                span(clock.syntax()),
                "a clock (SPEC §6.6) is a construct this compiler does not build",
            );
        }
        let capacity = node.capacity().and_then(|token| {
            (!self.touch(token.range()))
                .then(|| digits_value(token.text()))
                .flatten()
        });
        let misplaced = node.has_capacity() && (rule || word == CellWord::State);
        if misplaced && let Some(range) = brackets(&node) {
            let summary = if rule {
                "`[N]` states the capacity of a host-fed `stream` or `channel`, and this cell is derived"
            } else {
                "`[N]` states the capacity of a host-fed `stream` or `channel`, and this cell is a `state`"
            };
            self.draw(Code::CellCapacity, range, summary);
        }
        let capacity = capacity.and_then(|value| self.sized(&node, misplaced, value));
        let drawn = (from, self.drawn_mark());
        let name = self.name_of(node.name(), NameKind::Declaration);
        let faulty_line = self.faulty_line(&view);
        let ty_view = node.ty();
        let ty_value = ty_view
            .as_ref()
            .and_then(|ty| self.value_at(base, ty.syntax()));
        let ty = self.commit_type(ty_value);
        if let (Some(ty_view), Some(Value::Ty { base: ty_base, .. })) = (&ty_view, ty_value) {
            let after = ty_view.syntax().range().end;
            if self
                .first_in(after..annotation_end(&node).max(after))
                .is_some()
                && !matches!(self.ir.node(ty_base), Ok((Op::Bad, ..)))
            {
                let at = self.drawn_mark();
                self.set_node(ty_base, Op::Bad, (at, at, NONE));
            }
        }
        let init_value = node.init().and_then(|init| {
            let value = self.value_at(base, init.syntax());
            self.force(value)
        });
        let input = !rule && node.init().is_some();
        let (init, clock, rule_root) = if input {
            (self.or_bad(init_value.unwrap_or(NONE), whole), (0, 0), NONE)
        } else {
            let init = match node.init() {
                Some(init) => self.or_bad(init_value.unwrap_or(NONE), span(init.syntax())),
                None => NONE,
            };
            let clock = match node
                .clock()
                .and_then(|clock| self.value_at(base, clock.syntax()))
            {
                Some(Value::Members(range)) => range,
                _ => (0, 0),
            };
            let value = match node.match_block() {
                Some(block) => self.expr_of(base, Some(block), whole),
                None if rule => self.expr_of(base, node.value(), whole),
                None => self.bad(whole),
            };
            (init, clock, value)
        };
        let binder = self.binder(name, BinderKind::Cell, ty, whole);
        self.ir.cells.push(CellRec {
            binder,
            word,
            input,
            init,
            clock,
            rule: rule_root,
            capacity: if input {
                capacity.filter(|_| word != CellWord::State)
            } else {
                capacity
            },
            lo: whole.0,
            hi: whole.1,
            drawn,
            faulty_line,
        });
        Some(Value::Top(TopRec::Cell(index(self.ir.cells.len() - 1))))
    }

    fn sized(&mut self, node: &ast::CellDecl<'_>, misplaced: bool, value: u64) -> Option<u32> {
        let size = u32::try_from(value)
            .ok()
            .filter(|size| (1..LENGTH_LIMIT).contains(size));
        if size.is_none()
            && !misplaced
            && let Some(range) = brackets(node)
        {
            let (code, summary) = if value == 0 {
                (
                    Code::ZeroLength,
                    "the capacity of a host-fed `stream` or `channel` is at least 1",
                )
            } else {
                (
                    Code::LengthLimit,
                    "the capacity of a host-fed `stream` or `channel` is below 2³¹",
                )
            };
            self.draw(code, range, summary);
        }
        size
    }

    pub(super) fn clock(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::Clock::cast(view)?;
        let whole = span(&view);
        let entries: Vec<u32> = node
            .entries()
            .map(|entry| self.expr_of(base, Some(entry), whole))
            .collect();
        let start = index(self.ir.kids.len());
        self.ir.kids.extend(entries);
        Some(Value::Members((start, index(self.ir.kids.len()))))
    }

    pub(super) fn param(&mut self, view: NodeView<'p>, base: usize) -> Value {
        let (name, default) = match view.kind() {
            SyntaxKind::Param => match ast::Param::cast(view) {
                Some(node) => (node.name(), node.default_value()),
                None => (None, None),
            },
            _ => match ast::PatternParam::cast(view) {
                Some(node) => (node.name(), node.default_value()),
                None => (None, None),
            },
        };
        let name = self.name_of(name, NameKind::Binder);
        let default = default.and_then(|value| self.value_at(base, value.syntax()));
        let default = self.force(default).unwrap_or(NONE);
        Value::Param { name, default }
    }

    fn param_binder(
        &mut self,
        value: Option<Value>,
        kind: BinderKind,
        at: (u32, u32),
        owner: u32,
        is_default: bool,
    ) {
        let (name, default) = match value {
            Some(Value::Param { name, default }) => (name, default),
            _ => (NONE, NONE),
        };
        self.ir.binders.push(Binder {
            name,
            lo: at.0,
            hi: at.1,
            kind,
            annotation: NONE,
            owner,
            default,
            is_default,
        });
    }

    fn regions_of(
        &mut self,
        view: &NodeView<'_>,
        list: SyntaxKind,
    ) -> ((u32, u32), Vec<NodeView<'p>>) {
        let start = index(self.ir.regions.len());
        let mut found = Vec::new();
        let inside: Vec<NodeView<'_>> = view
            .children()
            .filter(|child| child.kind() == list)
            .flat_map(|list| list.children())
            .collect();
        for region in view.children().chain(inside) {
            if region.kind() == SyntaxKind::ErrorNode {
                let range = region.range();
                self.ir.regions.push((range.start, range.end));
                found.push(NodeView::new(self.parsed, region.index()));
            }
        }
        ((start, index(self.ir.regions.len())), found)
    }

    pub(super) fn fn_decl(
        &mut self,
        view: NodeView<'p>,
        base: usize,
        marks: Marks,
    ) -> Option<Value> {
        let node = ast::FnDecl::cast(view)?;
        let whole = span(&view);
        let name = self.name_of(node.name(), NameKind::Declaration);
        let mut parts: Vec<((u32, u32), &str)> = Vec::new();
        if let Some(params) = node.generic_params() {
            parts.push((span(params.syntax()), GENERIC));
        }
        for param in node.params() {
            if let Some(marker) = param.marker() {
                parts.push((span_of(marker.range()), "a `cell` parameter (SPEC §6.3)"));
            }
            if let Some(ty) = param.ty() {
                parts.push((
                    span(ty.syntax()),
                    "an annotation on a parameter (SPEC §6.3)",
                ));
            }
        }
        if let Some(result) = node.result() {
            parts.push((span(result.syntax()), "a result annotation (SPEC §6.3)"));
        }
        for local in node.body().into_iter().flat_map(|body| body.lets()) {
            if let Some(lambda) = local.lambda() {
                parts.push((span(lambda.syntax()), LAMBDA));
            }
            if let Some(ty) = local.ty() {
                parts.push((span(ty.syntax()), LOCAL_ANNOTATION));
            }
        }
        if !parts.is_empty() {
            return Some(self.unbuilt_declaration(name, whole, parts, marks));
        }
        let owner = index(self.ir.fns.len());
        let list = node
            .syntax()
            .children()
            .find(|child| child.kind() == SyntaxKind::ParamList)
            .and_then(|list| self.value_at(base, &list));
        let params_start = index(self.ir.binders.len());
        for param in node.params() {
            let value = self.staged_at(list, param.syntax());
            self.param_binder(
                value,
                BinderKind::FnParam,
                span(param.syntax()),
                owner,
                false,
            );
        }
        let params = (params_start, index(self.ir.binders.len()));
        let lets_start = index(self.ir.lets.len());
        let value = match node.body() {
            Some(body) => {
                let staged = self.value_at(base, body.syntax());
                for local in body.lets() {
                    if let Some(Value::Local(local)) = self.staged_at(staged, local.syntax()) {
                        self.local_let(local, owner, (local.lo, local.hi));
                    }
                }
                let value = match body.match_block() {
                    Some(block) => self.staged_at(staged, block.syntax()),
                    None => body
                        .value()
                        .and_then(|value| self.staged_at(staged, value.syntax())),
                };
                match self.force(value) {
                    Some(id) => id,
                    None => self.bad(whole),
                }
            }
            None => self.bad(whole),
        };
        let lets = (lets_start, index(self.ir.lets.len()));
        let (regions, _) = self.regions_of(&view, SyntaxKind::ParamList);
        let faulty_line = self.faulty_line(&view);
        self.ir.fns.push(FnRec {
            name,
            params,
            lets,
            value,
            regions,
            lo: whole.0,
            hi: whole.1,
            faulty_line,
        });
        Some(Value::Top(TopRec::Fn(owner)))
    }

    fn unbuilt_declaration(
        &mut self,
        name: u32,
        whole: (u32, u32),
        parts: Vec<((u32, u32), &str)>,
        marks: Marks,
    ) -> Value {
        marks.truncate(self.ir);
        let from = self.drawn_mark();
        if self.first_in(whole.0..whole.1).is_none() {
            for (range, construct) in parts {
                self.draw_unbuilt(range, construct);
            }
        }
        Value::Top(TopRec::Placeholder {
            name,
            drawn: (from, self.drawn_mark()),
        })
    }

    pub(super) fn pattern_decl(
        &mut self,
        view: NodeView<'p>,
        base: usize,
        marks: Marks,
    ) -> Option<Value> {
        let node = ast::PatternDecl::cast(view)?;
        let whole = span(&view);
        let name = self.name_of(node.name(), NameKind::Declaration);
        let mut parts: Vec<((u32, u32), &str)> = Vec::new();
        if let Some(params) = node.generic_params() {
            parts.push((span(params.syntax()), GENERIC));
        }
        for param in node.params() {
            if param.is_children() {
                parts.push((span(param.syntax()), "a `children` parameter (SPEC §6.4)"));
                continue;
            }
            let markers: Vec<std::ops::Range<u32>> =
                param.markers().map(|marker| marker.range()).collect();
            if let (Some(first), Some(last)) = (markers.first(), markers.last()) {
                parts.push(((first.start, last.end), "a parameter marker (SPEC §6.4)"));
            }
            if let Some(ty) = param.ty() {
                parts.push((
                    span(ty.syntax()),
                    "an annotation on a pattern parameter (SPEC §6.4)",
                ));
            }
        }
        if let Some(placement_type) = node.placement_type() {
            parts.push((
                span(placement_type.syntax()),
                "a pattern's placement type (SPEC §6.4)",
            ));
        }
        if let Some(body) = node.body() {
            for cell in body.cells() {
                parts.push((
                    trimmed(cell.syntax()),
                    "a cell in a pattern body (SPEC §6.4)",
                ));
            }
            for local in body.lets() {
                if let Some(lambda) = local.lambda() {
                    parts.push((span(lambda.syntax()), LAMBDA));
                }
                if let Some(ty) = local.ty() {
                    parts.push((span(ty.syntax()), LOCAL_ANNOTATION));
                }
            }
        }
        if !parts.is_empty() {
            return Some(self.unbuilt_declaration(name, whole, parts, marks));
        }
        let owner = index(self.ir.patterns.len());
        let list = node
            .syntax()
            .children()
            .find(|child| child.kind() == SyntaxKind::PatternParamList)
            .and_then(|list| self.value_at(base, &list));
        let params_start = index(self.ir.binders.len());
        for param in node.params() {
            let value = self.staged_at(list, param.syntax());
            self.param_binder(
                value,
                BinderKind::PatternParam,
                span(param.syntax()),
                owner,
                param.is_default(),
            );
        }
        let params = (params_start, index(self.ir.binders.len()));
        let body = match node
            .body()
            .and_then(|body| self.value_at(base, body.syntax()))
        {
            Some(Value::Block(block)) => block,
            _ => ABSENT,
        };
        if let Some(lets) = self.ir.blocks.get(body as usize).map(|block| block.lets) {
            for at in lets.0..lets.1 {
                let binder = self.ir.lets.get_mut(at as usize).map(|local| {
                    local.owner = owner;
                    local.binder
                });
                if let Some(binder) =
                    binder.and_then(|binder| self.ir.binders.get_mut(binder as usize))
                {
                    binder.owner = owner;
                }
            }
        }
        let (regions, found) = self.regions_of(&view, SyntaxKind::PatternParamList);
        let skipped_default = found
            .iter()
            .any(|region| self.holds_token(region, SyntaxKind::DefaultKw));
        let faulty_line = self.faulty_line(&view);
        self.ir.patterns.push(PatternRec {
            name,
            params,
            body,
            regions,
            skipped_default,
            lo: whole.0,
            hi: whole.1,
            faulty_line,
        });
        Some(Value::Top(TopRec::Pattern(owner)))
    }

    pub(super) fn holds_token(&self, view: &NodeView<'_>, kind: SyntaxKind) -> bool {
        let (first, end) = self.parsed.tree().tokens(view.index());
        (first..end).any(|token| self.parsed.tokens().kind(token as usize) == Some(kind))
    }

    pub(super) fn type_decl(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::TypeDecl::cast(view)?;
        let whole = span(&view);
        let name = self.name_of(node.name(), NameKind::Declaration);
        let alias = match node
            .alias()
            .and_then(|alias| self.value_at(base, alias.syntax()))
        {
            Some(Value::Ty { root, .. }) => root,
            _ => NONE,
        };
        let fields_start = index(self.ir.fields.len());
        for field in node.fields() {
            if let Some(Value::Field { name, ty, lo, hi }) = self.value_at(base, field.syntax()) {
                self.ir.fields.push(FieldRec { name, ty, lo, hi });
            }
        }
        let fields = (fields_start, index(self.ir.fields.len()));
        self.ir.types.push(TypeRec {
            name,
            alias,
            fields,
            lo: whole.0,
            hi: whole.1,
        });
        let from = self.drawn_mark();
        if (node.alias().is_some() || node.fields().next().is_some())
            && self.first_in(whole.0..whole.1).is_none()
        {
            self.draw_unbuilt(trimmed(&view), "a `type` declaration (SPEC §3.8)");
        }
        Some(Value::Top(TopRec::Placeholder {
            name,
            drawn: (from, self.drawn_mark()),
        }))
    }

    pub(super) fn field_decl(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::FieldDecl::cast(view)?;
        let name = self.name_of(node.name(), NameKind::Declaration);
        let ty = match node.ty().and_then(|ty| self.value_at(base, ty.syntax())) {
            Some(Value::Ty { root, .. }) => root,
            _ => NONE,
        };
        let whole = span(&view);
        Some(Value::Field {
            name,
            ty,
            lo: whole.0,
            hi: whole.1,
        })
    }

    pub(super) fn drawn_item(
        &mut self,
        view: NodeView<'p>,
        marks: Marks,
        construct: &str,
    ) -> Value {
        marks.truncate(self.ir);
        let from = self.drawn_mark();
        if self.first_in(span(&view).0..span(&view).1).is_none() {
            self.draw_unbuilt(trimmed(&view), construct);
        }
        Value::Top(TopRec::Drawn((from, self.drawn_mark())))
    }
}

pub(super) fn span_of(range: std::ops::Range<u32>) -> (u32, u32) {
    (range.start, range.end)
}
