use super::engine::Engine;
use super::fast_hash::FastSet;
use super::ids::NodeId;
use super::kind::Kind;
use super::op::Op;
use crate::semantics::Internal;

impl Engine<'_> {
    pub(super) fn untypable_values(&self, node: NodeId) -> Result<Vec<NodeId>, Internal> {
        let mut found = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::UntypableValuesUntypableValues.step();
            if !seen.insert(current) {
                continue;
            }
            let op = &self.node(current)?.op;
            if let Op::Bare(_) = op {
                found.push(current);
                continue;
            }
            if let Op::Use(inner) = op {
                let target = self.through_uses(*inner)?;
                if self
                    .memo
                    .sigs
                    .get(&target)
                    .is_some_and(|sig| !sig.reactive && sig.kind == Kind::Bare)
                {
                    found.push(target);
                }
                continue;
            }
            stack.extend(op.operands());
        }
        Ok(found)
    }
}
