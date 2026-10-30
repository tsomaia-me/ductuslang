use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::op::Op;

pub(crate) fn shape_parts(engine: &Engine<'_>, op: &Op) -> Result<Vec<NodeId>, Internal> {
    Ok(match op {
        Op::Read(binding) => vec![engine.binding(*binding)?.init],
        Op::Use(operand) | Op::Chosen(operand) | Op::Annotate(_, operand) => vec![*operand],
        Op::Element { base, .. } => vec![*base],
        Op::If(_, then, otherwise) => vec![*then, *otherwise],
        Op::Match {
            arms, otherwise, ..
        } => {
            let mut parts: Vec<NodeId> = arms.iter().map(|(_, value)| *value).collect();
            parts.extend(otherwise.iter().copied());
            parts
        }
        Op::Array(elements) => elements.clone(),
        Op::Join(lhs, rhs) => vec![*lhs, *rhs],
        Op::Slice { base, .. } => vec![*base],
        Op::History { fallback, .. } => fallback.iter().copied().collect(),
        Op::Builtin(symphonyscript_core::Builtin::Bytes, operands) => operands.clone(),
        Op::Native { .. } => Vec::new(),
        Op::Number(..)
        | Op::Const(_)
        | Op::Bool(_)
        | Op::Str(_)
        | Op::Concat(_)
        | Op::Variant(..)
        | Op::Bare(_)
        | Op::Pi
        | Op::NodeRef(_)
        | Op::Arg(_)
        | Op::Slot(_)
        | Op::Unary(..)
        | Op::Binary(..)
        | Op::Pick { .. }
        | Op::Convert(..)
        | Op::Builtin(..)
        | Op::Recursive { .. }
        | Op::For(_)
        | Op::ToRead { .. } => Vec::new(),
    })
}
