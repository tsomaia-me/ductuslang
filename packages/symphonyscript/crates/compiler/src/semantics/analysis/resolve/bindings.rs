use std::rc::Rc;

use symphonyscript_core::{AttrDefault, AttrId, Scalar};

use crate::diagnostics::Code;
use crate::semantics::analysis::attr_source::AttrValueSource;
use crate::semantics::analysis::binding::{Binding, BindingKind};
use crate::semantics::analysis::bound::{Bound, BoundVal};
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::query::{LetRef, Query};
use crate::semantics::analysis::scope::{LocalsOwner, ParamSource, ScopeKind};
use crate::semantics::analysis::shape::Shape;
use crate::semantics::analysis::sig::{Kind, Sig, Width, kind_of, widens};
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{InputId, LetDecl, Loc, TypeBase, TypeRef};

pub(crate) struct BindSpec<'n> {
    pub kind: BindingKind,
    pub annotation: Option<&'n Annotation>,
    pub name: &'n str,
    pub loc: Loc,
    pub scope: ScopeId,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Annotation {
    pub scalar: Scalar,
    pub lengths: Vec<u32>,
}

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn run_let(
        &mut self,
        scope: ScopeId,
        local: LetRef,
        need: &mut Vec<Query>,
    ) -> bool {
        let Some(decl) = self.let_decl(scope, local) else {
            self.memo.lets.insert((scope, local), BoundVal::Error);
            return true;
        };
        let Some(val) = self.val(Query::Expr(decl.value, scope), need) else {
            return false;
        };
        let Some(sigs) = self.element_sigs(&val, need) else {
            return false;
        };
        let annotation = decl.ty.as_ref().and_then(|ty| self.annotation(ty));
        let name = decl
            .name
            .as_ref()
            .map(|name| name.text.clone())
            .unwrap_or_default();
        let spec = BindSpec {
            kind: BindingKind::Let,
            annotation: annotation.as_ref(),
            name: &name,
            loc: decl.loc,
            scope,
        };
        let bound = self.bind(&val, &sigs, &spec);
        self.memo.lets.insert((scope, local), bound);
        true
    }

    pub(in crate::semantics::analysis) fn let_decl(
        &self,
        scope: ScopeId,
        local: LetRef,
    ) -> Option<&LetDecl> {
        match local {
            LetRef::Top(id) => Some(self.hir.let_decl(id)),
            LetRef::Local(index) => {
                let ScopeKind::Locals(locals) = &self.scope_of(scope).kind else {
                    return None;
                };
                let lets = match locals.owner {
                    LocalsOwner::Fn(function) => &self.hir.fn_decl(function).lets,
                    LocalsOwner::Pattern(pattern) => &self.hir.pattern(pattern).lets,
                };
                lets.get(index as usize)
            }
        }
    }

    pub(in crate::semantics::analysis) fn run_param(
        &mut self,
        scope: ScopeId,
        index: u32,
        need: &mut Vec<Query>,
    ) -> bool {
        let ScopeKind::Locals(locals) = &self.scope_of(scope).kind else {
            self.memo.params.insert((scope, index), BoundVal::Error);
            return true;
        };
        let owner = locals.owner;
        let source = locals
            .params
            .get(index as usize)
            .cloned()
            .unwrap_or(ParamSource::Missing);
        let params = match owner {
            LocalsOwner::Fn(function) => &self.hir.fn_decl(function).params,
            LocalsOwner::Pattern(pattern) => &self.hir.pattern(pattern).params,
        };
        let Some(param) = params.get(index as usize) else {
            self.memo.params.insert((scope, index), BoundVal::Error);
            return true;
        };
        let name = param
            .name
            .as_ref()
            .map(|name| name.text.clone())
            .unwrap_or_default();
        let (val, loc) = match source {
            ParamSource::Bound(bound) => {
                self.memo.params.insert((scope, index), bound);
                return true;
            }
            ParamSource::Missing => {
                self.memo.params.insert((scope, index), BoundVal::Error);
                return true;
            }
            ParamSource::Arg {
                expr,
                scope: caller,
            } => {
                let loc = self.hir.expr(expr).loc;
                (self.val(Query::Expr(expr, caller), need), loc)
            }
            ParamSource::Default => {
                let Some(default) = param.default else {
                    self.memo.params.insert((scope, index), BoundVal::Error);
                    return true;
                };
                let prefix = self.prefix_scope(scope, index);
                let loc = self.hir.expr(default).loc;
                (self.val(Query::Expr(default, prefix), need), loc)
            }
        };
        let Some(val) = val else {
            return false;
        };
        let Some(sigs) = self.element_sigs(&val, need) else {
            return false;
        };
        let spec = BindSpec {
            kind: BindingKind::Param,
            annotation: None,
            name: &name,
            loc,
            scope,
        };
        let bound = self.bind(&val, &sigs, &spec);
        self.memo.params.insert((scope, index), bound);
        true
    }

    fn prefix_scope(&mut self, locals: ScopeId, count: u32) -> ScopeId {
        if let Some(existing) = self.memo.prefix_scopes.get(&(locals, count)) {
            return *existing;
        }
        let parent = self.scope_of(locals).parent;
        let chain = self.chain(locals);
        let template = self.template_of(locals);
        let created = self.scope_in(
            ScopeKind::ParamPrefix { locals, count },
            parent,
            chain,
            template,
        );
        self.memo.prefix_scopes.insert((locals, count), created);
        created
    }

    pub(in crate::semantics::analysis) fn run_input(
        &mut self,
        input: InputId,
        need: &mut Vec<Query>,
    ) -> bool {
        let decl = self.hir.input(input);
        let global = self.source_global;
        let Some(val) = self.val(Query::Expr(decl.value, global), need) else {
            return false;
        };
        let Some(sigs) = self.element_sigs(&val, need) else {
            return false;
        };
        let loc = self.hir.expr(decl.value).loc;
        if sigs.iter().any(|sig| sig.reactive) {
            self.error(
                Code::ReactiveInitializer,
                loc,
                global,
                "an input's initializer is a compile-time value and reads no input",
            );
            self.memo.inputs.insert(input, BoundVal::Error);
            return true;
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
                self.unplaced_variant(node);
            }
            self.memo.inputs.insert(input, BoundVal::Error);
            return true;
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
            self.error(Code::InputValue, loc, global, message);
            self.memo.inputs.insert(input, BoundVal::Error);
            return true;
        }
        let annotation = decl.ty.as_ref().and_then(|ty| self.annotation(ty));
        if let Some(annotation) = &annotation
            && annotation.lengths != val.shape().lengths()
        {
            self.error(
                Code::TypeMismatch,
                loc,
                global,
                "the initializer's shape differs from the input's type",
            );
            self.memo.inputs.insert(input, BoundVal::Error);
            return true;
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
        true
    }

    pub(in crate::semantics::analysis) fn run_attr(
        &mut self,
        item: ItemRef,
        attr: AttrId,
        need: &mut Vec<Query>,
    ) -> bool {
        let Some(decl) = self.attr_decl(item, attr).cloned() else {
            self.memo.attrs.insert((item, attr), BoundVal::Error);
            return true;
        };
        let source = self
            .attr_sources(item)
            .iter()
            .find(|source| source.attr == attr)
            .copied();
        let item_loc = self.item_loc(item);
        let item_scope = self.item_scope(item);
        let scalar = decl.ty.scalar;
        let bound = match source.map(|source| source.value) {
            Some(AttrValueSource::Expr(expr, scope)) => {
                let Some(val) = self.val(Query::Expr(expr, scope), need) else {
                    return false;
                };
                let Some(sigs) = self.element_sigs(&val, need) else {
                    return false;
                };
                if val.shape().lengths() != decl.ty.lengths {
                    BoundVal::Error
                } else {
                    let annotation = Annotation {
                        scalar,
                        lengths: decl.ty.lengths.clone(),
                    };
                    let loc = self.hir.expr(expr).loc;
                    let spec = BindSpec {
                        kind: BindingKind::Attr { item, attr },
                        annotation: Some(&annotation),
                        name: &decl.name,
                        loc,
                        scope,
                    };
                    self.bind_attr(&val, &sigs, &spec)
                }
            }
            Some(AttrValueSource::Set(value)) => {
                let node = self.push_node(Op::Bool(value), item_loc, item_scope);
                BoundVal::Scalar(Bound::Value(node))
            }
            None => match &decl.default {
                Some(AttrDefault::Elements(values)) => {
                    let nodes: Vec<Bound> = values
                        .iter()
                        .map(|value| {
                            Bound::Value(self.push_node(Op::Const(*value), item_loc, item_scope))
                        })
                        .collect();
                    if decl.ty.lengths.is_empty() {
                        nodes
                            .first()
                            .map_or(BoundVal::Error, |node| BoundVal::Scalar(*node))
                    } else {
                        BoundVal::Array(Rc::new((Shape::from_lengths(&decl.ty.lengths), nodes)))
                    }
                }
                Some(AttrDefault::Text(text)) => {
                    let node = self.push_node(Op::Str(text.clone()), item_loc, item_scope);
                    BoundVal::Scalar(Bound::Value(node))
                }
                None => BoundVal::Error,
            },
        };
        self.memo.attrs.insert((item, attr), bound);
        true
    }

    fn bind_attr(&mut self, val: &Val, sigs: &[Sig], spec: &BindSpec<'_>) -> BoundVal {
        let Some(annotation) = spec.annotation else {
            return BoundVal::Error;
        };
        let (kind, name, loc, scope) = (spec.kind, spec.name, spec.loc, spec.scope);
        let one = |engine: &mut Self, node: NodeId, sig: &Sig| -> Bound {
            if !sig.reactive {
                return Bound::Value(engine.push_node(
                    Op::Annotate(annotation.scalar, node),
                    loc,
                    scope,
                ));
            }
            if let Op::Read(binding) = engine.node(node).op {
                return Bound::Cell(binding);
            }
            Bound::Cell(engine.push_binding(Binding {
                kind,
                init: node,
                annotation: Some(annotation.scalar),
                name: name.to_string(),
                loc,
            }))
        };
        match val {
            Val::Scalar(node) => match sigs.first() {
                Some(sig) => BoundVal::Scalar(one(self, *node, sig)),
                None => BoundVal::Error,
            },
            Val::Array(array) => {
                let items = array
                    .elements
                    .iter()
                    .zip(sigs)
                    .map(|(node, sig)| one(self, *node, sig))
                    .collect();
                BoundVal::Array(Rc::new((array.shape.clone(), items)))
            }
            Val::Error => BoundVal::Error,
        }
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
    ) -> BoundVal {
        let (kind, annotation, name, loc, scope) =
            (spec.kind, spec.annotation, spec.name, spec.loc, spec.scope);
        if let Some(annotation) = annotation
            && annotation.lengths != val.shape().lengths()
            && !matches!(val, Val::Error)
        {
            self.error(
                Code::TypeMismatch,
                loc,
                scope,
                "the value's shape differs from its annotated type",
            );
            return BoundVal::Error;
        }
        let scalar = annotation.map(|annotation| annotation.scalar);
        if let Some(target) = scalar {
            for (node, sig) in val.nodes().into_iter().zip(sigs) {
                if !annotated(sig, target) {
                    self.error_at_node(
                        Code::TypeMismatch,
                        node,
                        "the value does not have the annotated type; a conversion states the rounding",
                    );
                    return BoundVal::Error;
                }
            }
        }
        let one = |engine: &mut Self, node: NodeId, sig: &Sig, alias: bool| -> Bound {
            if !sig.reactive {
                return Bound::Value(match scalar {
                    Some(scalar) => engine.push_node(Op::Annotate(scalar, node), loc, scope),
                    None => node,
                });
            }
            if alias
                && scalar.is_none()
                && let Op::Read(binding) = engine.node(node).op
            {
                return Bound::Cell(binding);
            }
            Bound::Cell(engine.push_binding(Binding {
                kind,
                init: node,
                annotation: scalar,
                name: name.to_string(),
                loc,
            }))
        };
        match val {
            Val::Scalar(node) => match sigs.first() {
                Some(sig) => BoundVal::Scalar(one(self, *node, sig, false)),
                None => BoundVal::Error,
            },
            Val::Array(array) => {
                let items = array
                    .elements
                    .iter()
                    .zip(sigs)
                    .map(|(node, sig)| one(self, *node, sig, true))
                    .collect();
                BoundVal::Array(Rc::new((array.shape.clone(), items)))
            }
            Val::Error => BoundVal::Error,
        }
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
