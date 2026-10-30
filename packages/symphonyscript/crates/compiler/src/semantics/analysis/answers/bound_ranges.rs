use symphonyscript_core::{BinaryOp, Builtin};

use super::bound_range::BoundRange;
use crate::semantics::Internal;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::op::Op;

enum Form {
    Leaf(BoundRange),
    Measured(NodeId),
    Through(NodeId),
    Pair(Combine, NodeId, NodeId),
}

#[derive(Clone, Copy)]
enum Combine {
    Smaller,
    Larger,
    Sum,
    Difference,
}

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn bound_range(
        &mut self,
        root: NodeId,
        capacity: &dyn Fn(NodeId) -> Option<i128>,
    ) -> Result<BoundRange, Internal> {
        let mut done: FastMap<NodeId, BoundRange> = FastMap::default();
        let mut stack = vec![(root, false)];
        while let Some((node, ready)) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::AnswersBoundRange.step();
            if done.contains_key(&node) {
                continue;
            }
            let form = self.bound_form_of(node)?;
            let range = match form {
                Form::Leaf(range) => range,
                Form::Measured(array) => BoundRange::between(0, capacity(array)),
                Form::Through(inner) => match done.get(&inner) {
                    Some(range) => *range,
                    None if ready => return Err(Internal::missed("bound range")),
                    None => {
                        stack.extend([(node, true), (inner, false)]);
                        continue;
                    }
                },
                Form::Pair(combine, left, right) => match (done.get(&left), done.get(&right)) {
                    (Some(left), Some(right)) => match combine {
                        Combine::Smaller => left.smaller(*right),
                        Combine::Larger => left.larger(*right),
                        Combine::Sum => left.sum(*right),
                        Combine::Difference => left.difference(*right),
                    },
                    _ if ready => return Err(Internal::missed("bound range")),
                    _ => {
                        stack.extend([(node, true), (left, false), (right, false)]);
                        continue;
                    }
                },
            };
            done.insert(node, range);
        }
        done.get(&root)
            .copied()
            .ok_or(Internal::missed("bound range"))
    }

    pub(in crate::semantics::analysis) fn bound_measures(
        &mut self,
        root: NodeId,
    ) -> Result<Vec<NodeId>, Internal> {
        let mut measured = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::AnswersBoundRange.step();
            match self.bound_form_of(node)? {
                Form::Leaf(_) => {}
                Form::Measured(array) => measured.push(array),
                Form::Through(inner) => stack.push(inner),
                Form::Pair(_, left, right) => stack.extend([left, right]),
            }
        }
        Ok(measured)
    }

    fn bound_form_of(&mut self, node: NodeId) -> Result<Form, Internal> {
        let sig = self.demand_sig(node)?;
        if sig.is_error() || sig.frame {
            return Ok(Form::Leaf(BoundRange::UNBOUNDED));
        }
        if !sig.reactive {
            return Ok(Form::Leaf(match self.demand_cval(node, EvalTy::Exact)? {
                CVal::Int(value) => BoundRange::exactly(value),
                CVal::F32(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Array(_)
                | CVal::Error => BoundRange::UNBOUNDED,
            }));
        }
        let unbounded = Form::Leaf(BoundRange::UNBOUNDED);
        Ok(match &self.node(node)?.op {
            Op::Use(inner) | Op::Chosen(inner) | Op::Annotate(_, inner) => Form::Through(*inner),
            Op::Read(binding) => {
                let record = self.binding(*binding)?;
                match record.kind {
                    BindingKind::Param | BindingKind::Let => Form::Through(record.init),
                    BindingKind::Input { .. }
                    | BindingKind::State { .. }
                    | BindingKind::Attr { .. } => unbounded,
                }
            }
            Op::Builtin(Builtin::Len, args) => match args.as_slice() {
                [array] => Form::Measured(*array),
                _ => unbounded,
            },
            Op::Builtin(Builtin::Min, args) => match args.as_slice() {
                [left, right] => Form::Pair(Combine::Smaller, *left, *right),
                _ => unbounded,
            },
            Op::Builtin(Builtin::Max, args) => match args.as_slice() {
                [left, right] => Form::Pair(Combine::Larger, *left, *right),
                _ => unbounded,
            },
            Op::Binary(BinaryOp::Add, left, right) => Form::Pair(Combine::Sum, *left, *right),
            Op::Binary(BinaryOp::Sub, left, right) => {
                Form::Pair(Combine::Difference, *left, *right)
            }
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
            | Op::If(..)
            | Op::Match { .. }
            | Op::Pick { .. }
            | Op::Element { .. }
            | Op::Array(_)
            | Op::Convert(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::For(_)
            | Op::ToRead { .. }
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::History { .. }
            | Op::Native { .. } => unbounded,
        })
    }
}
