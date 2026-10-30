use crate::diagnostics::Code;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::shape_fault::ShapeFault;

pub(crate) fn combined_shape(
    node: NodeId,
    op: &Op,
    shapes: &[Vec<u32>],
) -> Result<Vec<u32>, ShapeFault> {
    let first = shapes.first().cloned().unwrap_or_default();
    let agree = shapes.iter().all(|shape| *shape == first);
    match op {
        Op::Element { base, .. } => {
            let mut lengths = first;
            match lengths.pop() {
                Some(_) => Ok(lengths),
                None => Err(ShapeFault {
                    node: *base,
                    code: Code::NotAnArray,
                    message: "only an array is indexed",
                }),
            }
        }
        Op::If(..) | Op::Match { .. } => {
            if agree {
                Ok(first)
            } else {
                Err(ShapeFault {
                    node,
                    code: Code::TypeMismatch,
                    message: "the branches hold arrays of different shapes",
                })
            }
        }
        Op::Array(elements) => {
            if !agree {
                return Err(ShapeFault {
                    node,
                    code: Code::RaggedArray,
                    message: "the elements of an array are arrays of one length, or all single values",
                });
            }
            let mut lengths = first;
            lengths.push(u32::try_from(elements.len()).unwrap_or(u32::MAX));
            Ok(lengths)
        }
        Op::Join(..) => {
            let second = shapes.get(1).cloned().unwrap_or_default();
            match (first.split_last(), second.split_last()) {
                (Some((left, left_inner)), Some((right, right_inner)))
                    if left_inner == right_inner =>
                {
                    let mut lengths = left_inner.to_vec();
                    lengths.push(left.saturating_add(*right));
                    Ok(lengths)
                }
                _ => Err(ShapeFault {
                    node,
                    code: Code::RaggedArray,
                    message: "the rows of the joined arrays have different shapes",
                }),
            }
        }
        Op::Slice { .. } | Op::History { .. } | Op::Native { .. } => Ok(first),
        Op::Use(_)
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
        | Op::Slot(_)
        | Op::Unary(..)
        | Op::Binary(..)
        | Op::Pick { .. }
        | Op::Convert(..)
        | Op::Builtin(..)
        | Op::Recursive { .. }
        | Op::For(_)
        | Op::ToRead { .. } => Ok(first),
    }
}
