use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::op::Op;
use crate::semantics::ir::ExprId;

impl Engine<'_> {
    pub(super) fn bare_label(
        &mut self,
        label: ExprId,
        name: &str,
        scope: ScopeId,
    ) -> Result<NodeId, Internal> {
        if let Some(node) = self.memo.bare_labels.get(&(label, scope)) {
            return Ok(*node);
        }
        let loc = self.hir.expr(label)?.loc;
        let node = self.push_node(Op::Bare(String::from(name)), loc, scope)?;
        self.memo.bare_labels.insert((label, scope), node);
        Ok(node)
    }
}
