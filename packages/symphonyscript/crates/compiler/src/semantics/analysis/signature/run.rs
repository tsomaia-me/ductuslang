use symphonyscript_core::{BinaryOp, Builtin, UnaryOp};

use super::extra::Extra;
use crate::semantics::Internal;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind_of::kind_of;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::width::Width;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn run_sig(
        &mut self,
        node: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let op = self.node(node)?.op.clone();
        let operands = op.operands();
        let sigs: Vec<Option<Sig>> = operands
            .iter()
            .map(|operand| self.sig(*operand, need))
            .collect();
        let group = match self.memo.groups.get(&node) {
            Some(group) => match self.sig(*group, need) {
                Some(sig) => Some(sig),
                None => return Ok(false),
            },
            None => None,
        };
        let extra = match &op {
            Op::Read(binding) => {
                let init = self.binding(*binding)?.init;
                self.sig(init, need).map(Extra::Read)
            }
            Op::ToRead { wire, attrs, .. } => self.destination_read_reactive(*wire, attrs, need),
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Use(_)
            | Op::Chosen(_)
            | Op::Arg(_)
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
            | Op::Recursive { .. } => Some(Extra::None),
        };
        if sigs.iter().any(Option::is_none) {
            return Ok(false);
        }
        let Some(extra) = extra else {
            return Ok(false);
        };
        let sigs: Vec<Sig> = sigs.into_iter().flatten().collect();
        let open = sigs.iter().any(|operand| operand.open);
        let reactive = sigs.iter().any(|operand| operand.reactive);
        let errors = sigs.iter().any(Sig::is_error);
        let mut sig = if let Op::Arg(_) = op {
            self.memo
                .arg_sigs
                .get(&node)
                .copied()
                .ok_or(Internal::missed("argument signature"))?
        } else if let (Op::Recursive { function, args }, false) = (&op, reactive) {
            if errors {
                Sig::ERROR
            } else if open {
                Sig::open(false)
            } else {
                match self.recursive_sig(node, *function, args, need)? {
                    Some(sig) => sig,
                    None => return Ok(false),
                }
            }
        } else if errors && !matches!(op, Op::Recursive { .. }) {
            Sig::ERROR
        } else if open && !joins(&op) {
            Sig::open(reactive)
        } else {
            let mut sig = self.rule(node, &op, &operands, &sigs, extra)?;
            if open && sig.family.is_none() && sig.is_numeric() && !sig.is_error() {
                sig.open = true;
            }
            sig
        };
        if let Some(group) = group {
            sig = element_of(sig, group);
        }
        sig.frame = matches!(op, Op::Arg(_)) || sigs.iter().any(|operand| operand.frame);
        self.memo.sigs.insert(node, sig);
        Ok(true)
    }

    fn destination_read_reactive(
        &self,
        wire: crate::semantics::analysis::ids::WireId,
        attrs: &[(
            crate::semantics::analysis::item_ref::ItemRef,
            symphonyscript_core::AttrId,
        )],
        need: &mut Vec<Query>,
    ) -> Option<Extra> {
        let destination = self.destination(Query::Destination(wire), need);
        let mut reactive = matches!(destination, Some(Destination::Select { .. }));
        let mut missing = destination.is_none();
        for (item, attr) in attrs {
            match self.bound(Query::Attr(*item, *attr), need) {
                None => missing = true,
                Some(BoundVal::Scalar(Bound::Cell(_))) => reactive = true,
                Some(BoundVal::Array(array)) => {
                    if array.1.iter().any(|bound| matches!(bound, Bound::Cell(_))) {
                        reactive = true;
                    }
                }
                Some(_) => {}
            }
        }
        (!missing).then_some(Extra::ToRead { reactive })
    }

    pub(super) fn read_sig(
        &self,
        binding: crate::semantics::analysis::ids::BindingId,
        init: Sig,
    ) -> Result<Sig, Internal> {
        let annotation = self.binding(binding)?.annotation;
        let kind = annotation.map_or(init.kind, kind_of);
        Ok(Sig::of(kind, Width::Adaptive, true, init.family))
    }
}

fn element_of(own: Sig, group: Sig) -> Sig {
    if own.reactive {
        return own;
    }
    if own.is_error() || group.is_error() {
        return Sig::ERROR;
    }
    let width = match group.width {
        Width::Adaptive => own.width,
        Width::Flexible | Width::Fixed(_) | Width::Widened(_) => group.width,
    };
    Sig {
        kind: group.kind,
        width,
        family: group.family.or(own.family),
        ..own
    }
}

fn joins(op: &Op) -> bool {
    match op {
        Op::Use(_)
        | Op::Chosen(_)
        | Op::Unary(UnaryOp::Neg, _)
        | Op::If(..)
        | Op::Match { .. }
        | Op::Pick { .. }
        | Op::Element { .. }
        | Op::Array(_)
        | Op::Convert(..)
        | Op::Annotate(..) => true,
        Op::Binary(op, _, _) => matches!(
            op,
            BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Rem
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge
                | BinaryOp::Is
                | BinaryOp::IsNot
        ),
        Op::Builtin(builtin, _) => matches!(
            builtin,
            Builtin::Abs
                | Builtin::Min
                | Builtin::Max
                | Builtin::Clamp
                | Builtin::Lerp
                | Builtin::Floor
                | Builtin::Ceil
                | Builtin::Round
        ),
        Op::Number(..)
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
        | Op::Unary(UnaryOp::Not, _)
        | Op::Recursive { .. }
        | Op::ToRead { .. } => false,
    }
}
