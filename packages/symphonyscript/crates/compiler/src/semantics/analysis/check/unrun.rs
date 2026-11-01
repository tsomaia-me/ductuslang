use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::answers::CellRead;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::op::Op;
use crate::semantics::ir::StateId;

#[derive(Default)]
struct GroupReads {
    outside: bool,
    own_history: bool,
}

impl Engine<'_> {
    pub(super) fn unrun_cells(&mut self) -> Result<(), Internal> {
        let mut checked = vec![false; self.hir.states.len()];
        for index in 0..self.hir.states.len() {
            crate::semantics::counted_walk::CountedWalk::UnrunCells.step();
            if checked.get(index).copied().unwrap_or(true) {
                continue;
            }
            let state = StateId(u32::try_from(index).map_err(|_| Internal::missed("state"))?);
            let members: Vec<StateId> = self.collected.cell_groups.group_of(state).0.to_vec();
            for member in &members {
                if let Some(slot) = checked.get_mut(member.index()) {
                    *slot = true;
                }
            }
            if !self.group_unrun(&members)? {
                continue;
            }
            let Some(first) = members.iter().min().copied() else {
                continue;
            };
            let decl = self.hir.state(first)?;
            let Some(name) = decl.name.as_ref() else {
                continue;
            };
            let message = if members.len() == 1 {
                format!(
                    "`{}` has no clock and reads no cell but its own history, so nothing runs it",
                    name.text
                )
            } else {
                format!(
                    "`{}` lies on a cycle of cells with no clock that reads no cell outside itself, so nothing runs it",
                    name.text
                )
            };
            let help = format!(
                "give `{0}` a clock, as in `state {0}(0) on tick = …`, or read a cell that steps in its rule",
                name.text
            );
            let (loc, global) = (name.loc, self.source_global);
            self.error_helped(Code::UnrunCell, loc, global, message, help)?;
        }
        Ok(())
    }

    pub(in crate::semantics::analysis) fn group_unrun(
        &self,
        members: &[StateId],
    ) -> Result<bool, Internal> {
        let mut roots = Vec::with_capacity(members.len());
        for member in members {
            let decl = self.hir.state(*member)?;
            if !decl.clock.is_empty() {
                return Ok(false);
            }
            match self.memo.states.get(member) {
                Some(BoundVal::Scalar(Bound::Cell(binding))) => roots.push(*binding),
                Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Array(_) | BoundVal::Error)
                | None => return Ok(false),
            }
        }
        let mut reads = GroupReads::default();
        let mut seen_bindings: FastSet<BindingId> = roots.iter().copied().collect();
        let mut seen_nodes: FastSet<NodeId> = FastSet::default();
        let mut stack: Vec<NodeId> = Vec::new();
        for root in &roots {
            let init = self.binding(*root)?.init;
            if self.memo.sigs.get(&init).is_some_and(|sig| sig.is_error()) {
                return Ok(false);
            }
            stack.push(init);
        }
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::UnrunCells.step();
            if !seen_nodes.insert(node) {
                continue;
            }
            if let Op::Stepped(cell) = &self.node(node)?.op {
                self.note_cell(*cell, members, &mut reads);
            }
            match self.cell_read(node)? {
                CellRead::Binding(binding) => {
                    let record = self.binding(binding)?;
                    match record.kind {
                        BindingKind::State { state } if members.contains(&state) => {}
                        BindingKind::Input { .. } | BindingKind::State { .. } => {
                            reads.outside = true;
                        }
                        BindingKind::Let | BindingKind::Param | BindingKind::Attr { .. } => {
                            if seen_bindings.insert(binding) {
                                stack.push(record.init);
                            }
                        }
                    }
                }
                CellRead::History { cell, fallback } => {
                    self.note_cell(cell, members, &mut reads);
                    stack.extend(fallback);
                }
                CellRead::Named(inner) => stack.push(inner),
                CellRead::Operands(operands) => stack.extend(operands),
            }
            if reads.outside {
                return Ok(false);
            }
        }
        Ok(reads.own_history && !reads.outside)
    }

    fn note_cell(&self, cell: HistoryCell, members: &[StateId], reads: &mut GroupReads) {
        match cell {
            HistoryCell::State(state) if members.contains(&state) => reads.own_history = true,
            HistoryCell::State(_) | HistoryCell::Input(_) | HistoryCell::Let(_) => {
                reads.outside = true;
            }
        }
    }
}
