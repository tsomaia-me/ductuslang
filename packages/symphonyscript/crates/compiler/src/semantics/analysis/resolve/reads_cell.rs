use std::collections::{BTreeMap, BTreeSet};

use symphonyscript_core::{BinaryOp, Builtin};

use crate::semantics::Internal;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::query::Query;
use crate::semantics::collection::{BuiltinName, Decl, Namespace};
use crate::semantics::ir::{ExprId, ExprKind, FnId, StateId, TypeRef};

impl Engine<'_> {
    pub(super) fn body_reads_cell(
        &mut self,
        root: FnId,
        need: &mut Vec<Query>,
    ) -> Result<Option<bool>, Internal> {
        if let Some(known) = self.memo.reads_cell.get(&root) {
            return Ok(Some(*known));
        }
        let hir = self.hir;
        let mut seen = FastSet::default();
        let mut functions = vec![root];
        let mut reads = false;
        let mut missing = false;
        while let Some(function) = functions.pop() {
            if !seen.insert(function) {
                continue;
            }
            let decl = hir.fn_decl(function)?;
            let (namespace, global) = if hir.is_prelude_fn(function) {
                (&self.collected.prelude, self.prelude_global)
            } else {
                (&self.collected.source, self.source_global)
            };
            let locals: BTreeSet<&str> = decl
                .params
                .iter()
                .filter_map(|param| param.name.as_ref())
                .chain(decl.lets.iter().filter_map(|local| local.name.as_ref()))
                .map(|name| name.text.as_str())
                .collect();
            let mut stack: Vec<_> = decl
                .params
                .iter()
                .filter_map(|param| param.default)
                .collect();
            stack.extend(decl.lets.iter().map(|local| local.value));
            stack.push(decl.value);
            stack.extend(decl.bound.iter().map(|bound| bound.fallback));
            while let Some(expr) = stack.pop() {
                let kind = &hir.expr(expr)?.kind;
                match kind {
                    ExprKind::Name(name) if !locals.contains(name.as_str()) => {
                        match namespace.get(name) {
                            Some(Decl::Input(_) | Decl::State(_)) => reads = true,
                            Some(Decl::Let(id)) => {
                                match self.bound(Query::Let(global, LetRef::Top(id)), need) {
                                    Some(bound) => reads |= holds_cell(&bound),
                                    None => missing = true,
                                }
                            }
                            Some(Decl::Fn(callee)) => functions.push(callee),
                            Some(
                                Decl::Native(_)
                                | Decl::Pattern(_)
                                | Decl::Instance(_)
                                | Decl::Builtin(_)
                                | Decl::Node(_)
                                | Decl::Wire(_)
                                | Decl::Enum(_),
                            )
                            | None => {}
                        }
                    }
                    ExprKind::Field(..) => reads = true,
                    _ => {}
                }
                stack.extend(kind.children());
            }
        }
        if missing {
            return Ok(None);
        }
        self.memo.reads_cell.insert(root, reads);
        Ok(Some(reads))
    }

    pub(in crate::semantics::analysis) fn body_certainly_reads_cell(
        &self,
        root: FnId,
    ) -> Result<bool, Internal> {
        let hir = self.hir;
        let mut seen_functions = FastSet::default();
        let mut seen_lets = FastSet::default();
        let mut functions = vec![root];
        let mut lets = Vec::new();
        let empty_lets = BTreeMap::new();
        loop {
            let (namespace, value, params, locals) = if let Some(id) = lets.pop() {
                if !seen_lets.insert(id) || hir.is_prelude_let(id) {
                    continue;
                }
                let value = hir.let_decl(id)?.value;
                (
                    &self.collected.source,
                    value,
                    BTreeSet::new(),
                    empty_lets.clone(),
                )
            } else if let Some(function) = functions.pop() {
                if !seen_functions.insert(function) {
                    continue;
                }
                let decl = hir.fn_decl(function)?;
                let namespace = if hir.is_prelude_fn(function) {
                    &self.collected.prelude
                } else {
                    &self.collected.source
                };
                let params: BTreeSet<&str> = decl
                    .params
                    .iter()
                    .filter_map(|param| param.name.as_ref())
                    .map(|name| name.text.as_str())
                    .collect();
                let locals: BTreeMap<&str, ExprId> = decl
                    .lets
                    .iter()
                    .filter_map(|local| {
                        local
                            .name
                            .as_ref()
                            .map(|name| (name.text.as_str(), local.value))
                    })
                    .collect();
                (namespace, decl.value, params, locals)
            } else {
                return Ok(false);
            };
            let mut read_locals = BTreeSet::new();
            let mut stack = vec![value];
            while let Some(expr) = stack.pop() {
                let kind = &hir.expr(expr)?.kind;
                match kind {
                    ExprKind::Name(name) if params.contains(name.as_str()) => {}
                    ExprKind::Name(name) if locals.contains_key(name.as_str()) => {
                        if read_locals.insert(name.as_str())
                            && let Some(local) = locals.get(name.as_str())
                        {
                            stack.push(*local);
                        }
                    }
                    ExprKind::Name(name) => match namespace.get(name) {
                        Some(Decl::Input(_) | Decl::State(_)) => return Ok(true),
                        Some(Decl::Let(id)) => lets.push(id),
                        Some(Decl::Fn(callee)) => functions.push(callee),
                        Some(
                            Decl::Native(_)
                            | Decl::Pattern(_)
                            | Decl::Instance(_)
                            | Decl::Builtin(_)
                            | Decl::Node(_)
                            | Decl::Wire(_)
                            | Decl::Enum(_),
                        )
                        | None => {}
                    },
                    ExprKind::Call(callee, args)
                        if measures_length(
                            &hir.expr(*callee)?.kind,
                            &params,
                            &locals,
                            namespace,
                        ) =>
                    {
                        if self.measures_variable_cell(
                            args.values(),
                            (&params, &locals),
                            namespace,
                        )? {
                            return Ok(true);
                        }
                        continue;
                    }
                    _ => {}
                }
                stack.extend(kind.children());
            }
        }
    }
}

impl Engine<'_> {
    fn measures_variable_cell(
        &self,
        args: Vec<ExprId>,
        (params, lets): (&BTreeSet<&str>, &BTreeMap<&str, ExprId>),
        namespace: &Namespace,
    ) -> Result<bool, Internal> {
        self.variable_array(
            args.into_iter().map(|arg| (arg, false)).collect(),
            (params, lets),
            namespace,
        )
    }

    fn variable_array(
        &self,
        mut stack: Vec<(ExprId, bool)>,
        (params, lets): (&BTreeSet<&str>, &BTreeMap<&str, ExprId>),
        namespace: &Namespace,
    ) -> Result<bool, Internal> {
        let mut seen_lets = FastSet::default();
        while let Some((expr, top_level)) = stack.pop() {
            match &self.hir.expr(expr)?.kind {
                ExprKind::Name(name)
                    if top_level
                        || !params.contains(name.as_str()) && !lets.contains_key(name.as_str()) =>
                {
                    match namespace.get(name) {
                        Some(Decl::Input(id)) => {
                            if annotated_capacity(self.hir.input(id)?.ty.as_ref()) {
                                return Ok(true);
                            }
                        }
                        Some(Decl::State(id)) => {
                            if annotated_capacity(self.hir.state(id)?.ty.as_ref())
                                || self.settled_variable(id)
                            {
                                return Ok(true);
                            }
                        }
                        Some(Decl::Let(id)) => {
                            if self.hir.is_prelude_let(id) || !seen_lets.insert(id) {
                                continue;
                            }
                            let decl = self.hir.let_decl(id)?;
                            if annotated_capacity(decl.ty.as_ref()) {
                                return Ok(true);
                            }
                            stack.push((decl.value, true));
                        }
                        Some(_) | None => {}
                    }
                }
                ExprKind::Binary(BinaryOp::Add, lhs, rhs) => {
                    stack.extend([(*lhs, top_level), (*rhs, top_level)]);
                }
                ExprKind::Slice { base, .. } => stack.push((*base, top_level)),
                ExprKind::If(condition, then, otherwise) => {
                    if let ExprKind::Bool(taken) = self.hir.expr(*condition)?.kind {
                        stack.push((if taken { *then } else { *otherwise }, top_level));
                    } else if self.variable_choice(
                        (*condition, &[*then, *otherwise], top_level),
                        (params, lets),
                        namespace,
                    )? {
                        return Ok(true);
                    }
                }
                ExprKind::Match {
                    scrutinee,
                    arms,
                    otherwise,
                } => {
                    let values: Vec<ExprId> =
                        arms.iter().map(|arm| arm.value).chain(*otherwise).collect();
                    if self.variable_choice(
                        (*scrutinee, &values, top_level),
                        (params, lets),
                        namespace,
                    )? {
                        return Ok(true);
                    }
                }
                _ => {}
            }
        }
        Ok(false)
    }

    fn variable_choice(
        &self,
        (selector, arms, top_level): (ExprId, &[ExprId], bool),
        scope: (&BTreeSet<&str>, &BTreeMap<&str, ExprId>),
        namespace: &Namespace,
    ) -> Result<bool, Internal> {
        let mut variable = Vec::with_capacity(arms.len());
        for arm in arms {
            variable.push(self.variable_array(vec![(*arm, top_level)], scope, namespace)?);
        }
        if !self.selector_reads_cell((selector, top_level), scope, namespace)? {
            return Ok(variable.iter().all(|certain| *certain));
        }
        if variable.iter().any(|certain| *certain) {
            return Ok(true);
        }
        let mut lengths = Vec::new();
        for arm in arms {
            if let ExprKind::Array(elements) = &self.hir.expr(*arm)?.kind {
                lengths.push(elements.len());
            }
        }
        Ok(lengths.iter().any(|length| Some(length) != lengths.first()))
    }

    fn selector_reads_cell(
        &self,
        (selector, top_level): (ExprId, bool),
        (params, lets): (&BTreeSet<&str>, &BTreeMap<&str, ExprId>),
        namespace: &Namespace,
    ) -> Result<bool, Internal> {
        let mut stack = vec![(selector, top_level)];
        let mut seen_lets = FastSet::default();
        let mut seen_locals = BTreeSet::new();
        while let Some((expr, top_level)) = stack.pop() {
            let kind = &self.hir.expr(expr)?.kind;
            if let ExprKind::Name(name) = kind {
                if !top_level && params.contains(name.as_str()) {
                    continue;
                }
                if !top_level && let Some(local) = lets.get(name.as_str()) {
                    if seen_locals.insert(name.as_str()) {
                        stack.push((*local, false));
                    }
                    continue;
                }
                match namespace.get(name) {
                    Some(Decl::Input(_) | Decl::State(_)) => return Ok(true),
                    Some(Decl::Let(id)) => {
                        if !self.hir.is_prelude_let(id) && seen_lets.insert(id) {
                            stack.push((self.hir.let_decl(id)?.value, true));
                        }
                    }
                    Some(_) | None => {}
                }
                continue;
            }
            stack.extend(kind.children().into_iter().map(|child| (child, top_level)));
        }
        Ok(false)
    }

    fn settled_variable(&self, state: StateId) -> bool {
        let Some(BoundVal::Scalar(Bound::Cell(binding))) = self.memo.states.get(&state) else {
            return false;
        };
        self.memo.settled.contains(binding)
            && self
                .memo
                .binding_layouts
                .get(binding)
                .is_some_and(|layout| layout.variable)
    }
}

fn annotated_capacity(ty: Option<&TypeRef>) -> bool {
    ty.is_some_and(|ty| ty.capacity.is_some())
}

fn measures_length(
    callee: &ExprKind,
    params: &BTreeSet<&str>,
    lets: &BTreeMap<&str, ExprId>,
    namespace: &Namespace,
) -> bool {
    let ExprKind::Name(name) = callee else {
        return false;
    };
    !params.contains(name.as_str())
        && !lets.contains_key(name.as_str())
        && matches!(
            namespace.get(name),
            Some(Decl::Builtin(BuiltinName::Function(Builtin::Len)))
        )
}

pub(super) fn holds_cell(bound: &BoundVal) -> bool {
    match bound {
        BoundVal::Scalar(Bound::Cell(_)) => true,
        BoundVal::Array(array) => array.1.iter().any(|item| matches!(item, Bound::Cell(_))),
        BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error => false,
    }
}
