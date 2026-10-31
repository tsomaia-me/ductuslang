use super::extra::Extra;
use crate::semantics::Internal;
use crate::semantics::analysis::answers::joins;
use crate::semantics::analysis::binding::Binding;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
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
            .map(|operand| {
                crate::semantics::counted_walk::CountedWalk::SignatureRunSig.step();
                self.sig(*operand, need)
            })
            .collect();
        let group = match self.memo.groups.get(&node) {
            Some(group) => match self.sig(*group, need) {
                Some(sig) => Some(sig),
                None => return Ok(false),
            },
            None => None,
        };
        let extra = self.sig_extra(node, &op, need)?;
        if sigs.iter().any(Option::is_none) {
            return Ok(false);
        }
        let Some(extra) = extra else {
            return Ok(false);
        };
        let sigs: Vec<Sig> = sigs.into_iter().flatten().collect();
        let forms_known = arm_forms_known(&extra);
        let open = sigs.iter().any(|operand| operand.open);
        let reactive = sigs.iter().any(|operand| operand.reactive);
        let errors = sigs.iter().any(Sig::is_error)
            || matches!(op, Op::Pick { .. }) && group.is_some_and(|group| group.is_error());
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
            match self.kept_type(node, &op, &sigs, group)? {
                Some(sig) => {
                    self.memo.kept_types.insert(node);
                    sig
                }
                None => Sig::ERROR,
            }
        } else if open && !joins(&op) {
            Sig::open(reactive)
        } else {
            let mut sig = self.rule(node, &op, &operands, &sigs, extra)?;
            let open_arm = op
                .choice_arms()
                .iter()
                .any(|arm| self.memo.sigs.get(arm).is_some_and(|arm| arm.open));
            let unsettled = sig.family.is_none() && sig.is_numeric()
                || matches!(op, Op::Array(_))
                || open_arm && !sig.is_numeric();
            if open && unsettled && !sig.is_error() {
                sig.open = true;
            }
            sig
        };
        if sig.reactive && forms_known && !sig.is_error() {
            let arms = op.choice_arms();
            if !arms.is_empty() {
                let Some(chosen) = self.chosen_arm(&op, need)? else {
                    return Ok(false);
                };
                if let Some(arm) = chosen.and_then(|position| arms.get(position)) {
                    sig.reactive = self.memo.sigs.get(arm).is_some_and(|arm| arm.reactive);
                }
            }
        }
        if let Some(group) = group {
            sig = element_of(sig, group, self.memo.held_reads.contains(&node));
        }
        sig.frame = if let Op::For(for_node) = &op {
            let outer = for_node
                .iterable
                .nodes()
                .into_iter()
                .chain([for_node.start])
                .any(|operand| self.memo.sigs.get(&operand).is_some_and(|sig| sig.frame));
            outer || self.reaches_outer_frame(for_node)?
        } else if let Op::Slot(slot) = &op {
            self.slot(*slot)?.sig.frame
        } else {
            matches!(op, Op::Arg(_)) || sigs.iter().any(|operand| operand.frame)
        };
        if self.memo.open_joins.contains(&node) {
            sig = Sig::open(false);
        }
        if self.memo.frame_joins.contains(&node) {
            sig.frame = true;
            sig.reactive = false;
        }
        if !sig.is_error() && self.refused_settled_shape(node)? {
            sig = Sig::ERROR;
        }
        self.memo.sigs.insert(node, sig);
        Ok(true)
    }

    fn refused_settled_shape(&mut self, node: NodeId) -> Result<bool, Internal> {
        let Some(need) = self.memo.shapes.get(&node) else {
            return Ok(false);
        };
        let wanted = need.lengths.clone();
        if !self.rests_on_recursion(node)? {
            return Ok(false);
        }
        let Some(settled) = self.known_lengths(node)? else {
            return Ok(false);
        };
        let refused = settled.len() != wanted.len()
            || settled
                .iter()
                .zip(&wanted)
                .any(|(length, wanted)| length.is_some_and(|length| length != *wanted));
        if !refused {
            return Ok(false);
        }
        let need = self
            .memo
            .shapes
            .get(&node)
            .cloned()
            .ok_or(Internal::missed("shape need"))?;
        match need.at {
            Some((loc, scope)) => self.error(need.code, loc, scope, need.message)?,
            None => self.error_at_node(need.code, node, need.message)?,
        }
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn arm_forms(
        &mut self,
        choice: NodeId,
        arms: &[NodeId],
        need: &mut Vec<Query>,
    ) -> Result<Option<Extra>, Internal> {
        if !arms.iter().any(|arm| self.arrays.contains(arm)) {
            return Ok(Some(Extra::None));
        }
        let single = arms.iter().any(|arm| !self.arrays.contains(arm));
        let mut definite = Vec::with_capacity(arms.len());
        let mut array = false;
        for arm in arms {
            let known = self.definite_array(*arm)?;
            definite.push(known);
            array |= known;
        }
        let mut forms = Vec::with_capacity(arms.len());
        let mut outers = Vec::with_capacity(arms.len());
        for &arm in arms {
            if !self.arrays.contains(&arm) {
                let settled = self.reaches_recursion(arm)?;
                forms.push((if settled { None } else { Some(false) }, None));
                continue;
            }
            if let Some(lengths) = self.known_lengths(arm)? {
                forms.push(match lengths.split_last() {
                    Some((outer, inner)) => {
                        outers.push(*outer);
                        (Some(true), inner.iter().copied().collect())
                    }
                    None => (Some(false), None),
                });
                continue;
            }
            if !self.rests_on_recursion(arm)? {
                forms.push((self.definite_array(arm)?.then_some(true), None));
                continue;
            }
            let beside_array = arms
                .iter()
                .zip(&definite)
                .any(|(other, known)| *other != arm && *known);
            if !(single && array || beside_array) {
                forms.push((None, None));
                continue;
            }
            let Some(sig) = self.sig(arm, need) else {
                return Ok(None);
            };
            if sig.reactive || sig.frame || sig.is_error() {
                forms.push((None, None));
                continue;
            }
            let Some(found) = self.layout_of(arm, need)? else {
                return Ok(None);
            };
            forms.push(match found.shaped {
                Ok(Some(layout)) => {
                    outers.push(layout.outer());
                    (Some(true), Some(layout.inner().to_vec()))
                }
                Ok(None) => (Some(false), None),
                Err(_) => (None, None),
            });
        }
        let lengths = agreed_lengths(&forms, &outers);
        self.memo.choice_lengths.insert(choice, lengths);
        Ok(Some(Extra::Arms(forms)))
    }

    pub(in crate::semantics::analysis) fn history_sig(
        &mut self,
        node: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<Sig>>, Internal> {
        if let Some(found) = self.memo.history_kinds.get(&node) {
            return Ok(Some(*found));
        }
        let Op::History { cell, .. } = self.node(node)?.op else {
            return Ok(Some(None));
        };
        let query = self.history_cell_query(cell);
        let mut missing = Vec::new();
        if self.bound(query, &mut missing).is_none() {
            self.memo.provisional_reads.push(node);
            return Ok(Some(None));
        }
        if self.own_read(node, cell)? {
            return Ok(Some(None));
        }
        let Some(group) = self.kind_component(node, &mut missing)? else {
            self.memo.provisional_reads.push(node);
            return Ok(Some(None));
        };
        if let Some(group) = group {
            if !self.settle_kinds(group, need)? {
                return Ok(None);
            }
            return Ok(Some(self.memo.history_kinds.get(&node).copied().flatten()));
        }
        let Some(binding) = self.history_bindings(node)?.first().copied() else {
            return Ok(Some(Some(Sig::ERROR)));
        };
        let Some(init) = self.sig(self.binding(binding)?.init, need) else {
            return Ok(None);
        };
        Ok(Some(Some(self.read_sig(binding, init)?)))
    }

    pub(in crate::semantics::analysis) fn destination_read_reactive(
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

    pub(in crate::semantics::analysis) fn first_binding(
        &self,
        bound: &BoundVal,
    ) -> Option<crate::semantics::analysis::ids::BindingId> {
        match bound {
            BoundVal::Scalar(Bound::Cell(binding)) => Some(*binding),
            BoundVal::Array(array) => array.1.iter().find_map(|item| match item {
                Bound::Cell(binding) => Some(*binding),
                Bound::Value(_) => None,
            }),
            BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error => None,
        }
    }

    pub(in crate::semantics::analysis) fn read_sig(
        &self,
        binding: crate::semantics::analysis::ids::BindingId,
        init: Sig,
    ) -> Result<Sig, Internal> {
        let record = self.binding(binding)?;
        let kind = match record.annotation {
            Some(annotation) => kind_of(annotation),
            None => self.cell_kind(record, init.kind),
        };
        Ok(Sig::of(kind, Width::Adaptive, true, init.family))
    }

    pub(in crate::semantics::analysis) fn cell_kind(&self, record: &Binding, own: Kind) -> Kind {
        match record.kind {
            BindingKind::Input { input, .. }
                if own == Kind::Int && self.memo.float_inputs.contains(&input) =>
            {
                Kind::Float
            }
            BindingKind::Input { .. }
            | BindingKind::Let
            | BindingKind::Param
            | BindingKind::State { .. }
            | BindingKind::Attr { .. } => own,
        }
    }
}

fn element_of(own: Sig, group: Sig, held: bool) -> Sig {
    if own.reactive || own.open {
        return own;
    }
    if group.open {
        return if held { Sig::open(false) } else { own };
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

fn agreed_lengths(
    forms: &[(Option<bool>, Option<Vec<u32>>)],
    outers: &[Option<u32>],
) -> Option<Vec<Option<u32>>> {
    let mut rows = None;
    for form in forms {
        let (Some(true), Some(inner)) = form else {
            return None;
        };
        match rows {
            None => rows = Some(inner),
            Some(first) if first == inner => {}
            Some(_) => return None,
        }
    }
    let outer = match outers.split_first() {
        Some((first, rest))
            if outers.len() == forms.len() && rest.iter().all(|outer| outer == first) =>
        {
            *first
        }
        Some(_) | None => None,
    };
    let mut lengths: Vec<Option<u32>> = rows?.iter().copied().map(Some).collect();
    lengths.push(outer);
    Some(lengths)
}

fn arm_forms_known(extra: &Extra) -> bool {
    match extra {
        Extra::Arms(forms) => forms.iter().all(|(array, rows)| match array {
            Some(true) => rows.is_some(),
            Some(false) => true,
            None => false,
        }),
        Extra::None | Extra::Read(_) | Extra::ToRead { .. } | Extra::History(_) => true,
    }
}
