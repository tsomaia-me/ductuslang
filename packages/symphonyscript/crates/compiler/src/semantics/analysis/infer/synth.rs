use symphonyscript_core::Scalar;

use crate::semantics::Internal;
use crate::semantics::analysis::answers::CellRead;
use crate::semantics::analysis::common::common;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::float_for::float_for;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::is_integer::is_integer;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::width::Width;

impl Engine<'_> {
    pub(super) fn synth(&mut self, root: NodeId) -> Result<Option<Scalar>, Internal> {
        let mut stack = vec![(root, false)];
        while let Some((node, ready)) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::SynthSynth.step();
            if self.typing.synth.contains_key(&node) {
                continue;
            }
            let sig = self.known_sig(node)?;
            if !sig.reactive {
                let ty = match sig.width {
                    Width::Fixed(scalar) => Some(scalar),
                    Width::Widened(scalar) => Some(float_for(scalar)),
                    Width::Flexible | Width::Adaptive => None,
                };
                self.typing.synth.insert(node, ty);
                continue;
            }
            let op = self.node(node)?.op.clone();
            let operands = match self.cell_read(node)? {
                CellRead::Binding(_) => Vec::new(),
                CellRead::History { fallback, .. } => fallback.into_iter().collect(),
                CellRead::Named(inner) => vec![inner],
                CellRead::Operands(operands) => operands,
            };
            if !ready {
                stack.push((node, true));
                stack.extend(
                    operands
                        .iter()
                        .filter(|operand| !self.typing.synth.contains_key(operand))
                        .map(|operand| (*operand, false)),
                );
                continue;
            }
            let ty = self.synth_of(&op, &sig)?;
            self.typing.synth.insert(node, ty);
        }
        self.synthesized(root)
    }

    pub(in crate::semantics::analysis) fn synthesized(
        &self,
        node: NodeId,
    ) -> Result<Option<Scalar>, Internal> {
        self.typing
            .synth
            .get(&node)
            .copied()
            .ok_or(Internal::missed("synthesized type"))
    }

    pub(in crate::semantics::analysis) fn known(
        &self,
        node: NodeId,
    ) -> Result<(Option<Scalar>, Kind), Internal> {
        Ok((self.synthesized(node)?, self.known_sig(node)?.kind))
    }

    pub(in crate::semantics::analysis) fn combine(
        &self,
        operands: &[NodeId],
    ) -> Result<Option<Scalar>, Internal> {
        let mut known: Option<Scalar> = None;
        let mut flexible_float = false;
        for operand in operands {
            match self.known(*operand)? {
                (Some(scalar), _) => {
                    known = Some(match known {
                        Some(current) => common(current, scalar).unwrap_or(current),
                        None => scalar,
                    });
                }
                (None, Kind::Float) => flexible_float = true,
                _ => {}
            }
        }
        Ok(match known {
            Some(scalar) if flexible_float && is_integer(scalar) => Some(float_for(scalar)),
            other => other,
        })
    }
}
