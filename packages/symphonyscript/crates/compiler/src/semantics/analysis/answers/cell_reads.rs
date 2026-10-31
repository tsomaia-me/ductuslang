use super::cell_read::CellRead;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::op::Op;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn cell_read(
        &self,
        node: NodeId,
    ) -> Result<CellRead, Internal> {
        let op = &self.node(node)?.op;
        Ok(match op {
            Op::Read(binding) => CellRead::Binding(*binding),
            Op::History { cell, fallback, .. } => CellRead::History {
                cell: *cell,
                fallback: *fallback,
            },
            Op::Use(inner) => CellRead::Named(*inner),
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Chosen(_)
            | Op::Arg(_)
            | Op::Slot(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::If(..)
            | Op::Match { .. }
            | Op::Pick { .. }
            | Op::Element { .. }
            | Op::Array(_)
            | Op::Convert(..)
            | Op::Annotate(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::For(_)
            | Op::ToRead { .. }
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::Stepped(_)
            | Op::Native { .. } => CellRead::Operands(op.operands()),
        })
    }
}
