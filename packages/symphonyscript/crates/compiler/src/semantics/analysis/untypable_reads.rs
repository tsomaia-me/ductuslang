use super::engine::Engine;
use super::fast_hash::FastSet;
use super::ids::{BindingId, NodeId};
use super::kind::Kind;
use super::op::Op;
use crate::semantics::Internal;

impl Engine<'_> {
    pub(super) fn untypable_reads(&self, node: NodeId) -> Result<Vec<BindingId>, Internal> {
        let mut found = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            let op = &self.node(current)?.op;
            if let Op::Read(binding) = op {
                if self.untypable(current) {
                    found.push(*binding);
                }
                continue;
            }
            stack.extend(
                op.operands()
                    .into_iter()
                    .filter(|operand| self.untypable(*operand)),
            );
        }
        Ok(found)
    }

    fn untypable(&self, node: NodeId) -> bool {
        self.memo.sigs.get(&node).is_some_and(|sig| {
            sig.reactive && matches!(sig.kind, Kind::Bare | Kind::Str | Kind::Node)
        })
    }
}
