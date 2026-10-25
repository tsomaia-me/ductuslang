use symphonyscript_core::{BinaryOp, EnumId, Scalar};

use super::position::Position;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;

impl Engine<'_> {
    pub(super) fn compile_time_enums(
        &self,
        valid: &[Position],
    ) -> Result<FastMap<NodeId, Vec<EnumId>>, Internal> {
        let mut stack: Vec<(NodeId, Option<EnumId>)> = Vec::new();
        for position in valid {
            let expected = enum_of(position.ty.scalar);
            stack.extend(
                position
                    .val
                    .nodes()
                    .into_iter()
                    .map(|node| (node, expected)),
            );
        }
        for index in 0..self.bindings.len() {
            let binding = BindingId(u32::try_from(index).map_err(|_| Internal::missed("binding"))?);
            let expected = self.typing.own.get(&binding).copied().and_then(enum_of);
            stack.push((self.binding(binding)?.init, expected));
        }
        let mut given: FastMap<NodeId, Vec<EnumId>> = FastMap::default();
        let mut seen = FastSet::default();
        while let Some((node, expected)) = stack.pop() {
            if !seen.insert((node, expected)) {
                continue;
            }
            if let Some(group) = self.memo.groups.get(&node).copied() {
                if let Some(id) = expected {
                    let enums = given.entry(group).or_default();
                    if !enums.contains(&id) {
                        enums.push(id);
                    }
                }
                stack.push((group, expected));
            }
            match self.node(node)?.op.clone() {
                Op::Use(inner) => {
                    if let Some(id) = expected {
                        let target = self.through_uses(inner)?;
                        let enums = given.entry(target).or_default();
                        if !enums.contains(&id) {
                            enums.push(id);
                        }
                    }
                    stack.push((inner, expected));
                }
                Op::Chosen(inner) => stack.push((inner, expected)),
                Op::If(condition, then, otherwise) => {
                    stack.extend([(condition, None), (then, expected), (otherwise, expected)]);
                }
                Op::Match {
                    scrutinee,
                    arms,
                    otherwise,
                } => {
                    stack.push((scrutinee, None));
                    stack.extend(arms.iter().map(|(_, value)| (*value, expected)));
                    stack.extend(otherwise.map(|value| (value, expected)));
                }
                Op::Pick { index, items } => {
                    stack.push((index, None));
                    stack.extend(items.iter().map(|item| (*item, expected)));
                }
                Op::Element { base, index } => {
                    stack.extend([(base, expected), (index, None)]);
                }
                Op::Array(elements) => {
                    stack.extend(elements.iter().map(|element| (*element, expected)));
                }
                Op::Binary(BinaryOp::Is | BinaryOp::IsNot, lhs, rhs) => {
                    let (left, right) = (self.enum_kind(lhs)?, self.enum_kind(rhs)?);
                    stack.extend([(lhs, right), (rhs, left)]);
                }
                other => stack.extend(other.operands().into_iter().map(|operand| (operand, None))),
            }
        }
        Ok(given)
    }

    fn enum_kind(&self, node: NodeId) -> Result<Option<EnumId>, Internal> {
        if let Kind::Enum(id) = self.known_sig(node)?.kind {
            return Ok(Some(id));
        }
        let source = self.through_uses(node)?;
        Ok(match self.node(source)?.op {
            Op::Read(binding) => self.typing.own.get(&binding).copied().and_then(enum_of),
            _ => self.memo.bare_enums.get(&source).copied(),
        })
    }
}

fn enum_of(scalar: Scalar) -> Option<EnumId> {
    match scalar {
        Scalar::Enum(id) => Some(id),
        Scalar::I32 | Scalar::I64 | Scalar::F32 | Scalar::F64 | Scalar::Bool | Scalar::Str => None,
    }
}
