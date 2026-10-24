use crate::semantics::analysis::condition::Condition;
use crate::semantics::analysis::condition_link::ConditionLink;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{ConditionsId, InstanceId, ScopeId};
use crate::semantics::analysis::index::index;
use crate::semantics::ir::{BlockId, BodyItem, ExprId};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn push_condition(
        &mut self,
        rest: Option<ConditionsId>,
        condition: Condition,
    ) -> Option<ConditionsId> {
        let id = ConditionsId(index(self.condition_links.len()));
        self.condition_links.push(ConditionLink { condition, rest });
        Some(id)
    }

    pub(in crate::semantics::analysis) fn conditions_of(
        &self,
        head: Option<ConditionsId>,
    ) -> Vec<Condition> {
        let mut found = Vec::new();
        let mut current = head;
        while let Some(link) = current.and_then(|id| self.condition_links.get(id.index())) {
            found.push(link.condition);
            current = link.rest;
        }
        found.reverse();
        found
    }

    pub(in crate::semantics::analysis) fn joined_conditions(
        &mut self,
        base: Option<ConditionsId>,
        extra: Option<ConditionsId>,
    ) -> Option<ConditionsId> {
        let mut head = base;
        for condition in self.conditions_of(extra) {
            head = self.push_condition(head, condition);
        }
        head
    }

    pub(in crate::semantics::analysis) fn register_gate(&mut self, expr: ExprId, scope: ScopeId) {
        if self.gate_set.insert((expr, scope)) {
            self.gates.push((expr, scope));
        }
    }

    pub(in crate::semantics::analysis) fn unknown_items(
        &mut self,
        owner: Option<InstanceId>,
        body: Option<BlockId>,
    ) {
        self.unknown_placement = true;
        match owner {
            Some(owner) => {
                self.unknown_children.insert(owner);
            }
            None => self.unknown_top = true,
        }
        if body.is_some_and(|block| self.holds_wire(block)) {
            self.unknown_wires = true;
        }
    }

    fn holds_wire(&self, root: BlockId) -> bool {
        let hir = self.hir;
        let mut blocks = vec![root];
        while let Some(block) = blocks.pop() {
            for item in &hir.block(block).items {
                match item {
                    BodyItem::Wire(_) => return true,
                    BodyItem::Placement(id) => blocks.extend(hir.placement(*id).body),
                    BodyItem::For { body, .. } => blocks.push(*body),
                    BodyItem::When {
                        arms, otherwise, ..
                    } => {
                        blocks.extend(arms.iter().map(|arm| arm.body));
                        blocks.extend(otherwise.iter().copied());
                    }
                    BodyItem::Bundle { .. } => {}
                }
            }
        }
        false
    }
}
