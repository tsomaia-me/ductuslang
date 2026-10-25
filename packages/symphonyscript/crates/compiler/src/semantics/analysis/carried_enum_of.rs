use symphonyscript_core::{EnumId, Scalar};

use super::engine::Engine;
use super::ids::NodeId;
use super::op::Op;
use crate::semantics::Internal;

impl Engine<'_> {
    pub(super) fn carried_enum_of(&self, node: NodeId) -> Result<Option<EnumId>, Internal> {
        let mut source = self.through_uses(node)?;
        while let Op::Chosen(inner) = self.node(source)?.op {
            source = self.through_uses(inner)?;
        }
        Ok(match self.node(source)?.op {
            Op::Read(binding) => match self.typing.own.get(&binding) {
                Some(Scalar::Enum(id)) => Some(*id),
                Some(_) | None => None,
            },
            _ => self.memo.bare_enums.get(&source).copied(),
        })
    }
}
