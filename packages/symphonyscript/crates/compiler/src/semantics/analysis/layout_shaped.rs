use super::op::Op;

pub(crate) fn layout_shaped(op: &Op) -> bool {
    match op {
        Op::Slice { .. } | Op::History { .. } | Op::Native { .. } => true,
        Op::Element { .. }
        | Op::If(..)
        | Op::Match { .. }
        | Op::Array(_)
        | Op::Join(..)
        | Op::Use(_)
        | Op::Chosen(_)
        | Op::Annotate(..)
        | Op::Number(..)
        | Op::Const(_)
        | Op::Bool(_)
        | Op::Str(_)
        | Op::Concat(_)
        | Op::Variant(..)
        | Op::Bare(_)
        | Op::Pi
        | Op::NodeRef(_)
        | Op::Read(_)
        | Op::Arg(_)
        | Op::Unary(..)
        | Op::Names(..)
        | Op::Binary(..)
        | Op::Pick { .. }
        | Op::Convert(..)
        | Op::Builtin(..)
        | Op::Recursive { .. }
        | Op::ToRead { .. } => false,
    }
}
