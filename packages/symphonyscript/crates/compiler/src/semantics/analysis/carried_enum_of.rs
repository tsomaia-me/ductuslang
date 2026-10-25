use symphonyscript_core::{EnumId, Scalar};

use super::engine::Engine;
use super::ids::NodeId;
use super::op::Op;
use crate::semantics::Internal;

impl Engine<'_> {
    pub(super) fn carried_enum_of(&self, node: NodeId) -> Result<Option<EnumId>, Internal> {
        let source = self.through_uses(node)?;
        Ok(match self.node(source)?.op {
            Op::Read(binding) => match self.typing.own.get(&binding) {
                Some(Scalar::Enum(id)) => Some(*id),
                Some(_) | None => None,
            },
            _ => self.memo.bare_enums.get(&source).copied(),
        })
    }
}
