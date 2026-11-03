use super::body_instance::BodyInstance;
use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::reason::Reason;
use crate::diagnostics::{Code, Related};
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{NONE, Op};

const NEW: u8 = 0;
const OPEN: u8 = 1;
const DONE: u8 = 2;

impl Core<'_> {
    fn function_roots(&self, function: u32) -> Result<Vec<u32>, Internal> {
        let rec = self
            .ir
            .fns
            .get(function as usize)
            .ok_or(Internal::missed("IR function"))?;
        let mut roots = Vec::new();
        for at in rec.lets.0..rec.lets.1 {
            roots.push(self.let_root(at)?);
        }
        roots.push(rec.value);
        Ok(roots)
    }

    pub fn mark_bodies(&mut self) -> Result<(), Internal> {
        for function in 0..u32::try_from(self.ir.fns.len()).unwrap_or(0) {
            for root in self.function_roots(function)? {
                for node in self.postorder(root)? {
                    let local = self.local(node)?;
                    if let Some(slot) = self.body_of.get_mut(local) {
                        *slot = function;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn scoped(&self, node: u32, sym: u32) -> Result<Option<DeclRef>, Internal> {
        let function = self.body_of.get(self.local(node)?).copied().unwrap_or(NONE);
        let Some(rec) = self.ir.fns.get(function as usize) else {
            return Ok(None);
        };
        for (position, binder) in (rec.params.0..rec.params.1).enumerate() {
            let name = self.ir.binder(binder)?.name;
            if self.ir.name(name).is_some_and(|entry| entry.sym.0 == sym) {
                return Ok(Some(DeclRef::Param(u32::try_from(position).unwrap_or(0))));
            }
        }
        for at in rec.lets.0..rec.lets.1 {
            let binder = self
                .ir
                .lets
                .get(at as usize)
                .ok_or(Internal::missed("IR let"))?
                .binder;
            let name = self.ir.binder(binder)?.name;
            if self.ir.name(name).is_some_and(|entry| entry.sym.0 == sym) {
                return Ok(Some(DeclRef::Let(at)));
            }
        }
        Ok(None)
    }

    pub fn arguments(&self, node: u32) -> Result<Vec<u32>, Internal> {
        let (_, _, start, shape) = self.ir.node(node)?;
        (start..start + shape / 2).map(|at| self.kid(at)).collect()
    }

    fn callee(&self, node: u32) -> Result<Option<u32>, Internal> {
        let (_, callee, _, _) = self.ir.node(node)?;
        let (op, _, _, _) = self.ir.node(callee)?;
        if op != Op::Name {
            return Ok(None);
        }
        Ok(match self.bound(callee)? {
            DeclRef::Fn(function) => Some(function),
            DeclRef::None
            | DeclRef::Let(_)
            | DeclRef::Cell(_)
            | DeclRef::Instance(_)
            | DeclRef::Duplicate
            | DeclRef::Param(_) => None,
        })
    }

    fn reached(&self, root: u32, functions: u32) -> Result<Vec<u32>, Internal> {
        let mut out = Vec::new();
        for node in self.postorder(root)? {
            let (op, _, _, _) = self.ir.node(node)?;
            if op == Op::Call
                && let Some(callee) = self.callee(node)?
            {
                out.push(callee);
            }
            if op == Op::Name
                && let DeclRef::Let(at) = self.bound(node)?
                && self
                    .ir
                    .lets
                    .get(at as usize)
                    .is_some_and(|rec| rec.owner == NONE)
            {
                out.push(functions + at);
            }
        }
        Ok(out)
    }

    pub fn recursive(&self) -> Result<bool, Internal> {
        let functions = u32::try_from(self.ir.fns.len()).unwrap_or(0);
        let count = self.ir.fns.len() + self.ir.lets.len();
        let mut edges: Vec<Vec<u32>> = vec![Vec::new(); count];
        for function in 0..functions {
            let mut out = Vec::new();
            for root in self.function_roots(function)? {
                out.extend(self.reached(root, functions)?);
            }
            if let Some(slot) = edges.get_mut(function as usize) {
                *slot = out;
            }
        }
        for (at, rec) in self.ir.lets.iter().enumerate() {
            if rec.owner != NONE {
                continue;
            }
            let out = self.reached(rec.value, functions)?;
            if let Some(slot) = edges.get_mut(functions as usize + at) {
                *slot = out;
            }
        }
        let mut state = vec![NEW; count];
        for start in 0..count {
            if state.get(start) != Some(&NEW) {
                continue;
            }
            let mut stack = vec![(u32::try_from(start).unwrap_or(0), 0usize)];
            if let Some(slot) = state.get_mut(start) {
                *slot = OPEN;
            }
            while let Some((vertex, next)) = stack.pop() {
                CountedWalk::BindLookup.step();
                let out = edges.get(vertex as usize).cloned().unwrap_or_default();
                if let Some(target) = out.get(next).copied() {
                    stack.push((vertex, next + 1));
                    match state.get(target as usize).copied() {
                        Some(OPEN) => {
                            let from = stack
                                .iter()
                                .position(|(open, _)| *open == target)
                                .unwrap_or(0);
                            if stack
                                .get(from..)
                                .unwrap_or_default()
                                .iter()
                                .any(|(open, _)| *open < functions)
                            {
                                return Ok(true);
                            }
                        }
                        Some(NEW) => {
                            if let Some(slot) = state.get_mut(target as usize) {
                                *slot = OPEN;
                            }
                            stack.push((target, 0));
                        }
                        Some(_) | None => {}
                    }
                } else if let Some(slot) = state.get_mut(vertex as usize) {
                    *slot = DONE;
                }
            }
        }
        Ok(false)
    }

    fn function_name(&self, function: u32) -> Result<String, Internal> {
        let rec = self
            .ir
            .fns
            .get(function as usize)
            .ok_or(Internal::missed("IR function"))?;
        let entry = self.ir.name(rec.name).ok_or(Internal::missed("IR name"))?;
        Ok(self.ir.symbols.text(entry.sym).to_string())
    }

    fn param_names(&self, function: u32) -> Result<Vec<String>, Internal> {
        let rec = self
            .ir
            .fns
            .get(function as usize)
            .ok_or(Internal::missed("IR function"))?;
        let mut names = Vec::new();
        for binder in rec.params.0..rec.params.1 {
            let name = self.ir.binder(binder)?.name;
            let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
            names.push(self.ir.symbols.text(entry.sym).to_string());
        }
        Ok(names)
    }

    pub fn call_fact(&mut self, node: u32) -> Stage<Fact> {
        let Some(function) = self.callee(node)? else {
            return Ok(Err(Declined(Reason::FunctionValue)));
        };
        let args = self.arguments(node)?;
        let params = self.param_names(function)?;
        let name = self.function_name(function)?;
        if args.len() > params.len() {
            let count = params.len();
            self.draw_at(
                Code::ArgumentCount,
                node,
                format!(
                    "`{name}` takes at most {count} argument{}, not {}",
                    if count == 1 { "" } else { "s" },
                    args.len()
                ),
            )?;
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        if args.len() < params.len() {
            let span = self.node_span(node)?;
            for param in params.iter().skip(args.len()) {
                let diagnostic = crate::diagnostics::Diagnostic::new(
                    Code::MissingArgument,
                    span.clone(),
                    format!("the parameter `{param}` has no default and no argument"),
                )
                .with_help(format!(
                    "pass `{param}` in the call, as in `{name}({})`",
                    params.join(", ")
                ));
                let diagnostic = self.chained(diagnostic);
                self.diagnostics.push(diagnostic);
            }
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        let mut facts = Vec::new();
        let mut reactive = Vec::new();
        for arg in &args {
            facts.push(self.fact(*arg)?);
            reactive.push(self.is_reactive(*arg)?);
        }
        let found = self.bodies.iter().position(|body| {
            body.function == function && body.args == facts && body.reactive_args == reactive
        });
        let body = match found {
            Some(body) => u32::try_from(body).unwrap_or(NONE),
            None => {
                self.bodies.push(BodyInstance {
                    function,
                    args: facts,
                    reactive_args: reactive,
                    diagnostics: Vec::new(),
                });
                let body = u32::try_from(self.bodies.len() - 1).unwrap_or(NONE);
                if let Err(declined) = self.type_body(body)? {
                    return Ok(Err(declined));
                }
                body
            }
        };
        self.call_body.insert((self.inst, node), body);
        let span = self.node_span(node)?;
        let replayed: Vec<_> = self
            .bodies
            .get(body as usize)
            .map(|body| body.diagnostics.clone())
            .unwrap_or_default();
        for diagnostic in replayed {
            let mut related = diagnostic.related.clone();
            related.push(Related {
                span: span.clone(),
                label: "in this call".to_string(),
            });
            let diagnostic = diagnostic.with_related(related);
            self.push_one_per_chain(diagnostic, self.inst != NONE);
        }
        let value = self
            .ir
            .fns
            .get(function as usize)
            .ok_or(Internal::missed("IR function"))?
            .value;
        let outer = std::mem::replace(&mut self.inst, body);
        let fact = self.fact(value);
        self.inst = outer;
        Ok(Ok(fact?))
    }

    fn type_body(&mut self, body: u32) -> Stage<()> {
        let function = self
            .bodies
            .get(body as usize)
            .ok_or(Internal::missed("body instance"))?
            .function;
        let outer = std::mem::replace(&mut self.inst, body);
        let drawn = std::mem::take(&mut self.diagnostics);
        let mut outcome = Ok(Ok(()));
        for root in self.function_roots(function)? {
            outcome = self.ensure_typed(root).map(|typed| typed.map(|_| ()));
            if !matches!(outcome, Ok(Ok(()))) {
                break;
            }
        }
        let typed = std::mem::replace(&mut self.diagnostics, drawn);
        if let Some(record) = self.bodies.get_mut(body as usize) {
            record.diagnostics = typed;
        }
        self.inst = outer;
        outcome
    }

    pub fn body_of_call(&self, node: u32) -> Option<u32> {
        self.call_body.get(&(self.inst, node)).copied()
    }

    pub fn body_value(&self, body: u32) -> Result<u32, Internal> {
        let function = self
            .bodies
            .get(body as usize)
            .ok_or(Internal::missed("body instance"))?
            .function;
        Ok(self
            .ir
            .fns
            .get(function as usize)
            .ok_or(Internal::missed("IR function"))?
            .value)
    }

    pub fn call_reactive(&mut self, node: u32) -> Result<bool, Internal> {
        for arg in self.arguments(node)? {
            if self.is_reactive(arg)? {
                return Ok(true);
            }
        }
        let Some(body) = self.body_of_call(node) else {
            return Ok(false);
        };
        let value = self.body_value(body)?;
        let outer = std::mem::replace(&mut self.inst, body);
        let reactive = self.is_reactive(value);
        self.inst = outer;
        reactive
    }

    pub fn param_fact(&self, position: u32) -> Result<(Fact, bool), Internal> {
        let body = self
            .bodies
            .get(self.inst as usize)
            .ok_or(Internal::missed("body instance"))?;
        let fact = body
            .args
            .get(position as usize)
            .cloned()
            .ok_or(Internal::missed("argument fact"))?;
        let reactive = body
            .reactive_args
            .get(position as usize)
            .copied()
            .unwrap_or(false);
        Ok((fact, reactive))
    }
}

impl Core<'_> {
    pub fn draw_at_root(
        &mut self,
        code: Code,
        root: u32,
        summary: impl Into<String>,
    ) -> Result<(), Internal> {
        let outer = self.inst;
        let mut node = root;
        let mut calls = Vec::new();
        loop {
            let (op, _, _, _) = self.ir.node(node)?;
            if op != Op::Call {
                break;
            }
            let Some(body) = self.body_of_call(node) else {
                break;
            };
            calls.push(node);
            self.inst = body;
            node = self.body_value(body)?;
        }
        self.inst = outer;
        if calls.is_empty() {
            node = root;
        }
        let span = self.node_span(node)?;
        let mut related = Vec::new();
        for call in calls.iter().rev() {
            related.push((self.node_span(*call)?, "in this call".to_string()));
        }
        self.draw_related(code, span, related, summary);
        Ok(())
    }
}

impl Core<'_> {
    fn local_names(&self, function: u32) -> Result<Vec<u32>, Internal> {
        let rec = self
            .ir
            .fns
            .get(function as usize)
            .ok_or(Internal::missed("IR function"))?;
        let mut syms = Vec::new();
        for binder in rec.params.0..rec.params.1 {
            let name = self.ir.binder(binder)?.name;
            syms.push(self.ir.name(name).ok_or(Internal::missed("IR name"))?.sym.0);
        }
        for at in rec.lets.0..rec.lets.1 {
            let binder = self
                .ir
                .lets
                .get(at as usize)
                .ok_or(Internal::missed("IR let"))?
                .binder;
            let name = self.ir.binder(binder)?.name;
            syms.push(self.ir.name(name).ok_or(Internal::missed("IR name"))?.sym.0);
        }
        Ok(syms)
    }

    pub fn local_faults(&mut self) -> Result<Option<Reason>, Internal> {
        for function in 0..u32::try_from(self.ir.fns.len()).unwrap_or(0) {
            let syms = self.local_names(function)?;
            for (position, sym) in syms.iter().enumerate() {
                let text = self.ir.symbols.text(crate::semantics::ir::flat::Sym(*sym));
                let clash = syms
                    .get(..position)
                    .is_some_and(|before| before.contains(sym))
                    || self.variants.owner(text).is_some()
                    || self.vocabulary_clash(text).is_some()
                    || self
                        .decl
                        .get(*sym as usize)
                        .is_some_and(|decl| *decl != DeclRef::None);
                if clash {
                    return Ok(Some(Reason::LocalClash));
                }
            }
            let rec = *self
                .ir
                .fns
                .get(function as usize)
                .ok_or(Internal::missed("IR function"))?;
            let lets: Vec<u32> = (rec.lets.0..rec.lets.1).collect();
            let mut reads: Vec<Vec<u32>> = Vec::new();
            for at in &lets {
                let mut read = Vec::new();
                for node in self.postorder(self.let_root(*at)?)? {
                    if let DeclRef::Let(other) = self.bound(node)?
                        && lets.contains(&other)
                    {
                        read.push(other - rec.lets.0);
                    }
                }
                reads.push(read);
            }
            let mut state = vec![NEW; lets.len()];
            for start in 0..lets.len() {
                if state.get(start) != Some(&NEW) {
                    continue;
                }
                let mut stack = vec![(u32::try_from(start).unwrap_or(0), 0usize)];
                if let Some(slot) = state.get_mut(start) {
                    *slot = OPEN;
                }
                while let Some((at, next)) = stack.pop() {
                    let out = reads.get(at as usize).cloned().unwrap_or_default();
                    if let Some(other) = out.get(next).copied() {
                        stack.push((at, next + 1));
                        match state.get(other as usize).copied() {
                            Some(OPEN) => return Ok(Some(Reason::LocalCycle)),
                            Some(NEW) => {
                                if let Some(slot) = state.get_mut(other as usize) {
                                    *slot = OPEN;
                                }
                                stack.push((other, 0));
                            }
                            Some(_) | None => {}
                        }
                    } else if let Some(slot) = state.get_mut(at as usize) {
                        *slot = DONE;
                    }
                }
            }
        }
        Ok(None)
    }

    pub fn inline_count(&self) -> Result<u64, Internal> {
        let count = self.ir.fns.len();
        let mut inner: Vec<Option<u64>> = vec![None; count];
        let mut calls_of: Vec<Vec<u32>> = vec![Vec::new(); count];
        for function in 0..u32::try_from(count).unwrap_or(0) {
            for root in self.function_roots(function)? {
                for node in self.postorder(root)? {
                    let (op, _, _, _) = self.ir.node(node)?;
                    if op == Op::Call
                        && let Some(callee) = self.callee(node)?
                        && let Some(out) = calls_of.get_mut(function as usize)
                    {
                        out.push(callee);
                    }
                }
            }
        }
        for start in 0..count {
            let mut stack = vec![u32::try_from(start).unwrap_or(0)];
            while let Some(function) = stack.last().copied() {
                if inner.get(function as usize).is_some_and(Option::is_some) {
                    stack.pop();
                    continue;
                }
                let callees = calls_of.get(function as usize).cloned().unwrap_or_default();
                let pending: Vec<u32> = callees
                    .iter()
                    .copied()
                    .filter(|callee| inner.get(*callee as usize).is_some_and(Option::is_none))
                    .collect();
                if pending.is_empty() {
                    let mut total = 0u64;
                    for callee in callees {
                        let below = inner.get(callee as usize).copied().flatten().unwrap_or(0);
                        total = total.saturating_add(1).saturating_add(below);
                    }
                    if let Some(slot) = inner.get_mut(function as usize) {
                        *slot = Some(total);
                    }
                    stack.pop();
                } else {
                    stack.extend(pending);
                }
            }
        }
        let mut total = 0u64;
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Call)
                || self.body_of.get(local).copied().unwrap_or(NONE) != NONE
            {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            if let Some(callee) = self.callee(node)? {
                let below = inner.get(callee as usize).copied().flatten().unwrap_or(0);
                total = total.saturating_add(1).saturating_add(below);
            }
        }
        Ok(total)
    }
}

fn chain_key(diagnostic: &crate::diagnostics::Diagnostic) -> Vec<u32> {
    diagnostic
        .related
        .iter()
        .map(|related| related.span.range.start)
        .collect()
}

impl Core<'_> {
    pub fn push_one_per_chain(&mut self, diagnostic: crate::diagnostics::Diagnostic, inner: bool) {
        let outermost = |seen: &crate::diagnostics::Diagnostic| {
            seen.related.last().map(|related| related.span.clone())
        };
        let found = self.diagnostics.iter().position(|seen| {
            seen.code == diagnostic.code
                && seen.span == diagnostic.span
                && seen.summary == diagnostic.summary
                && (inner || outermost(seen) == outermost(&diagnostic))
                && !seen.related.is_empty()
        });
        match found {
            Some(at) => {
                if let Some(seen) = self.diagnostics.get_mut(at)
                    && chain_key(&diagnostic) < chain_key(seen)
                {
                    *seen = diagnostic;
                }
            }
            None => self.diagnostics.push(diagnostic),
        }
    }
}

impl Core<'_> {
    pub fn calls_a_non_function(&self) -> Result<bool, Internal> {
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Call) {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            if self.callee(node)?.is_none() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
