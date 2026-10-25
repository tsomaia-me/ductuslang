use crate::semantics::Internal;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::op::Op;

impl Engine<'_> {
    pub(super) fn mark_own_history(&mut self) -> Result<(), Internal> {
        for index in 0..self.hir.states.len() {
            let state = crate::semantics::ir::StateId(u32::try_from(index).unwrap_or(u32::MAX));
            let Some(BoundVal::Scalar(Bound::Cell(binding))) =
                self.memo.states.get(&state).cloned()
            else {
                continue;
            };
            let init = self.binding(binding)?.init;
            for (node, cell) in self.history_reads(init)? {
                if cell == HistoryCell::State(state) {
                    self.memo.own_history.insert(node);
                }
            }
        }
        Ok(())
    }

    pub(in crate::semantics::analysis) fn history_bindings(
        &self,
        node: NodeId,
    ) -> Result<Vec<BindingId>, Internal> {
        let Op::History { cell, .. } = self.node(node)?.op else {
            return Ok(Vec::new());
        };
        let bound = match cell {
            HistoryCell::Input(input) => self.memo.inputs.get(&input),
            HistoryCell::Let(id) => self
                .memo
                .lets
                .get(&(self.global_of_let(id), LetRef::Top(id))),
            HistoryCell::State(state) => self.memo.states.get(&state),
        };
        Ok(match bound {
            Some(BoundVal::Scalar(Bound::Cell(binding))) => vec![*binding],
            Some(BoundVal::Array(array)) => array
                .1
                .iter()
                .filter_map(|item| match item {
                    Bound::Cell(binding) => Some(*binding),
                    Bound::Value(_) => None,
                })
                .collect(),
            Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error) | None => Vec::new(),
        })
    }
}
