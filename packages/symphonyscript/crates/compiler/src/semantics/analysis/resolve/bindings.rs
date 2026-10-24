use std::rc::Rc;

use symphonyscript_core::{AttrDefault, AttrId, Scalar};

use super::annotation::Annotation;
use super::bind_spec::BindSpec;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::attr_value_source::AttrValueSource;
use crate::semantics::analysis::binding::Binding;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::combined_shape::combined_shape;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::eval_ty_of::eval_ty_of;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::kind_of::kind_of;
use crate::semantics::analysis::lengths_of::lengths_of;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::shape::Shape;
use crate::semantics::analysis::shape_fault::ShapeFault;
use crate::semantics::analysis::shape_parts::shape_parts;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::analysis::widens::widens;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::{ExprId, InputId, LetDecl, Loc, TypeBase, TypeRef};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn run_let(
        &mut self,
        scope: ScopeId,
        local: LetRef,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let Some(decl) = self.let_decl(scope, local)? else {
            self.memo.lets.insert((scope, local), BoundVal::Error);
            return Ok(true);
        };
        let Some(val) = self.val(Query::Expr(decl.value, scope), need) else {
            return Ok(false);
        };
        let annotation = decl.ty.as_ref().and_then(|ty| self.annotation(ty));
        let loc = decl.loc;
        let name = decl
            .name
            .as_ref()
            .map(|name| name.text.clone())
            .unwrap_or_default();
        let Some((val, defaulted)) =
            self.reactive_elements(val, annotation.as_ref(), loc, scope, need)?
        else {
            return Ok(false);
        };
        let Some(sigs) = self.element_sigs(&val, need) else {
            return Ok(false);
        };
        let spec = BindSpec {
            kind: BindingKind::Let,
            annotation: annotation.as_ref(),
            name: &name,
            loc,
            scope,
        };
        let bound = self.bind(&val, &sigs, &spec)?;
        self.note_defaulted(defaulted, &bound);
        self.memo.lets.insert((scope, local), bound);
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn let_decl(
        &self,
        scope: ScopeId,
        local: LetRef,
    ) -> Result<Option<&LetDecl>, Internal> {
        Ok(match local {
            LetRef::Top(id) => Some(self.hir.let_decl(id)?),
            LetRef::Local(index) => {
                let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
                    return Ok(None);
                };
                let lets = match locals.owner {
                    LocalsOwner::Fn(function) => &self.hir.fn_decl(function)?.lets,
                    LocalsOwner::Pattern(pattern) => &self.hir.pattern(pattern)?.lets,
                };
                Some(
                    lets.get(index as usize)
                        .ok_or(Internal::missed("local let"))?,
                )
            }
        })
    }

    pub(in crate::semantics::analysis) fn run_param(
        &mut self,
        scope: ScopeId,
        index: u32,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
            self.memo.params.insert((scope, index), BoundVal::Error);
            return Ok(true);
        };
        let owner = locals.owner;
        let source = locals
            .params
            .get(index as usize)
            .cloned()
            .unwrap_or(ParamSource::Missing);
        let params = match owner {
            LocalsOwner::Fn(function) => &self.hir.fn_decl(function)?.params,
            LocalsOwner::Pattern(pattern) => &self.hir.pattern(pattern)?.params,
        };
        let param = params
            .get(index as usize)
            .ok_or(Internal::missed("parameter"))?;
        let name = param
            .name
            .as_ref()
            .map(|name| name.text.clone())
            .unwrap_or_default();
        let (val, loc) = match source {
            ParamSource::Bound(bound) => {
                self.memo.params.insert((scope, index), bound);
                return Ok(true);
            }
            ParamSource::Missing => {
                self.memo.params.insert((scope, index), BoundVal::Error);
                return Ok(true);
            }
            ParamSource::Arg {
                expr,
                scope: caller,
            } => {
                let loc = self.hir.expr(expr)?.loc;
                (self.val(Query::Expr(expr, caller), need), loc)
            }
            ParamSource::Default => {
                let Some(default) = param.default else {
                    self.memo.params.insert((scope, index), BoundVal::Error);
                    return Ok(true);
                };
                let prefix = self.prefix_scope(scope, index)?;
                let loc = self.hir.expr(default)?.loc;
                (self.val(Query::Expr(default, prefix), need), loc)
            }
        };
        let Some(val) = val else {
            return Ok(false);
        };
        let Some((val, defaulted)) = self.reactive_elements(val, None, loc, scope, need)? else {
            return Ok(false);
        };
        let Some(sigs) = self.element_sigs(&val, need) else {
            return Ok(false);
        };
        let spec = BindSpec {
            kind: BindingKind::Param,
            annotation: None,
            name: &name,
            loc,
            scope,
        };
        let bound = self.bind(&val, &sigs, &spec)?;
        self.note_defaulted(defaulted, &bound);
        self.memo.params.insert((scope, index), bound);
        Ok(true)
    }

    fn prefix_scope(&mut self, locals: ScopeId, count: u32) -> Result<ScopeId, Internal> {
        if let Some(existing) = self.memo.prefix_scopes.get(&(locals, count)) {
            return Ok(*existing);
        }
        let parent = self.scope_of(locals)?.parent;
        let chain = self.chain(locals)?;
        let template = self.template_of(locals)?;
        let created = self.scope_in(
            ScopeKind::ParamPrefix { locals, count },
            parent,
            chain,
            template,
        );
        self.memo.prefix_scopes.insert((locals, count), created);
        Ok(created)
    }

    pub(in crate::semantics::analysis) fn run_input(
        &mut self,
        input: InputId,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let decl = self.hir.input(input)?;
        let global = self.source_global;
        let Some(val) = self.val(Query::Expr(decl.value, global), need) else {
            return Ok(false);
        };
        let annotation = decl.ty.as_ref().and_then(|ty| self.annotation(ty));
        let Some(val) = self.input_elements(val, annotation.as_ref(), decl.value, need)? else {
            return Ok(false);
        };
        let Some(sigs) = self.element_sigs(&val, need) else {
            return Ok(false);
        };
        let loc = self.hir.expr(decl.value)?.loc;
        if sigs.iter().any(|sig| sig.reactive) {
            self.error(
                Code::ReactiveInitializer,
                loc,
                global,
                "an input's initializer is a compile-time value and reads no input",
            )?;
            self.memo.inputs.insert(input, BoundVal::Error);
            return Ok(true);
        }
        let bare: Vec<NodeId> = val
            .nodes()
            .into_iter()
            .zip(&sigs)
            .filter(|(_, sig)| sig.kind == Kind::Bare)
            .map(|(node, _)| node)
            .collect();
        if !bare.is_empty() {
            for node in bare {
                self.unplaced_variant(node)?;
            }
            self.memo.inputs.insert(input, BoundVal::Error);
            return Ok(true);
        }
        if let Some(bad) = sigs
            .iter()
            .find(|sig| matches!(sig.kind, Kind::Str | Kind::Node))
        {
            let message = if bad.kind == Kind::Str {
                "an input never holds a string, which exists only at compile time"
            } else {
                "an input never holds a node reference"
            };
            self.error(Code::InputValue, loc, global, message)?;
            self.memo.inputs.insert(input, BoundVal::Error);
            return Ok(true);
        }
        if let Some(annotation) = &annotation
            && annotation.lengths != val.shape().lengths()
        {
            self.error(
                Code::TypeMismatch,
                loc,
                global,
                "the initializer's shape differs from the input's type",
            )?;
            self.memo.inputs.insert(input, BoundVal::Error);
            return Ok(true);
        }
        let name = decl
            .name
            .as_ref()
            .map(|name| name.text.clone())
            .unwrap_or_default();
        let scalar = annotation.as_ref().map(|annotation| annotation.scalar);
        let make = |engine: &mut Self, element: u32, init: NodeId| {
            Bound::Cell(engine.push_binding(Binding {
                kind: BindingKind::Input { input, element },
                init,
                annotation: scalar,
                name: name.clone(),
                loc: decl.loc,
            }))
        };
        let bound = match &val {
            Val::Scalar(node) => BoundVal::Scalar(make(self, 0, *node)),
            Val::Array(array) => {
                let items = array
                    .elements
                    .iter()
                    .enumerate()
                    .map(|(element, node)| {
                        make(self, u32::try_from(element).unwrap_or(u32::MAX), *node)
                    })
                    .collect();
                BoundVal::Array(Rc::new((array.shape.clone(), items)))
            }
            Val::Error => BoundVal::Error,
        };
        self.memo.inputs.insert(input, bound);
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn run_attr(
        &mut self,
        item: ItemRef,
        attr: AttrId,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let Some(decl) = self.attr_decl(item, attr)?.cloned() else {
            self.memo.attrs.insert((item, attr), BoundVal::Error);
            return Ok(true);
        };
        let source = self
            .attr_sources(item)?
            .iter()
            .find(|source| source.attr == attr)
            .copied();
        let item_loc = self.item_loc(item)?;
        let item_scope = self.item_scope(item)?;
        let scalar = decl.ty.scalar;
        let bound = match source.map(|source| source.value) {
            Some(AttrValueSource::Expr(expr, scope)) => {
                let Some(val) = self.val(Query::Expr(expr, scope), need) else {
                    return Ok(false);
                };
                let val = self.attr_elements(val, &decl.ty.lengths, expr, scope)?;
                let Some(sigs) = self.element_sigs(&val, need) else {
                    return Ok(false);
                };
                if val.shape().lengths() != decl.ty.lengths {
                    BoundVal::Error
                } else {
                    let annotation = Annotation {
                        scalar,
                        lengths: decl.ty.lengths.clone(),
                    };
                    let loc = self.hir.expr(expr)?.loc;
                    let spec = BindSpec {
                        kind: BindingKind::Attr { item, attr },
                        annotation: Some(&annotation),
                        name: &decl.name,
                        loc,
                        scope,
                    };
                    self.bind_attr(&val, &sigs, &spec)?
                }
            }
            Some(AttrValueSource::Set(value)) => {
                let node = self.push_node(Op::Bool(value), item_loc, item_scope)?;
                BoundVal::Scalar(Bound::Value(node))
            }
            None => match &decl.default {
                Some(AttrDefault::Elements(values)) => {
                    let nodes: Vec<Bound> = values
                        .iter()
                        .map(|value| {
                            self.push_node(Op::Const(*value), item_loc, item_scope)
                                .map(Bound::Value)
                        })
                        .collect::<Result<_, _>>()?;
                    if decl.ty.lengths.is_empty() {
                        nodes
                            .first()
                            .map_or(BoundVal::Error, |node| BoundVal::Scalar(*node))
                    } else {
                        BoundVal::Array(Rc::new((Shape::from_lengths(&decl.ty.lengths), nodes)))
                    }
                }
                Some(AttrDefault::Text(text)) => {
                    let node = self.push_node(Op::Str(text.clone()), item_loc, item_scope)?;
                    BoundVal::Scalar(Bound::Value(node))
                }
                None => BoundVal::Error,
            },
        };
        self.memo.attrs.insert((item, attr), bound);
        Ok(true)
    }

    fn input_elements(
        &mut self,
        val: Val,
        annotation: Option<&Annotation>,
        expr: ExprId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let Val::Scalar(node) = val else {
            return Ok(Some(val));
        };
        if !self.arrays.contains(&node) {
            return Ok(Some(val));
        }
        let ty = eval_ty_of(annotation.map(|annotation| annotation.scalar));
        let value = some!(self.cval(Query::Eval(node, ty), need));
        let CVal::Array(array) = value else {
            return Ok(Some(val));
        };
        let shape = Shape::from_lengths(&array.lengths());
        let loc = self.hir.expr(expr)?.loc;
        let scope = self.source_global;
        let nodes = self.projected(
            node,
            &shape,
            (
                Code::TypeMismatch,
                "the initializer's shape differs from the input's type",
            ),
            loc,
            scope,
        )?;
        Ok(Some(Val::array(shape, nodes)))
    }

    fn reactive_elements(
        &mut self,
        val: Val,
        annotation: Option<&Annotation>,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<(Val, Option<NodeId>)>, Internal> {
        let Val::Scalar(node) = val else {
            return Ok(Some((val, None)));
        };
        if !self.arrays.contains(&node) || !some!(self.sig(node, need)).reactive {
            return Ok(Some((val, None)));
        }
        if let Some(annotation) = annotation {
            if annotation.lengths.is_empty() {
                return Ok(Some((val, None)));
            }
            let shape = Shape::from_lengths(&annotation.lengths);
            let nodes = self.projected(
                node,
                &shape,
                (Code::TypeMismatch, ANNOTATED_SHAPE),
                loc,
                scope,
            )?;
            return Ok(Some((Val::array(shape, nodes), None)));
        }
        let lengths = match some!(self.default_shape(node, need)?) {
            Ok(lengths) => lengths,
            Err(fault) => {
                if let Some(fault) = fault {
                    self.error_at_node(fault.code, fault.node, fault.message)?;
                }
                return Ok(Some((Val::Error, None)));
            }
        };
        if lengths.is_empty() {
            return Ok(Some((val, None)));
        }
        let shape = Shape::from_lengths(&lengths);
        let nodes = self.projected(
            node,
            &shape,
            (Code::InconsistentShapes, DEFAULT_SHAPE),
            loc,
            scope,
        )?;
        Ok(Some((Val::array(shape, nodes), Some(node))))
    }

    fn default_shape(
        &self,
        root: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Lengths>, Internal> {
        let mut shapes: FastMap<NodeId, Lengths> = FastMap::default();
        let mut stack = vec![(root, false)];
        let mut missing = false;
        while let Some((node, ready)) = stack.pop() {
            if shapes.contains_key(&node) {
                continue;
            }
            let Some(sig) = self.sig(node, need) else {
                missing = true;
                continue;
            };
            if sig.is_error() {
                shapes.insert(node, Err(None));
                continue;
            }
            if !sig.reactive {
                let Some(value) = self.cval(Query::Eval(node, EvalTy::Default), need) else {
                    missing = true;
                    continue;
                };
                let shape = if value.is_error() {
                    Err(None)
                } else {
                    Ok(lengths_of(&value))
                };
                shapes.insert(node, shape);
                continue;
            }
            let op = &self.node(node)?.op;
            let parts = shape_parts(self, op)?;
            if !self.arrays.contains(&node) || parts.is_empty() {
                shapes.insert(node, Ok(Vec::new()));
                continue;
            }
            if !ready {
                stack.push((node, true));
                stack.extend(parts.iter().map(|part| (*part, false)));
                continue;
            }
            let mut found = Vec::new();
            let mut failed = None;
            for part in &parts {
                match shapes.get(part) {
                    Some(Ok(lengths)) => found.push(lengths.clone()),
                    Some(Err(fault)) => {
                        failed = Some(*fault);
                        break;
                    }
                    None => missing = true,
                }
            }
            let shape = match failed {
                Some(fault) => Err(fault),
                None => combined_shape(node, op, &found).map_err(Some),
            };
            shapes.insert(node, shape);
        }
        if missing {
            return Ok(None);
        }
        Ok(shapes.remove(&root))
    }

    fn note_defaulted(&mut self, base: Option<NodeId>, bound: &BoundVal) {
        let (Some(base), BoundVal::Array(array)) = (base, bound) else {
            return;
        };
        let elements = array
            .1
            .iter()
            .filter_map(|item| match item {
                Bound::Cell(binding) => Some(*binding),
                Bound::Value(_) => None,
            })
            .collect();
        self.memo.defaulted.insert(base, elements);
    }

    fn attr_elements(
        &mut self,
        val: Val,
        lengths: &[u32],
        expr: ExprId,
        scope: ScopeId,
    ) -> Result<Val, Internal> {
        let Val::Scalar(node) = val else {
            return Ok(val);
        };
        if !self.arrays.contains(&node) {
            return Ok(val);
        }
        if lengths.is_empty() {
            return Ok(val);
        }
        let shape = Shape::from_lengths(lengths);
        let loc = self.hir.expr(expr)?.loc;
        let nodes = self.projected(
            node,
            &shape,
            (Code::TypeMismatch, ATTRIBUTE_SHAPE),
            loc,
            scope,
        )?;
        Ok(Val::array(shape, nodes))
    }

    fn bind_attr(
        &mut self,
        val: &Val,
        sigs: &[Sig],
        spec: &BindSpec<'_>,
    ) -> Result<BoundVal, Internal> {
        let Some(annotation) = spec.annotation else {
            return Ok(BoundVal::Error);
        };
        let (kind, name, loc, scope) = (spec.kind, spec.name, spec.loc, spec.scope);
        let one = |engine: &mut Self, node: NodeId, sig: &Sig| -> Result<Bound, Internal> {
            if !sig.reactive {
                return Ok(Bound::Value(engine.push_node(
                    Op::Annotate(annotation.scalar, node),
                    loc,
                    scope,
                )?));
            }
            if let Op::Read(binding) = engine.node(node)?.op {
                return Ok(Bound::Cell(binding));
            }
            Ok(Bound::Cell(engine.push_binding(Binding {
                kind,
                init: node,
                annotation: Some(annotation.scalar),
                name: name.to_string(),
                loc,
            })))
        };
        Ok(match val {
            Val::Scalar(node) => match sigs.first() {
                Some(sig) => BoundVal::Scalar(one(self, *node, sig)?),
                None => BoundVal::Error,
            },
            Val::Array(array) => {
                let items = array
                    .elements
                    .iter()
                    .zip(sigs)
                    .map(|(node, sig)| one(self, *node, sig))
                    .collect::<Result<_, _>>()?;
                BoundVal::Array(Rc::new((array.shape.clone(), items)))
            }
            Val::Error => BoundVal::Error,
        })
    }

    pub(in crate::semantics::analysis) fn element_sigs(
        &self,
        val: &Val,
        need: &mut Vec<Query>,
    ) -> Option<Vec<Sig>> {
        let nodes = val.nodes();
        let sigs: Vec<Option<Sig>> = nodes.iter().map(|node| self.sig(*node, need)).collect();
        if sigs.iter().any(Option::is_none) {
            return None;
        }
        Some(sigs.into_iter().flatten().collect())
    }

    pub(in crate::semantics::analysis) fn bind(
        &mut self,
        val: &Val,
        sigs: &[Sig],
        spec: &BindSpec<'_>,
    ) -> Result<BoundVal, Internal> {
        let (kind, annotation, name, loc, scope) =
            (spec.kind, spec.annotation, spec.name, spec.loc, spec.scope);
        if let Some(annotation) = annotation
            && annotation.lengths != val.shape().lengths()
            && !matches!(val, Val::Error)
            && !self.holds_array(val)
        {
            self.error(
                Code::TypeMismatch,
                loc,
                scope,
                "the value's shape differs from its annotated type",
            )?;
            return Ok(BoundVal::Error);
        }
        let scalar = annotation.map(|annotation| annotation.scalar);
        if let Some(target) = scalar {
            for (node, sig) in val.nodes().into_iter().zip(sigs) {
                if !annotated(sig, target) {
                    self.error_at_node(
                        Code::TypeMismatch,
                        node,
                        "the value does not have the annotated type; a conversion states the rounding",
                    )?;
                    return Ok(BoundVal::Error);
                }
            }
        }
        let one =
            |engine: &mut Self, node: NodeId, sig: &Sig, alias: bool| -> Result<Bound, Internal> {
                if !sig.reactive {
                    return Ok(Bound::Value(match scalar {
                        Some(scalar) => engine.push_node(Op::Annotate(scalar, node), loc, scope)?,
                        None => node,
                    }));
                }
                if alias
                    && scalar.is_none()
                    && let Op::Read(binding) = engine.node(node)?.op
                {
                    return Ok(Bound::Cell(binding));
                }
                Ok(Bound::Cell(engine.push_binding(Binding {
                    kind,
                    init: node,
                    annotation: scalar,
                    name: name.to_string(),
                    loc,
                })))
            };
        Ok(match val {
            Val::Scalar(node) => match sigs.first() {
                Some(sig) => {
                    let bound = one(self, *node, sig, false)?;
                    if let (Some(annotation), Bound::Value(annotated)) = (annotation, bound)
                        && self.arrays.contains(node)
                    {
                        self.require_shape(
                            annotated,
                            annotation.lengths.clone(),
                            Code::TypeMismatch,
                            "the value's shape differs from its annotated type",
                        );
                    }
                    BoundVal::Scalar(bound)
                }
                None => BoundVal::Error,
            },
            Val::Array(array) => {
                let items = array
                    .elements
                    .iter()
                    .zip(sigs)
                    .map(|(node, sig)| one(self, *node, sig, true))
                    .collect::<Result<_, _>>()?;
                BoundVal::Array(Rc::new((array.shape.clone(), items)))
            }
            Val::Error => BoundVal::Error,
        })
    }

    pub(in crate::semantics::analysis) fn annotation(&self, ty: &TypeRef) -> Option<Annotation> {
        let scalar = match &ty.base {
            TypeBase::Scalar(scalar) => *scalar,
            TypeBase::Enum(name) => Scalar::Enum(self.vocabulary.enum_named(&name.text)?),
            TypeBase::Error => return None,
        };
        let lengths = ty
            .lengths
            .iter()
            .map(|(length, _)| length.filter(|length| *length > 0))
            .collect::<Option<Vec<u32>>>()?;
        Some(Annotation { scalar, lengths })
    }
}

fn annotated(sig: &Sig, target: Scalar) -> bool {
    if sig.is_error() {
        return true;
    }
    let fits = match (sig.kind, kind_of(target)) {
        (Kind::Any, _) | (Kind::Int, Kind::Float) | (Kind::Bare, Kind::Enum(_)) => true,
        (operand, wanted) => operand == wanted,
    };
    let widens_into = match sig.width {
        Width::Fixed(scalar) if sig.is_numeric() && !sig.reactive => widens(scalar, target),
        Width::Fixed(_) | Width::Flexible | Width::Widened(_) | Width::Adaptive => true,
    };
    fits && widens_into
}

const ATTRIBUTE_SHAPE: &str = "the value's array shape differs from the attribute's type";

type Lengths = Result<Vec<u32>, Option<ShapeFault>>;

const ANNOTATED_SHAPE: &str = "the value's shape differs from its annotated type";

const DEFAULT_SHAPE: &str =
    "a use yields this value in another shape than the defaults give its recursive call";
