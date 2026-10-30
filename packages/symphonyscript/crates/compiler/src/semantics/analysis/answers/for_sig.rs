use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::for_node::ForNode;
use crate::semantics::analysis::ids::{NodeId, SlotId};
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::signature::branch_width;
use crate::semantics::analysis::unify_kinds::unify_kinds;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn for_sig(
        &mut self,
        node: NodeId,
        for_node: &ForNode,
        sigs: &[Sig],
    ) -> Result<Sig, Internal> {
        let reactive = sigs.iter().any(|sig| sig.reactive);
        let accumulator = self.slot(for_node.accumulator)?.clone();
        if accumulator.annotated {
            return Ok(Sig {
                reactive,
                frame: false,
                open: false,
                ..accumulator.sig
            });
        }
        let (Some(start), Some(body)) = (sigs.iter().rev().nth(1), sigs.last()) else {
            return Err(Internal::missed("operand signature"));
        };
        let Some(kind) = unify_kinds(start.kind, body.kind) else {
            self.error_at_node(Code::TypeMismatch, node, START_AND_BODY)?;
            return Ok(Sig::ERROR);
        };
        let Some(family) =
            self.one_family(node, &[(for_node.start, *start), (for_node.body, *body)])?
        else {
            return Ok(Sig::ERROR);
        };
        let width = branch_width(kind, start, body);
        Ok(Sig::of(kind, width, reactive, family))
    }

    pub(in crate::semantics::analysis) fn reaches_outer_frame(
        &self,
        for_node: &ForNode,
    ) -> Result<bool, Internal> {
        let mut own: Vec<SlotId> = [for_node.index, for_node.element, Some(for_node.accumulator)]
            .into_iter()
            .flatten()
            .collect();
        let mut seen: FastSet<NodeId> = FastSet::default();
        let mut stack = for_node.operands();
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::ForSigOuterFrame.step();
            if !seen.insert(node) {
                continue;
            }
            if !self.memo.sigs.get(&node).is_some_and(|sig| sig.frame) {
                continue;
            }
            let op = &self.node(node)?.op;
            if let Op::For(inner) = op {
                own.extend(
                    [inner.index, inner.element, Some(inner.accumulator)]
                        .into_iter()
                        .flatten(),
                );
            }
            if matches!(op, Op::Arg(_)) {
                return Ok(true);
            }
            if let Op::Slot(slot) = op
                && !own.contains(slot)
            {
                return Ok(true);
            }
            stack.extend(op.operands());
        }
        Ok(false)
    }
}

const START_AND_BODY: &str =
    "the start and the body of a `for` expression hold values of different types";
