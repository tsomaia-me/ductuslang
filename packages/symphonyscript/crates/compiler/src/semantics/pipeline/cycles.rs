use super::core::Core;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::TopRec;

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

    pub fn declaration_cycles(&mut self) -> Result<(), Internal> {
        let count = self.ir.lets.len();
        let mut edges: Vec<Vec<u32>> = vec![Vec::new(); count];
        for at in self.top_lets() {
            let mut reads = Vec::new();
            for node in self.postorder(self.let_root(at)?)? {
                if let super::decl_ref::DeclRef::Let(read) = self.bound(node)? {
                    reads.push(read);
                }
            }
            if let Some(slot) = edges.get_mut(at as usize) {
                *slot = reads;
            }
        }
        let components = strongly_connected(&edges);
        for component in components {
            let Some(&first) = component.iter().min() else {
                continue;
            };
            let looped = component.len() > 1
                || edges
                    .get(first as usize)
                    .is_some_and(|reads| reads.contains(&first));
            if !looped {
                continue;
            }
            for member in &component {
                if let Some(slot) = self.cyclic.get_mut(*member as usize) {
                    *slot = true;
                }
            }
            let cycle = shortest_cycle(&edges, &component, first);
            self.draw_cycle(&cycle)?;
        }
        Ok(())
    }

    fn draw_cycle(&mut self, cycle: &[u32]) -> Result<(), Internal> {
        let mut names = Vec::new();
        for at in cycle {
            let rec = self
                .ir
                .lets
                .get(*at as usize)
                .ok_or(Internal::missed("IR let"))?;
            let binder = *self.ir.binder(rec.binder)?;
            let entry = *self
                .ir
                .name(binder.name)
                .ok_or(Internal::missed("IR name"))?;
            let file = self.let_file.get(*at as usize).copied().unwrap_or(0);
            names.push((
                self.ir.symbols.text(entry.sym).to_string(),
                self.span(file, entry.lo, entry.hi),
            ));
        }
        let Some((first_name, first_span)) = names.first().cloned() else {
            return Ok(());
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
        Ok(())
    }
}

fn shortest_cycle(edges: &[Vec<u32>], component: &[u32], first: u32) -> Vec<u32> {
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

fn strongly_connected(edges: &[Vec<u32>]) -> Vec<Vec<u32>> {
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
