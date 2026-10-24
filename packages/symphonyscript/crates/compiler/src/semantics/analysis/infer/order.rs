use std::collections::{HashMap, HashSet};

use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::op::Op;

impl Engine<'_> {
    pub(super) fn reads_of(&self, root: NodeId) -> Vec<BindingId> {
        let mut found = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            match &self.node(node).op {
                Op::Read(binding) => found.push(*binding),
                op => stack.extend(op.operands()),
            }
        }
        found
    }

    pub(super) fn binding_order(&self) -> Vec<BindingId> {
        let count = self.bindings.len();
        let mut reads: Vec<Vec<BindingId>> = Vec::with_capacity(count);
        let mut readers: HashMap<BindingId, usize> = HashMap::new();
        for index in 0..count {
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            let mut targets = self.reads_of(self.binding(binding).init);
            targets.sort();
            targets.dedup();
            for target in &targets {
                *readers.entry(*target).or_insert(0) += 1;
            }
            reads.push(targets);
        }
        let mut ready: Vec<BindingId> = (0..count)
            .map(|index| BindingId(u32::try_from(index).unwrap_or(u32::MAX)))
            .filter(|binding| !readers.contains_key(binding))
            .collect();
        ready.reverse();
        let mut order = Vec::with_capacity(count);
        while let Some(binding) = ready.pop() {
            order.push(binding);
            if let Some(targets) = reads.get(binding.index()) {
                for target in targets {
                    if let Some(remaining) = readers.get_mut(target) {
                        *remaining -= 1;
                        if *remaining == 0 {
                            ready.push(*target);
                        }
                    }
                }
            }
        }
        order
    }
}
