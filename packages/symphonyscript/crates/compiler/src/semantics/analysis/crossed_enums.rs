use super::engine::Engine;
use super::ids::NodeId;
use super::kind::Kind;
use crate::diagnostics::Code;
use crate::semantics::Internal;

impl Engine<'_> {
    pub(super) fn crossed_enums(
        &mut self,
        node: NodeId,
        lhs: NodeId,
        rhs: NodeId,
    ) -> Result<bool, Internal> {
        let (left, right) = (self.known_sig(lhs)?.kind, self.known_sig(rhs)?.kind);
        if left != Kind::Bare && right != Kind::Bare {
            return Ok(false);
        }
        let enum_of = |engine: &Self, operand: NodeId, kind: Kind| match kind {
            Kind::Enum(id) => Ok(Some(id)),
            Kind::Bare => engine.carried_enum_of(operand),
            Kind::Int
            | Kind::Float
            | Kind::Bool
            | Kind::Str
            | Kind::Node
            | Kind::Any
            | Kind::Error => Ok(None),
        };
        let (Some(left), Some(right)) = (enum_of(self, lhs, left)?, enum_of(self, rhs, right)?)
        else {
            return Ok(false);
        };
        if left == right {
            return Ok(false);
        }
        self.error_at_node(
            Code::OperandType,
            node,
            "`is` compares two numbers, `bool`s, values of one enum or strings",
        )?;
        Ok(true)
    }
}
