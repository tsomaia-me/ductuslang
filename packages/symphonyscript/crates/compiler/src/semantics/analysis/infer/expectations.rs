use symphonyscript_core::{BinaryOp, Builtin, Scalar, UnaryOp};

use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::is_float::is_float;
use crate::semantics::analysis::is_integer::is_integer;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::ir::Loc;

impl Engine<'_> {
    fn read_as_float(&self, operand: NodeId, other: NodeId) -> Option<Scalar> {
        let kind = |node: NodeId| self.memo.sigs.get(&node).map(|sig| sig.kind);
        (kind(operand) == Some(Kind::Int) && kind(other) == Some(Kind::Float))
            .then_some(Scalar::F64)
    }

    pub(super) fn expect(
        &mut self,
        root: NodeId,
        expected: Option<Scalar>,
    ) -> Result<(), Internal> {
        let mut stack = vec![(root, expected)];
        while let Some((node, expected)) = stack.pop() {
            if !self.typing.expected_seen.insert((node, expected)) {
                continue;
            }
            let reactive = self.memo.sigs.get(&node).is_some_and(|sig| sig.reactive);
            if !reactive {
                continue;
            }
            let loc: Loc = self.node(node)?.loc;
            let op = self.node(node)?.op.clone();
            let numeric = expected.filter(|scalar| is_integer(*scalar) || is_float(*scalar));
            let float = expected.filter(|scalar| is_float(*scalar));
            let integer = expected.filter(|scalar| is_integer(*scalar));
            match op {
                Op::Read(binding) => {
                    self.typing.reads.entry(binding).or_default().push(loc);
                    if let Some(expected) = expected {
                        self.typing
                            .expectations
                            .entry(binding)
                            .or_default()
                            .push((expected, loc));
                    }
                }
                Op::Use(inner) => stack.push((inner, expected)),
                Op::Unary(UnaryOp::Neg, operand) => stack.push((operand, numeric)),
                Op::Unary(UnaryOp::Not, operand) => stack.push((operand, None)),
                Op::Binary(
                    BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge
                    | BinaryOp::Is
                    | BinaryOp::IsNot,
                    lhs,
                    rhs,
                ) => {
                    stack.push((lhs, self.read_as_float(lhs, rhs)));
                    stack.push((rhs, self.read_as_float(rhs, lhs)));
                }
                Op::Binary(op, lhs, rhs) => {
                    let operands = match op {
                        BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Rem
                        | BinaryOp::Pow => numeric,
                        BinaryOp::Div => float,
                        BinaryOp::IntDiv => integer,
                        BinaryOp::Is
                        | BinaryOp::IsNot
                        | BinaryOp::Lt
                        | BinaryOp::Le
                        | BinaryOp::Gt
                        | BinaryOp::Ge
                        | BinaryOp::And
                        | BinaryOp::Or => None,
                    };
                    stack.push((lhs, operands));
                    stack.push((rhs, operands));
                }
                Op::If(condition, then, otherwise) => {
                    stack.push((condition, None));
                    stack.push((then, expected));
                    stack.push((otherwise, expected));
                }
                Op::Match {
                    scrutinee,
                    arms,
                    otherwise,
                } => {
                    stack.push((scrutinee, None));
                    for (_, value) in arms {
                        stack.push((value, expected));
                    }
                    stack.extend(otherwise.map(|value| (value, expected)));
                }
                Op::Pick { index, items } => {
                    stack.push((index, None));
                    stack.extend(items.into_iter().map(|item| (item, expected)));
                }
                Op::Element { base, index } => {
                    stack.push((index, None));
                    stack.push((base, expected));
                }
                Op::Array(elements) => {
                    stack.extend(elements.into_iter().map(|element| (element, expected)));
                }
                Op::Convert(target, operand) => {
                    stack.push((operand, is_float(target).then_some(target)));
                }
                Op::Annotate(target, operand) => stack.push((operand, Some(target))),
                Op::Builtin(builtin, args) => {
                    let operands = match builtin {
                        Builtin::Abs | Builtin::Min | Builtin::Max | Builtin::Clamp => numeric,
                        Builtin::Rand => Some(Scalar::I64),
                        Builtin::Floor
                        | Builtin::Ceil
                        | Builtin::Round
                        | Builtin::Sqrt
                        | Builtin::Sin
                        | Builtin::Cos
                        | Builtin::Tan
                        | Builtin::Exp
                        | Builtin::Log
                        | Builtin::Lerp => float,
                    };
                    stack.extend(args.into_iter().map(|arg| (arg, operands)));
                }
                other => stack.extend(other.operands().into_iter().map(|operand| (operand, None))),
            }
        }
        Ok(())
    }
}
