use std::collections::HashMap;

use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::op::Op;

impl Engine<'_> {
    pub(super) fn reads_of(&mut self, root: NodeId) -> Result<Vec<BindingId>, Internal> {
        self.mark_reaching_reads()?;
        let mut found = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !self.reaches_read(node) || !seen.insert(node) {
                continue;
            }
            self.memo.read_walk += 1;
            match &self.node(node)?.op {
                Op::Read(binding) => found.push(*binding),
                op => stack.extend(op.operands()),
            }
        }
        Ok(found)
    }

    pub(super) fn reaches_read(&self, node: NodeId) -> bool {
        self.memo
            .reaching_reads
            .get(node.index())
            .copied()
            .unwrap_or(true)
    }

    fn mark_reaching_reads(&mut self) -> Result<(), Internal> {
        let marked = self.memo.reaching_reads.len();
        let mut known: FastMap<NodeId, bool> = FastMap::default();
        for index in marked..self.nodes.len() {
            let root = NodeId(u32::try_from(index).map_err(|_| Internal::missed("node"))?);
            let mut stack = vec![(root, false)];
            while let Some((node, ready)) = stack.pop() {
                if node.index() < marked || known.contains_key(&node) {
                    continue;
                }
                let op = &self.node(node)?.op;
                if matches!(op, Op::Read(_)) {
                    known.insert(node, true);
                    continue;
                }
                let operands = op.operands();
                if !ready {
                    stack.push((node, true));
                    stack.extend(operands.into_iter().map(|operand| (operand, false)));
                    continue;
                }
                let reaches = operands.iter().any(|operand| {
                    self.memo
                        .reaching_reads
                        .get(operand.index())
                        .copied()
                        .or_else(|| known.get(operand).copied())
                        .unwrap_or(false)
                });
                known.insert(node, reaches);
            }
        }
        for index in marked..self.nodes.len() {
            let node = NodeId(u32::try_from(index).map_err(|_| Internal::missed("node"))?);
            let reaches = known.get(&node).copied().ok_or(Internal::missed("node"))?;
            self.memo.reaching_reads.push(reaches);
        }
        Ok(())
    }

    pub(super) fn binding_order(&mut self) -> Result<Vec<BindingId>, Internal> {
        let count = self.bindings.len();
        let mut reads: Vec<Vec<BindingId>> = Vec::with_capacity(count);
        let mut readers: HashMap<BindingId, usize> = HashMap::new();
        for index in 0..count {
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            let mut targets = self.reads_of(self.binding(binding)?.init)?;
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
            let targets = reads
                .get(binding.index())
                .ok_or(Internal::missed("binding"))?;
            for target in targets {
                let remaining = readers.get_mut(target).ok_or(Internal::missed("binding"))?;
                *remaining -= 1;
                if *remaining == 0 {
                    ready.push(*target);
                }
            }
        }
        Ok(order)
    }
}
