use symphonyscript_core::Span;

use super::core::Core;
use super::decl_ref::DeclRef;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{NONE, Op, TopRec};

impl Core<'_> {
    fn top_lets(&self) -> Vec<u32> {
        self.ir
            .top
            .iter()
            .filter_map(|top| match top {
                TopRec::Let(at) => Some(*at),
                TopRec::Cell(_)
                | TopRec::Fn(_)
                | TopRec::Pattern(_)
                | TopRec::Placement(_)
                | TopRec::Placeholder { .. }
                | TopRec::Drawn(_)
                | TopRec::Skip => None,
            })
            .collect()
    }

    fn declaration_reads(
        &self,
        root: u32,
        env: u32,
        (lets, cells, fns): (u32, u32, u32),
    ) -> Result<Vec<u32>, Internal> {
        let mut reads = Vec::new();
        let mut stack = vec![(root, env)];
        let mut seen = std::collections::HashSet::new();
        while let Some((root, env)) = stack.pop() {
            if !seen.insert((root, env)) {
                continue;
            }
            for node in self.deep_postorder(root)? {
                if self.unbound_nodes.contains(&node) {
                    continue;
                }
                let op = self.ir.node(node)?.0;
                if matches!(op, Op::Call | Op::NamedCall)
                    && let Some(function) = self.callee(node)?
                {
                    reads.push(fns + function);
                }
                if op == Op::Name && !self.is_history_target(node) {
                    match self.bound(node)? {
                        DeclRef::Let(read) => reads.push(read),
                        DeclRef::Cell(read) => reads.push(cells + read),
                        DeclRef::Param(position) if env != NONE => {
                            stack.extend(self.argument_of(node, position, env)?);
                        }
                        DeclRef::Param(_)
                        | DeclRef::None
                        | DeclRef::Instance(_)
                        | DeclRef::Duplicate
                        | DeclRef::Fn(_)
                        | DeclRef::Pattern(_) => {}
                    }
                } else if op == Op::Field
                    && let Some(cell) = self.cell_sizes.get(&node)
                {
                    reads.push(cells + cell);
                } else if op == Op::Field {
                    for read in self.node_reads(node)? {
                        reads.push(lets + read);
                    }
                }
            }
        }
        Ok(reads)
    }

    fn declaration_key(
        &self,
        member: u32,
        (lets, cells, fns): (u32, u32, u32),
    ) -> Result<(u32, u32), Internal> {
        if let Some(at) = member.checked_sub(fns) {
            let rec = self
                .ir
                .fns
                .get(at as usize)
                .ok_or(Internal::missed("IR function"))?;
            return Ok((self.file_of(rec.value), rec.lo));
        }
        if let Some(at) = member.checked_sub(cells) {
            let rec = self
                .ir
                .cells
                .get(at as usize)
                .ok_or(Internal::missed("IR cell"))?;
            return Ok((self.file_of(self.cell_root(at)?), rec.lo));
        }
        if let Some(read) = member
            .checked_sub(lets)
            .and_then(|at| self.attr_reads.get(at as usize))
        {
            return Ok(read.key);
        }
        let rec = self
            .ir
            .lets
            .get(member as usize)
            .ok_or(Internal::missed("IR let"))?;
        Ok((
            self.let_file.get(member as usize).copied().unwrap_or(0),
            rec.lo,
        ))
    }

    pub fn cell_name(&self, at: u32) -> Result<(String, Span), Internal> {
        let rec = self
            .ir
            .cells
            .get(at as usize)
            .ok_or(Internal::missed("IR cell"))?;
        let name = self.ir.binder(rec.binder)?.name;
        let entry = *self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        Ok((
            self.ir.symbols.text(entry.sym).to_string(),
            self.span(self.file_of(self.cell_root(at)?), entry.lo, entry.hi),
        ))
    }

    pub fn declaration_cycles(&mut self) -> Result<(), Internal> {
        for local in 0..self.ir.op.len() {
            let node = self.ir.base + u32::try_from(local).unwrap_or(u32::MAX);
            for argument in self.unbound_arguments(node)? {
                let inner = self.deep_postorder(argument)?;
                self.unbound_nodes.extend(inner);
            }
        }
        let lets = u32::try_from(self.ir.lets.len()).unwrap_or(u32::MAX);
        let cells = u32::try_from(self.ir.lets.len() + self.attr_reads.len()).unwrap_or(u32::MAX);
        let fns = cells + u32::try_from(self.ir.cells.len()).unwrap_or(u32::MAX);
        let count =
            self.ir.lets.len() + self.attr_reads.len() + self.ir.cells.len() + self.ir.fns.len();
        let mut edges: Vec<Vec<u32>> = vec![Vec::new(); count];
        for at in self.top_lets() {
            let reads = self.declaration_reads(self.let_root(at)?, NONE, (lets, cells, fns))?;
            if let Some(slot) = edges.get_mut(at as usize) {
                *slot = reads;
            }
        }
        for at in 0..self.attr_reads.len() {
            let Some(read) = self.attr_reads.get(at).cloned() else {
                continue;
            };
            if read.root == NONE {
                continue;
            }
            let env = self.read_env(&read)?;
            let reads = self.declaration_reads(read.root, env, (lets, cells, fns))?;
            if let Some(slot) = edges.get_mut(self.ir.lets.len() + at) {
                *slot = reads;
            }
        }
        for at in 0..u32::try_from(self.ir.cells.len()).unwrap_or(0) {
            let root = if self.derived(at) {
                self.cell_root(at)?
            } else {
                self.ir.cells.get(at as usize).map_or(NONE, |rec| rec.init)
            };
            if root == NONE {
                continue;
            }
            let reads = self.declaration_reads(root, NONE, (lets, cells, fns))?;
            if let Some(slot) = edges.get_mut((cells + at) as usize) {
                *slot = reads;
            }
        }
        for at in 0..self.functions() {
            let mut reads = Vec::new();
            for root in self.function_roots(at)? {
                reads.extend(self.declaration_reads(root, NONE, (lets, cells, fns))?);
            }
            if let Some(slot) = edges.get_mut((fns + at) as usize) {
                *slot = reads;
            }
        }
        let components = strongly_connected(&edges);
        for component in &components {
            for member in component {
                if let Some(at) = member.checked_sub(lets)
                    && *member < cells
                {
                    self.attr_order.push(at);
                }
            }
        }
        for component in components {
            let mut first = None;
            for member in &component {
                let key = self.declaration_key(*member, (lets, cells, fns))?;
                if first.is_none_or(|(seen, _)| key < seen) {
                    first = Some((key, *member));
                }
            }
            let Some((_, first)) = first else {
                continue;
            };
            let looped = component.len() > 1
                || edges
                    .get(first as usize)
                    .is_some_and(|reads| reads.contains(&first));
            if !looped || component.iter().all(|member| *member >= fns) {
                continue;
            }
            for member in &component {
                if *member < lets && self.refused_call_reads(self.let_root(*member)?)? {
                    self.cycle_through_refused = true;
                }
            }
            for member in &component {
                if *member >= fns {
                    continue;
                }
                if let Some(at) = member.checked_sub(cells) {
                    if let Some(slot) = self.cell_cyclic.get_mut(at as usize) {
                        *slot = true;
                    }
                } else if let Some(read) = member
                    .checked_sub(lets)
                    .and_then(|at| self.attr_reads.get_mut(at as usize))
                {
                    read.cyclic = true;
                } else if let Some(slot) = self.cyclic.get_mut(*member as usize) {
                    *slot = true;
                }
            }
            let cycle = shortest_cycle(&edges, &component, first);
            let mut names = Vec::new();
            for member in cycle {
                if let Some(at) = member.checked_sub(fns) {
                    names.push(self.fn_name(at)?);
                    continue;
                }
                if let Some(at) = member.checked_sub(cells) {
                    names.push(self.cell_name(at)?);
                    continue;
                }
                match member
                    .checked_sub(lets)
                    .and_then(|at| self.attr_reads.get(at as usize))
                {
                    Some(read) => names.push((read.label.clone(), read.span.clone())),
                    None => names.push(self.let_name(member)?),
                }
            }
            self.draw_named_cycle(names);
        }
        Ok(())
    }

    fn let_name(&self, at: u32) -> Result<(String, Span), Internal> {
        let rec = self
            .ir
            .lets
            .get(at as usize)
            .ok_or(Internal::missed("IR let"))?;
        let binder = *self.ir.binder(rec.binder)?;
        let entry = *self
            .ir
            .name(binder.name)
            .ok_or(Internal::missed("IR name"))?;
        let file = if rec.owner == NONE {
            self.let_file.get(at as usize).copied().unwrap_or(0)
        } else {
            self.file_of(rec.value)
        };
        Ok((
            self.ir.symbols.text(entry.sym).to_string(),
            self.span(file, entry.lo, entry.hi),
        ))
    }

    pub(super) fn draw_cycle(&mut self, cycle: &[u32]) -> Result<(), Internal> {
        let mut names = Vec::new();
        for at in cycle {
            names.push(self.let_name(*at)?);
        }
        self.draw_named_cycle(names);
        Ok(())
    }

    fn draw_named_cycle(&mut self, names: Vec<(String, Span)>) {
        let Some((first_name, first_span)) = names.first().cloned() else {
            return;
        };
        let mut text: Vec<&str> = names.iter().map(|(name, _)| name.as_str()).collect();
        text.push(first_name.as_str());
        let summary = format!("these declarations reach themselves: {}", text.join(" → "));
        let related = names
            .iter()
            .skip(1)
            .map(|(_, span)| (span.clone(), "a declaration on the cycle".to_string()))
            .collect();
        self.draw_related(Code::DeclarationCycle, first_span, related, summary);
    }
}

pub(super) fn shortest_cycle(edges: &[Vec<u32>], component: &[u32], first: u32) -> Vec<u32> {
    let inside = |node: u32| component.contains(&node);
    let mut parent: Vec<Option<u32>> = vec![None; edges.len()];
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(first);
    let mut seen = vec![false; edges.len()];
    if let Some(slot) = seen.get_mut(first as usize) {
        *slot = true;
    }
    while let Some(node) = queue.pop_front() {
        for next in edges
            .get(node as usize)
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            if *next == first {
                let mut cycle = vec![node];
                let mut current = node;
                while let Some(Some(previous)) = parent.get(current as usize) {
                    cycle.push(*previous);
                    current = *previous;
                }
                cycle.reverse();
                return cycle;
            }
            if inside(*next) && !seen.get(*next as usize).copied().unwrap_or(true) {
                if let Some(slot) = seen.get_mut(*next as usize) {
                    *slot = true;
                }
                if let Some(slot) = parent.get_mut(*next as usize) {
                    *slot = Some(node);
                }
                queue.push_back(*next);
            }
        }
    }
    vec![first]
}

pub(super) fn strongly_connected(edges: &[Vec<u32>]) -> Vec<Vec<u32>> {
    let count = edges.len();
    let mut index = vec![u32::MAX; count];
    let mut low = vec![0u32; count];
    let mut on_stack = vec![false; count];
    let mut stack: Vec<u32> = Vec::new();
    let mut components = Vec::new();
    let mut next = 0u32;
    for start in 0..count {
        if index.get(start).copied() != Some(u32::MAX) {
            continue;
        }
        let mut work: Vec<(u32, usize)> = vec![(u32::try_from(start).unwrap_or(0), 0)];
        while let Some((node, child)) = work.pop() {
            let at = node as usize;
            if child == 0 {
                if let Some(slot) = index.get_mut(at) {
                    *slot = next;
                }
                if let Some(slot) = low.get_mut(at) {
                    *slot = next;
                }
                next += 1;
                stack.push(node);
                if let Some(slot) = on_stack.get_mut(at) {
                    *slot = true;
                }
            }
            let reads = edges.get(at).map(Vec::as_slice).unwrap_or_default();
            if let Some(&target) = reads.get(child) {
                work.push((node, child + 1));
                let target_at = target as usize;
                if index.get(target_at).copied() == Some(u32::MAX) {
                    work.push((target, 0));
                } else if on_stack.get(target_at).copied().unwrap_or(false) {
                    let target_index = index.get(target_at).copied().unwrap_or(0);
                    if let Some(slot) = low.get_mut(at) {
                        *slot = (*slot).min(target_index);
                    }
                }
                continue;
            }
            if let Some((parent, _)) = work.last() {
                let node_low = low.get(at).copied().unwrap_or(0);
                if let Some(slot) = low.get_mut(*parent as usize) {
                    *slot = (*slot).min(node_low);
                }
            }
            if low.get(at) == index.get(at) {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    if let Some(slot) = on_stack.get_mut(member as usize) {
                        *slot = false;
                    }
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                components.push(component);
            }
        }
    }
    components
}

impl Core<'_> {
    fn fn_name(&self, at: u32) -> Result<(String, Span), Internal> {
        let rec = self
            .ir
            .fns
            .get(at as usize)
            .ok_or(Internal::missed("IR function"))?;
        let entry = *self.ir.name(rec.name).ok_or(Internal::missed("IR name"))?;
        Ok((
            self.ir.symbols.text(entry.sym).to_string(),
            self.span(self.file_of(rec.value), entry.lo, entry.hi),
        ))
    }
}

impl Core<'_> {
    pub fn check_cyclic_roots(&mut self) -> super::core::Stage<()> {
        for top in self.ir.top.clone() {
            let roots = match top {
                TopRec::Let(at) if self.cyclic.get(at as usize).copied().unwrap_or(false) => {
                    vec![self.let_root(at)?]
                }
                TopRec::Cell(at) if self.is_cyclic_cell(at) => {
                    let rec = self
                        .ir
                        .cells
                        .get(at as usize)
                        .ok_or(Internal::missed("IR cell"))?;
                    [rec.rule, rec.init]
                        .into_iter()
                        .filter(|root| *root != NONE)
                        .collect()
                }
                TopRec::Let(_)
                | TopRec::Cell(_)
                | TopRec::Fn(_)
                | TopRec::Pattern(_)
                | TopRec::Placement(_)
                | TopRec::Placeholder { .. }
                | TopRec::Drawn(_)
                | TopRec::Skip => continue,
            };
            for root in roots {
                if let Err(declined) = self.ensure_typed(root)? {
                    return Ok(Err(declined));
                }
                if let Err(declined) = self.own_check(root)? {
                    return Ok(Err(declined));
                }
            }
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    fn refused_call_reads(&self, root: u32) -> Result<bool, Internal> {
        for node in self.deep_postorder(root)? {
            if !matches!(self.ir.node(node)?.0, Op::Call | Op::NamedCall) {
                continue;
            }
            let Some(function) = self.callee(node)? else {
                continue;
            };
            let Some(rec) = self.ir.fns.get(function as usize) else {
                continue;
            };
            let arguments = self.arguments(node)?;
            if arguments.len() == (rec.params.1 - rec.params.0) as usize {
                continue;
            }
            for argument in arguments {
                for inner in self.deep_postorder(argument)? {
                    if matches!(self.ir.node(inner)?.0, Op::Name | Op::Field | Op::Call) {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
}

impl Core<'_> {
    pub fn unbound_arguments(&self, node: u32) -> Result<Vec<u32>, Internal> {
        let op = self.ir.node(node)?.0;
        if !matches!(op, Op::Call | Op::NamedCall) {
            return Ok(Vec::new());
        }
        let Some(function) = self.callee(node)? else {
            if !self.not_callable(node)? && self.math_named(node)?.is_none() {
                return Ok(Vec::new());
            }
            if op == Op::Call {
                return self.arguments(node);
            }
            return Ok(self
                .named_arguments(node)?
                .into_iter()
                .map(|(_, value)| value)
                .collect());
        };
        let params = self.param_syms(function)?;
        if op == Op::Call {
            return Ok(self
                .arguments(node)?
                .into_iter()
                .skip(params.len())
                .collect());
        }
        let mut unbound = Vec::new();
        for (name, value) in self.named_arguments(node)? {
            let sym = self.ir.name(name).ok_or(Internal::missed("IR name"))?.sym.0;
            if !params.contains(&sym) {
                unbound.push(value);
            }
        }
        Ok(unbound)
    }
}
