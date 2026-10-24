use super::engine::Engine;
use super::ids::NodeId;
use super::op::Op;

pub(crate) fn shape_parts(engine: &Engine<'_>, op: &Op) -> Vec<NodeId> {
    match op {
        Op::Read(binding) => vec![engine.binding(*binding).init],
        Op::Use(operand) | Op::Annotate(_, operand) => vec![*operand],
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
        | Op::Unary(..)
        | Op::Binary(..)
        | Op::Pick { .. }
        | Op::Convert(..)
        | Op::Builtin(..)
        | Op::Recursive { .. }
        | Op::ToRead { .. } => Vec::new(),
    }
}
