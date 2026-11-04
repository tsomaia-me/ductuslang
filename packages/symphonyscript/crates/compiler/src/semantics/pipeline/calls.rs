use std::collections::HashMap;

use super::body_instance::BodyInstance;
use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::loops::loop_body;
use super::need::Need;
use super::reason::Reason;
use crate::diagnostics::{Code, Related};
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{NONE, Op};

const NEW: u8 = 0;
const OPEN: u8 = 1;
const DONE: u8 = 2;

impl Core<'_> {
    pub fn functions(&self) -> u32 {
        u32::try_from(self.ir.fns.len()).unwrap_or(0)
    }

    pub fn owners(&self) -> u32 {
        self.functions()
            + u32::try_from(self.ir.fors.len()).unwrap_or(0)
            + u32::try_from(self.loops.len()).unwrap_or(0)
    }

    pub fn function_roots(&self, owner: u32) -> Result<Vec<u32>, Internal> {
        if self.loop_at(owner).is_some() {
            let rec = self.loop_rec(owner)?;
            return Ok(loop_body(self.ir, rec.body)?.0);
        }
        let (lets, value) = if owner < self.functions() {
            let rec = self
                .ir
                .fns
                .get(owner as usize)
                .ok_or(Internal::missed("IR function"))?;
            (rec.lets, rec.value)
        } else {
            let rec = self
                .ir
                .fors
                .get((owner - self.functions()) as usize)
                .ok_or(Internal::missed("IR for"))?;
            (rec.lets, rec.body)
        };
        let mut roots = Vec::new();
        for at in lets.0..lets.1 {
            roots.push(self.let_root(at)?);
        }
        roots.push(value);
        Ok(roots)
    }

    pub fn mark_bodies(&mut self) -> Result<(), Internal> {
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) == Some(&Op::For)
                && let Some(record) = self.ir.a.get(local).copied()
                && let Some(slot) = self.fold_nodes.get_mut(record as usize)
            {
                *slot = self.ir.base + u32::try_from(local).unwrap_or(0);
            }
        }
        for function in 0..self.owners() {
            for root in self.function_roots(function)? {
                for node in self.postorder(root)? {
                    let mut current = node;
                    loop {
                        let local = self.local(current)?;
                        if let Some(slot) = self.body_of.get_mut(local) {
                            *slot = function;
                        }
                        let (op, receiver, _, _) = self.ir.node(current)?;
                        if op != Op::Field {
                            break;
                        }
                        current = receiver;
                    }
                }
            }
        }
        Ok(())
    }

    fn binder_sym(&self, binder: u32) -> Result<u32, Internal> {
        let name = self.ir.binder(binder)?.name;
        Ok(match self.ir.name(name) {
            Some(entry) if !entry.faulty => entry.sym.0,
            Some(_) | None => u32::MAX,
        })
    }

    pub fn scope_of(&self, owner: u32) -> Result<Vec<(u32, DeclRef)>, Internal> {
        let mut scope = Vec::new();
        let (params, lets) = if self.loop_at(owner).is_some() {
            let head = self.loop_rec(owner)?.head;
            ((head.names.0..head.names.1).collect(), (0, 0))
        } else if owner < self.functions() {
            let rec = self
                .ir
                .fns
                .get(owner as usize)
                .ok_or(Internal::missed("IR function"))?;
            let params: Vec<u32> = (rec.params.0..rec.params.1).collect();
            (params, rec.lets)
        } else {
            let rec = self
                .ir
                .fors
                .get((owner - self.functions()) as usize)
                .ok_or(Internal::missed("IR for"))?;
            let mut params: Vec<u32> = (rec.names.0..rec.names.1).collect();
            params.push(rec.accumulator);
            (params, rec.lets)
        };
        for (position, binder) in params.into_iter().enumerate() {
            scope.push((
                self.binder_sym(binder)?,
                DeclRef::Param(u32::try_from(position).unwrap_or(0)),
            ));
        }
        for at in lets.0..lets.1 {
            let binder = self
                .ir
                .lets
                .get(at as usize)
                .ok_or(Internal::missed("IR let"))?
                .binder;
            scope.push((self.binder_sym(binder)?, DeclRef::Let(at)));
        }
        Ok(scope)
    }

    pub fn scoped(&self, node: u32, sym: u32) -> Result<Option<DeclRef>, Internal> {
        let owner = self.body_of.get(self.local(node)?).copied().unwrap_or(NONE);
        if owner == NONE {
            return Ok(None);
        }
        Ok(self
            .scope_of(owner)?
            .into_iter()
            .find(|(bound, _)| *bound == sym)
            .map(|(_, decl)| decl))
    }

    pub fn enclosing_decl(
        &mut self,
        node: u32,
        sym: u32,
        memo: &mut HashMap<(u32, u32), Option<(u32, DeclRef)>>,
    ) -> Result<Option<(u32, DeclRef)>, Internal> {
        let mut owner = self.body_of.get(self.local(node)?).copied().unwrap_or(NONE);
        let mut walked = Vec::new();
        let mut crossed = Vec::new();
        let found = loop {
            if owner == NONE || owner < self.functions() {
                break None;
            }
            if let Some(hit) = memo.get(&(owner, sym)) {
                break *hit;
            }
            walked.push(owner);
            if self.anchor(owner) == NONE {
                break None;
            }
            if self.is_fold(owner) {
                crossed.push(owner - self.functions());
            }
            let parent = self.parent_owner(owner)?;
            if parent == NONE {
                break None;
            }
            let decl = self
                .scope_of(parent)?
                .into_iter()
                .find(|(bound, _)| *bound == sym)
                .map(|(_, decl)| decl);
            if let Some(decl) = decl {
                break Some((parent, decl));
            }
            owner = parent;
        };
        if found.is_some() {
            for record in crossed {
                if let Some(slot) = self.reaches_out.get_mut(record as usize) {
                    *slot = true;
                }
            }
        }
        for owner in walked {
            memo.insert((owner, sym), found);
        }
        Ok(found)
    }

    pub fn arguments(&self, node: u32) -> Result<Vec<u32>, Internal> {
        let (op, _, start, shape) = self.ir.node(node)?;
        if op != Op::NamedCall {
            return (start..start + shape / 2).map(|at| self.kid(at)).collect();
        }
        if let Some(ordered) = self.named_order(node)? {
            return Ok(ordered);
        }
        Ok(self
            .named_arguments(node)?
            .into_iter()
            .map(|(_, value)| value)
            .collect())
    }

    pub fn named_arguments(&self, node: u32) -> Result<Vec<(u32, u32)>, Internal> {
        let (_, _, start, shape) = self.ir.node(node)?;
        let count = shape / 2;
        let mut named = Vec::new();
        for at in 0..count {
            named.push((self.kid(start + 2 * at)?, self.kid(start + 2 * at + 1)?));
        }
        Ok(named)
    }

    pub fn param_syms(&self, function: u32) -> Result<Vec<u32>, Internal> {
        let rec = self
            .ir
            .fns
            .get(function as usize)
            .ok_or(Internal::missed("IR function"))?;
        (rec.params.0..rec.params.1)
            .map(|binder| self.binder_sym(binder))
            .collect()
    }

    fn named_order(&self, node: u32) -> Result<Option<Vec<u32>>, Internal> {
        let Some(function) = self.callee(node)? else {
            return Ok(None);
        };
        let params = self.param_syms(function)?;
        let named = self.named_arguments(node)?;
        if named.len() != params.len() {
            return Ok(None);
        }
        let mut ordered = Vec::new();
        for param in params {
            let mut found = None;
            for (name, value) in &named {
                let sym = self
                    .ir
                    .name(*name)
                    .ok_or(Internal::missed("IR name"))?
                    .sym
                    .0;
                if sym == param {
                    if found.is_some() {
                        return Ok(None);
                    }
                    found = Some(*value);
                }
            }
            let Some(value) = found else {
                return Ok(None);
            };
            ordered.push(value);
        }
        Ok(Some(ordered))
    }

    pub fn callee(&self, node: u32) -> Result<Option<u32>, Internal> {
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
        let mut nodes = self.postorder(root)?;
        let mut cursor = 0;
        while let Some(node) = nodes.get(cursor).copied() {
            cursor += 1;
            let (op, record, _, _) = self.ir.node(node)?;
            if op == Op::For {
                for fold_root in self.function_roots(functions + record)? {
                    nodes.extend(self.postorder(fold_root)?);
                }
            }
            if matches!(op, Op::Call | Op::NamedCall)
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
            if op == Op::Field {
                let lets = u32::try_from(self.ir.lets.len()).unwrap_or(0);
                for read in self.node_reads(node)? {
                    out.push(functions + lets + read);
                }
            }
        }
        Ok(out)
    }

    pub fn recursive(&self) -> Result<bool, Internal> {
        let functions = u32::try_from(self.ir.fns.len()).unwrap_or(0);
        let count = self.ir.fns.len() + self.ir.lets.len() + self.attr_reads.len();
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
        let reads = self.ir.fns.len() + self.ir.lets.len();
        for (at, read) in self.attr_reads.iter().enumerate() {
            if read.root == NONE {
                continue;
            }
            let out = self.reached(read.root, functions)?;
            if let Some(slot) = edges.get_mut(reads + at) {
                *slot = out;
            }
        }
        let mut state = vec![NEW; count];
        let mut starts: Vec<usize> = Vec::new();
        for (at, rec) in self.ir.lets.iter().enumerate() {
            if rec.owner == NONE {
                starts.push(functions as usize + at);
            }
        }
        starts.extend((0..self.attr_reads.len()).map(|at| reads + at));
        for local in 0..self.ir.op.len() {
            if !matches!(self.ir.op.get(local), Some(Op::Call | Op::NamedCall)) {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            if self.function_owner(node)? == NONE
                && let Some(callee) = self.callee(node)?
            {
                starts.push(callee as usize);
            }
        }
        for start in starts {
            if state.get(start) != Some(&NEW) {
                continue;
            }
            let mut stack = vec![(u32::try_from(start).unwrap_or(0), 0usize)];
            if let Some(slot) = state.get_mut(start) {
                *slot = OPEN;
            }
            while let Some((vertex, next)) = stack.pop() {
                CountedWalk::BindLookup.step();
                let target = edges
                    .get(vertex as usize)
                    .and_then(|out| out.get(next))
                    .copied();
                if let Some(target) = target {
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

    pub fn function_name(&self, function: u32) -> Result<String, Internal> {
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
        if self.ir.node(node)?.0 == Op::NamedCall && self.named_order(node)?.is_none() {
            self.named_faults(node, function, &name, &params)?;
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        if args.len() != params.len() && self.cut(node)? {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
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
        let key = (function, NONE, facts.clone(), reactive.clone());
        let found = self.body_index.get(&key).copied();
        let body = match found {
            Some(body) => body,
            None => {
                self.bodies.push(BodyInstance {
                    function,
                    parent: NONE,
                    args: facts,
                    reactive_args: reactive,
                    diagnostics: Vec::new(),
                });
                let body = u32::try_from(self.bodies.len() - 1).unwrap_or(NONE);
                self.body_index.insert(key, body);
                return self.pending(Need::Body(body));
            }
        };
        let key = self.inst_key(node)?;
        self.call_body.insert((key, node), body);
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

    pub fn type_body(&mut self, body: u32) -> Stage<()> {
        let function = self
            .bodies
            .get(body as usize)
            .ok_or(Internal::missed("body instance"))?
            .function;
        let outer = std::mem::replace(&mut self.inst, body);
        let drawn = std::mem::take(&mut self.diagnostics);
        self.typing_bodies += 1;
        let mut outcome = Ok(Ok(()));
        for root in self.function_roots(function)? {
            outcome = self.ensure_typed(root).map(|typed| typed.map(|_| ()));
            if !matches!(outcome, Ok(Ok(()))) {
                break;
            }
        }
        let typed = std::mem::replace(&mut self.diagnostics, drawn);
        self.typing_bodies -= 1;
        if self.typing_bodies == 0 {
            let top = std::mem::take(&mut self.top_drawn);
            self.diagnostics.extend(top);
        }
        if let Some(record) = self.bodies.get_mut(body as usize) {
            record.diagnostics.extend(typed);
        }
        self.inst = outer;
        outcome
    }

    pub fn body_of_call(&self, node: u32) -> Option<u32> {
        self.inst_key(node)
            .ok()
            .and_then(|key| self.call_body.get(&(key, node)).copied())
    }

    pub fn body_value(&self, body: u32) -> Result<u32, Internal> {
        let function = self
            .bodies
            .get(body as usize)
            .ok_or(Internal::missed("body instance"))?
            .function;
        self.function_roots(function)?
            .last()
            .copied()
            .ok_or(Internal::missed("a body value"))
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

    pub fn param_fact(&self, node: u32, position: u32) -> Result<(Fact, bool), Internal> {
        let owner = self.binding_owner(node)?;
        let body = self
            .bodies
            .get(self.owner_inst(owner) as usize)
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
            if !matches!(op, Op::Call | Op::NamedCall) {
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
    pub fn inline_charges(&self) -> Result<Vec<(u32, u32, u64)>, Internal> {
        let count = self.ir.fns.len();
        let mut inner: Vec<Option<u64>> = vec![None; count];
        let mut calls_of: Vec<Vec<u32>> = vec![Vec::new(); count];
        for local in 0..self.ir.op.len() {
            if !matches!(self.ir.op.get(local), Some(Op::Call | Op::NamedCall)) {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            let function = self.function_owner(node)?;
            if function != NONE
                && let Some(callee) = self.callee(node)?
                && let Some(out) = calls_of.get_mut(function as usize)
            {
                out.push(callee);
            }
        }
        let mut opened = vec![false; count];
        for start in 0..count {
            let mut stack = vec![u32::try_from(start).unwrap_or(0)];
            while let Some(function) = stack.last().copied() {
                if inner.get(function as usize).is_some_and(Option::is_some) {
                    stack.pop();
                    continue;
                }
                if let Some(slot) = opened.get_mut(function as usize) {
                    *slot = true;
                }
                let callees = calls_of.get(function as usize).cloned().unwrap_or_default();
                let pending: Vec<u32> = callees
                    .iter()
                    .copied()
                    .filter(|callee| {
                        inner.get(*callee as usize).is_some_and(Option::is_none)
                            && !opened.get(*callee as usize).copied().unwrap_or(true)
                    })
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
        let mut charges = Vec::new();
        for local in 0..self.ir.op.len() {
            if !matches!(self.ir.op.get(local), Some(Op::Call | Op::NamedCall)) {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            if self.function_owner(node)? != NONE {
                continue;
            }
            if let Some(callee) = self.callee(node)? {
                let below = inner.get(callee as usize).copied().flatten().unwrap_or(0);
                let (lo, _) = self.ir.span(node)?;
                charges.push((
                    (self.file_of(node), lo),
                    (node, callee, below.saturating_add(1)),
                ));
            }
        }
        charges.sort_by_key(|(order, _)| *order);
        Ok(charges.into_iter().map(|(_, charge)| charge).collect())
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
        let mut callees = std::collections::HashSet::new();
        for local in 0..self.ir.op.len() {
            let reached = self.reached.get(local).copied().unwrap_or(false);
            if !reached || !matches!(self.ir.op.get(local), Some(Op::Call | Op::NamedCall)) {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            if self.callee(node)?.is_none() {
                return Ok(true);
            }
            callees.insert(self.ir.node(node)?.1);
        }
        for local in 0..self.ir.op.len() {
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            if matches!(self.bind.get(local), Some(DeclRef::Fn(_))) && !callees.contains(&node) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Core<'_> {
    fn named_faults(
        &mut self,
        node: u32,
        function: u32,
        name: &str,
        params: &[String],
    ) -> Result<(), Internal> {
        let syms = self.param_syms(function)?;
        let file = self.file_of(node);
        let mut seen: Vec<u32> = Vec::new();
        let mut faulty = false;
        for (arg, _) in self.named_arguments(node)? {
            let entry = *self.ir.name(arg).ok_or(Internal::missed("IR name"))?;
            let text = self.ir.symbols.text(entry.sym).to_string();
            let span = self.span(file, entry.lo, entry.hi);
            if !syms.contains(&entry.sym.0) {
                let declared: Vec<String> =
                    params.iter().map(|param| format!("`{param}`")).collect();
                let diagnostic = crate::diagnostics::Diagnostic::new(
                    Code::UnknownParameter,
                    span,
                    format!("the function has no parameter `{text}`"),
                )
                .with_help(format!(
                    "name a parameter `{name}` declares: {}",
                    declared.join(", ")
                ));
                let diagnostic = self.chained(diagnostic);
                self.diagnostics.push(diagnostic);
                faulty = true;
            } else if seen.contains(&entry.sym.0) {
                let diagnostic = crate::diagnostics::Diagnostic::new(
                    Code::DuplicateArgument,
                    span,
                    format!("the parameter `{text}` is supplied twice"),
                )
                .with_help("supply the parameter once");
                let diagnostic = self.chained(diagnostic);
                self.diagnostics.push(diagnostic);
                faulty = true;
            }
            seen.push(entry.sym.0);
        }
        if faulty || self.cut(node)? {
            return Ok(());
        }
        let span = self.node_span(node)?;
        for (sym, param) in syms.iter().zip(params) {
            if seen.contains(sym) {
                continue;
            }
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
        Ok(())
    }
}

impl Core<'_> {
    pub fn unread_named_faults(&mut self) -> Result<(), Internal> {
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::NamedCall) {
                continue;
            }
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            let Some(function) = self.callee(node)? else {
                continue;
            };
            let syms = self.param_syms(function)?;
            let params = self.param_names(function)?;
            let name = self.function_name(function)?;
            let file = self.file_of(node);
            for (arg, _) in self.named_arguments(node)? {
                let entry = *self.ir.name(arg).ok_or(Internal::missed("IR name"))?;
                if syms.contains(&entry.sym.0) {
                    continue;
                }
                let span = self.span(file, entry.lo, entry.hi);
                if self
                    .diagnostics
                    .iter()
                    .any(|seen| seen.code == Code::UnknownParameter && seen.span == span)
                {
                    continue;
                }
                let text = self.ir.symbols.text(entry.sym).to_string();
                let declared: Vec<String> =
                    params.iter().map(|param| format!("`{param}`")).collect();
                let diagnostic = crate::diagnostics::Diagnostic::new(
                    Code::UnknownParameter,
                    span,
                    format!("the function has no parameter `{text}`"),
                )
                .with_help(format!(
                    "name a parameter `{name}` declares: {}",
                    declared.join(", ")
                ));
                self.diagnostics.push(diagnostic);
            }
        }
        Ok(())
    }
}

impl Core<'_> {
    pub fn cut(&self, node: u32) -> Result<bool, Internal> {
        Ok(self.ir.node(node)?.3 % 2 == 1)
    }
}
