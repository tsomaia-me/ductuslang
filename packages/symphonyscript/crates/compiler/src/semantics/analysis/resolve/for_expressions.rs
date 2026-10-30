use std::collections::BTreeMap;
use std::rc::Rc;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::for_iterable::ForIterable;
use crate::semantics::analysis::for_node::ForNode;
use crate::semantics::analysis::ids::{NodeId, ScopeId, SlotId};
use crate::semantics::analysis::index::index as id_index;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::kind_of::kind_of;
use crate::semantics::analysis::layout::Layout;
use crate::semantics::analysis::local_name::LocalName;
use crate::semantics::analysis::locals::Locals;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::loop_template::LoopTemplate;
use crate::semantics::analysis::loop_types::LoopTypes;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::signature::branch_width;
use crate::semantics::analysis::slot::Slot;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::unify_kinds::unify_kinds;
use crate::semantics::analysis::val::Val;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::{ExprId, ForExpr, Iterable, Loc, LoopVar, Param, TypeRef};

const ROUNDS: u8 = 4;

struct Head {
    iterable: ForIterable,
    element: (Sig, Option<Layout>),
    start: NodeId,
    start_sig: Sig,
    start_layout: Option<Layout>,
}

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn for_expression(
        &mut self,
        expr: ExprId,
        for_expr: &ForExpr,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let Some(head) = some!(self.loop_head(for_expr, scope, need)?) else {
            return Ok(Some(Val::Error));
        };
        let element_name = match for_expr.names.as_slice() {
            [one] => Some(one),
            [_, element, ..] => Some(element),
            [] => None,
        };
        if let Some(var) = element_name
            && let Some(name) = &var.name
            && let Some((fixed, _)) = var.ty.as_ref().and_then(|ty| self.loop_annotation(ty))
            && !holds_annotated(head.element.0.kind, fixed.kind)
        {
            self.error(
                Code::TypeMismatch,
                name.loc,
                scope,
                "the elements do not have the loop name's annotated type; a conversion states the rounding",
            )?;
            return Ok(Some(Val::Error));
        }
        let annotated = match &for_expr.accumulator_ty {
            Some(ty) => match self.loop_annotation(ty) {
                Some(fixed) => Some(fixed),
                None => return Ok(Some(Val::Error)),
            },
            None => None,
        };
        loop {
            let state = self.memo.loop_templates.get(&(expr, scope)).cloned();
            let (template, types, round) = match state {
                Some(LoopTemplate::Final(node)) => {
                    return Ok(Some(node.map_or(Val::Error, Val::Scalar)));
                }
                Some(LoopTemplate::Trying {
                    scope: template,
                    types,
                    round,
                }) => (template, types, round),
                None => {
                    let types = match (&annotated, self.memo.loop_guesses.get(&expr)) {
                        (Some(fixed), _) => LoopTypes {
                            accumulator: fixed.clone(),
                        },
                        (None, Some(earlier)) => earlier.clone(),
                        (None, None) => LoopTypes {
                            accumulator: (
                                head.start_sig,
                                head.start_layout
                                    .as_ref()
                                    .map(|layout| Layout::variable(layout.inner().to_vec())),
                            ),
                        },
                    };
                    let template = self.loop_body(expr, for_expr, &head, &types, scope)?;
                    self.memo.loop_templates.insert(
                        (expr, scope),
                        LoopTemplate::Trying {
                            scope: template,
                            types,
                            round: 0,
                        },
                    );
                    continue;
                }
            };
            let body_val = some!(self.val(Query::Expr(for_expr.body, template), need));
            let body_loc = self.hir.expr(for_expr.body)?.loc;
            let Some(body) = self.boxed(&body_val, body_loc, template)? else {
                self.memo
                    .loop_templates
                    .insert((expr, scope), LoopTemplate::Final(None));
                continue;
            };
            let body_sig = some!(self.sig(body, need));
            let shaped = some!(self.layout_of(body, need)?).shaped;
            let body_layout = match shaped {
                Ok(layout) => layout,
                Err(_) if round == 0 && annotated.is_none() => {
                    let exact = LoopTypes {
                        accumulator: (head.start_sig, head.start_layout.clone()),
                    };
                    let template = self.loop_body(expr, for_expr, &head, &exact, scope)?;
                    self.memo.loop_templates.insert(
                        (expr, scope),
                        LoopTemplate::Trying {
                            scope: template,
                            types: exact,
                            round: 1,
                        },
                    );
                    continue;
                }
                Err(fault) => {
                    if let Some(fault) = fault {
                        self.error_at_node(fault.code, fault.node, fault.message)?;
                    }
                    self.memo
                        .loop_templates
                        .insert((expr, scope), LoopTemplate::Final(None));
                    continue;
                }
            };
            let next = match &annotated {
                Some(fixed) => LoopTypes {
                    accumulator: fixed.clone(),
                },
                None => {
                    let Some(layout) = merged_layout(head.start_layout.as_ref(), body_layout)
                    else {
                        self.error_helped(
                            Code::TypeMismatch,
                            loc,
                            scope,
                            LAYOUTS.to_string(),
                            crate::diagnostics::for_start_and_body_help(),
                        )?;
                        self.memo
                            .loop_templates
                            .insert((expr, scope), LoopTemplate::Final(None));
                        continue;
                    };
                    LoopTypes {
                        accumulator: (unified(head.start_sig, body_sig), layout),
                    }
                }
            };
            let state = if settled(&next, &types) || round + 1 >= ROUNDS {
                self.memo.loop_guesses.insert(expr, types);
                LoopTemplate::Final(Some(self.loop_node(&head, body, template, (loc, scope))?))
            } else {
                let template = self.loop_body(expr, for_expr, &head, &next, scope)?;
                LoopTemplate::Trying {
                    scope: template,
                    types: next,
                    round: round + 1,
                }
            };
            self.memo.loop_templates.insert((expr, scope), state);
        }
    }

    fn loop_head(
        &mut self,
        for_expr: &ForExpr,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<Head>>, Internal> {
        let start_expr = for_expr.start;
        let start_val = self.val(Query::Expr(start_expr, scope), need);
        let (iterable, element) = match for_expr.iterable {
            Iterable::Range(low, high) => {
                let low_val = self.val(Query::Expr(low, scope), need);
                let high_val = self.val(Query::Expr(high, scope), need);
                let (Some(low_val), Some(high_val)) = (low_val, high_val) else {
                    return Ok(None);
                };
                if let [_, second, ..] = for_expr.names.as_slice() {
                    let range = self.hir.expr(low)?.loc.to(self.hir.expr(high)?.loc);
                    let at = second.name.as_ref().map_or(range, |name| name.loc);
                    self.error_helped(
                        Code::IterableType,
                        at,
                        scope,
                        RANGE_NAMES.to_string(),
                        "write one name over a range, as in `for i in 0..4`".to_string(),
                    )?;
                    return Ok(Some(None));
                }
                let mut bounds = Vec::with_capacity(2);
                for (bound, val) in [(low, low_val), (high, high_val)] {
                    let bound_loc = self.hir.expr(bound)?.loc;
                    match val {
                        Val::Scalar(node) if !self.arrays.contains(&node) => bounds.push(node),
                        Val::Error => return Ok(Some(None)),
                        Val::Scalar(_) | Val::Array(_) => {
                            self.error(
                                Code::RangeBound,
                                bound_loc,
                                scope,
                                "a range bound is an integer, not an array",
                            )?;
                            return Ok(Some(None));
                        }
                    }
                }
                let (Some(low), Some(high)) = (bounds.first().copied(), bounds.get(1).copied())
                else {
                    return Err(Internal::missed("range bound"));
                };
                let low_sig = some!(self.sig(low, need));
                let high_sig = some!(self.sig(high, need));
                for (node, sig) in [(low, low_sig), (high, high_sig)] {
                    if sig.is_error() {
                        return Ok(Some(None));
                    }
                    if !matches!(sig.kind, Kind::Int | Kind::Any) {
                        self.error_at_node(Code::RangeBound, node, "a range bound is an integer")?;
                        return Ok(Some(None));
                    }
                    if sig.family.is_some() {
                        self.error_at_node(
                            Code::UnitOperation,
                            node,
                            "a unit value is not a range bound",
                        )?;
                        return Ok(Some(None));
                    }
                }
                let element = Sig::of(Kind::Int, Width::Flexible, false, None);
                (ForIterable::Range(low, high), (element, None))
            }
            Iterable::Values(values) => {
                let Some(values_val) = self.val(Query::Expr(values, scope), need) else {
                    return Ok(None);
                };
                let values_loc = self.hir.expr(values)?.loc;
                let Some(node) = self.boxed(&values_val, values_loc, scope)? else {
                    return Ok(Some(None));
                };
                let sig = some!(self.sig(node, need));
                if sig.is_error() {
                    return Ok(Some(None));
                }
                let Some(layout) = self.loop_layout(node, need)? else {
                    return Ok(None);
                };
                let Some(layout) = layout else {
                    self.error(Code::IterableType, values_loc, scope, ITERABLE)?;
                    return Ok(Some(None));
                };
                let inner = layout.inner().to_vec();
                let element_layout = (!inner.is_empty()).then(|| Layout::fixed(inner));
                (ForIterable::Values(node), (sig, element_layout))
            }
        };
        let Some(start_val) = start_val else {
            return Ok(None);
        };
        let start_loc = self.hir.expr(start_expr)?.loc;
        let Some(start) = self.boxed(&start_val, start_loc, scope)? else {
            return Ok(Some(None));
        };
        let start_sig = some!(self.sig(start, need));
        if start_sig.is_error() {
            return Ok(Some(None));
        }
        let Some(start_layout) = self.loop_layout(start, need)? else {
            return Ok(None);
        };
        Ok(Some(Some(Head {
            iterable,
            element,
            start,
            start_sig,
            start_layout,
        })))
    }

    fn loop_layout(
        &mut self,
        node: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<Layout>>, Internal> {
        let Some(found) = self.layout_of(node, need)? else {
            return Ok(None);
        };
        Ok(Some(found.shaped.ok().flatten()))
    }

    fn loop_annotation(&self, ty: &TypeRef) -> Option<(Sig, Option<Layout>)> {
        let annotation = self.annotation(ty)?;
        let sig = Sig::of(
            kind_of(annotation.scalar),
            Width::Fixed(annotation.scalar),
            false,
            None,
        );
        let layout = match annotation.capacity {
            Some(_) => Some(Layout::variable(annotation.lengths)),
            None if annotation.lengths.is_empty() => None,
            None => Some(Layout::fixed(annotation.lengths)),
        };
        Some((sig, layout))
    }

    fn loop_body(
        &mut self,
        expr: ExprId,
        for_expr: &ForExpr,
        head: &Head,
        types: &LoopTypes,
        scope: ScopeId,
    ) -> Result<ScopeId, Internal> {
        let loc = self.hir.expr(expr)?.loc;

        let typed = |sig: Sig| Sig {
            reactive: false,
            frame: true,
            open: false,
            ..sig
        };
        let source = match head.iterable {
            ForIterable::Values(values) => Some(values),
            ForIterable::Range(..) => None,
        };
        let element_name = match for_expr.names.as_slice() {
            [one] => Some(one),
            [_, element, ..] => Some(element),
            [] => None,
        };
        let element_type = self.named_type(element_name, head.element.clone());
        let element = self.push_slot(Slot {
            sig: typed(element_type.0),
            layout: element_type.1,
            capacity: None,
            source,
            annotated: element_name.is_some_and(|var| var.ty.is_some()),
        });
        let index = match for_expr.names.as_slice() {
            [index, _, ..] => {
                let integer = (Sig::of(Kind::Int, Width::Flexible, false, None), None);
                let (sig, layout) = self.named_type(Some(index), integer);
                Some(self.push_slot(Slot {
                    sig: typed(sig),
                    layout,
                    capacity: None,
                    source: None,
                    annotated: false,
                }))
            }
            [] | [_] => None,
        };
        let annotated_capacity = for_expr
            .accumulator_ty
            .as_ref()
            .and_then(|ty| self.annotation(ty))
            .and_then(|annotation| annotation.capacity);
        let accumulator = self.push_slot(Slot {
            sig: typed(types.accumulator.0),
            layout: types.accumulator.1.clone(),
            capacity: annotated_capacity,
            source: None,
            annotated: for_expr.accumulator_ty.is_some(),
        });
        let template = self.template_of(scope)?;
        let chain = self.chain(scope)?;
        let body_scope = self.scope_in(
            ScopeKind::Locals(Locals {
                owner: LocalsOwner::For(expr),
                params: Vec::new(),
                names: Rc::new(BTreeMap::new()),
                recursive: false,
            }),
            Some(scope),
            chain,
            template,
        )?;
        let mut slots: Vec<(Option<&LoopVar>, SlotId)> = Vec::new();
        match for_expr.names.as_slice() {
            [one] => slots.push((Some(one), element)),
            [first, second, ..] => {
                if let Some(index) = index {
                    slots.push((Some(first), index));
                }
                slots.push((Some(second), element));
            }
            [] => {}
        }
        let mut params = Vec::new();
        let mut sources = Vec::new();
        let mut names: BTreeMap<String, LocalName> = BTreeMap::new();
        let mut bound = |engine: &mut Self,
                         name: Option<&crate::semantics::ir::Ident>,
                         slot: SlotId|
         -> Result<(), Internal> {
            let at = name.map_or(loc, |name| name.loc);
            let node = engine.push_node(Op::Slot(slot), at, body_scope)?;
            let position =
                u32::try_from(params.len()).map_err(|_| Internal::missed("loop name"))?;
            if let Some(name) = name {
                names
                    .entry(name.text.clone())
                    .or_insert(LocalName::Param(position));
            }
            params.push(Param {
                name: name.cloned(),
                default: None,
                is_default: false,
                loc: at,
            });
            sources.push(ParamSource::Bound(BoundVal::Scalar(Bound::Value(node))));
            Ok(())
        };
        for (var, slot) in slots {
            bound(self, var.and_then(|var| var.name.as_ref()), slot)?;
        }
        bound(self, for_expr.accumulator.as_ref(), accumulator)?;
        for (position, local) in for_expr.lets.iter().enumerate() {
            if let Some(name) = &local.name {
                let position =
                    u32::try_from(position).map_err(|_| Internal::missed("local let"))?;
                names
                    .entry(name.text.clone())
                    .or_insert(LocalName::Let(position));
            }
        }
        self.memo.loop_params.insert(expr, params);
        self.memo
            .loop_slots
            .insert(body_scope, (index, element, accumulator));
        self.scopes
            .get_mut(body_scope.index())
            .ok_or(Internal::missed("scope"))?
            .kind = ScopeKind::Locals(Locals {
            owner: LocalsOwner::For(expr),
            params: sources,
            names: Rc::new(names),
            recursive: false,
        });
        Ok(body_scope)
    }

    fn named_type(
        &self,
        var: Option<&LoopVar>,
        own: (Sig, Option<Layout>),
    ) -> (Sig, Option<Layout>) {
        match var
            .and_then(|var| var.ty.as_ref())
            .and_then(|ty| self.loop_annotation(ty))
        {
            Some((sig, layout)) => (
                Sig {
                    family: own.0.family,
                    ..sig
                },
                layout,
            ),
            None => own,
        }
    }

    fn push_slot(&mut self, slot: Slot) -> SlotId {
        let id = SlotId(id_index(self.slots.len()));
        self.slots.push(slot);
        id
    }

    fn loop_node(
        &mut self,
        head: &Head,
        body: NodeId,
        template: ScopeId,
        (loc, scope): (Loc, ScopeId),
    ) -> Result<NodeId, Internal> {
        let (index, element, accumulator) = *self
            .memo
            .loop_slots
            .get(&template)
            .ok_or(Internal::missed("loop slots"))?;
        self.push_node(
            Op::For(Box::new(ForNode {
                iterable: head.iterable,
                start: head.start,
                body,
                index,
                element: Some(element),
                accumulator,
            })),
            loc,
            scope,
        )
    }
}

fn unified(start: Sig, body: Sig) -> Sig {
    match unify_kinds(start.kind, body.kind) {
        Some(kind) => Sig::of(
            kind,
            branch_width(kind, &start, &body),
            start.reactive || body.reactive,
            if start.family == body.family {
                start.family
            } else {
                start.family.or(body.family)
            },
        ),
        None => start,
    }
}

fn merged_layout(start: Option<&Layout>, body: Option<Layout>) -> Option<Option<Layout>> {
    match (start, body) {
        (None, None) => Some(None),
        (Some(start), Some(body)) => {
            if *start == body && !start.variable {
                return Some(Some(body));
            }
            let empty = start.outer() == Some(0) && start.inner().is_empty();
            let inner = if empty {
                body.inner().to_vec()
            } else {
                start.inner().to_vec()
            };
            (body.inner() == inner.as_slice()).then(|| Some(Layout::variable(inner)))
        }
        (Some(_), None) | (None, Some(_)) => None,
    }
}

const ITERABLE: &str = "a `for` iterates over a range or an array";

const RANGE_NAMES: &str =
    "a range binds one name; two names iterate an array, its positions and its elements";

const LAYOUTS: &str =
    "the start and the body of a `for` expression hold arrays of different shapes";

fn holds_annotated(element: Kind, annotated: Kind) -> bool {
    element == annotated
        || matches!(element, Kind::Any | Kind::Error | Kind::Bare)
        || element == Kind::Int && annotated == Kind::Float
}

fn settled(next: &LoopTypes, assumed: &LoopTypes) -> bool {
    let (next_sig, next_layout) = &next.accumulator;
    let (sig, layout) = &assumed.accumulator;
    (next_sig.kind == sig.kind || sig.kind == Kind::Any)
        && next_sig.family == sig.family
        && next_layout == layout
}
