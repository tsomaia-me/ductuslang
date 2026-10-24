use crate::semantics::analysis::bound::{Bound, BoundVal};
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::sig::{Sig, Width, kind_of};

pub(super) enum Extra {
    None,
    Read(Sig),
    ToRead { reactive: bool },
}

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn run_sig(
        &mut self,
        node: NodeId,
        need: &mut Vec<Query>,
    ) -> bool {
        let op = self.node(node).op.clone();
        let operands = op.operands();
        let sigs: Vec<Option<Sig>> = operands
            .iter()
            .map(|operand| self.sig(*operand, need))
            .collect();
        let extra = match &op {
            Op::Read(binding) => {
                let init = self.binding(*binding).init;
                self.sig(init, need).map(Extra::Read)
            }
            Op::ToRead { wire, attrs, .. } => self.destination_read_reactive(*wire, attrs, need),
            _ => Some(Extra::None),
        };
        if sigs.iter().any(Option::is_none) {
            return false;
        }
        let Some(extra) = extra else {
            return false;
        };
        let sigs: Vec<Sig> = sigs.into_iter().flatten().collect();
        let mut sig = if let Op::Arg(_) = op {
            self.memo.arg_sigs.get(&node).copied().unwrap_or(Sig::ERROR)
        } else if sigs.iter().any(Sig::is_error) && !matches!(op, Op::Recursive { .. }) {
            Sig::ERROR
        } else {
            self.rule(node, &op, &operands, &sigs, extra)
        };
        sig.frame = matches!(op, Op::Arg(_)) || sigs.iter().any(|operand| operand.frame);
        self.memo.sigs.insert(node, sig);
        true
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
    ) -> Sig {
        let annotation = self.binding(binding).annotation;
        let kind = annotation.map_or(init.kind, kind_of);
        Sig::of(kind, Width::Adaptive, true, init.family)
    }
}
