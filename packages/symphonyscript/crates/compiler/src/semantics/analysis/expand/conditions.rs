use crate::semantics::Internal;
use crate::semantics::analysis::condition::Condition;
use crate::semantics::analysis::condition_link::ConditionLink;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{ConditionsId, InstanceId, ScopeId};
use crate::semantics::analysis::index::index;
use crate::semantics::collection::Decl;
use crate::semantics::ir::{BlockId, BodyItem, ExprId};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn push_condition(
        &mut self,
        rest: Option<ConditionsId>,
        condition: Condition,
    ) -> Result<Option<ConditionsId>, Internal> {
        let loc = self.hir.expr(condition.expr)?.loc;
        self.claim_id(self.condition_links.len(), "gate conditions", loc)?;
        let id = ConditionsId(index(self.condition_links.len()));
        self.condition_links.push(ConditionLink { condition, rest });
        Ok(Some(id))
    }

    pub(in crate::semantics::analysis) fn conditions_of(
        &self,
        head: Option<ConditionsId>,
    ) -> Result<Vec<Condition>, Internal> {
        let mut found = Vec::new();
        let mut current = head;
        while let Some(id) = current {
            let link = self
                .condition_links
                .get(id.index())
                .ok_or(Internal::missed("condition"))?;
            found.push(link.condition);
            current = link.rest;
        }
        found.reverse();
        Ok(found)
    }

    pub(in crate::semantics::analysis) fn joined_conditions(
        &mut self,
        base: Option<ConditionsId>,
        extra: Option<ConditionsId>,
    ) -> Result<Option<ConditionsId>, Internal> {
        let mut head = base;
        for condition in self.conditions_of(extra)? {
            head = self.push_condition(head, condition)?;
        }
        Ok(head)
    }

    pub(in crate::semantics::analysis) fn register_gate(&mut self, expr: ExprId, scope: ScopeId) {
        if self.gate_set.insert((expr, scope)) {
            self.gates.push((expr, scope));
        }
    }

    pub(in crate::semantics::analysis) fn unknown_items(
        &mut self,
        owner: Option<InstanceId>,
        supplies_wires: bool,
    ) {
        self.unknown_placement = true;
        match owner {
            Some(owner) => {
                self.unknown_children.insert(owner);
            }
            None => self.unknown_top = true,
        }
        if supplies_wires {
            self.unknown_wires = true;
        }
    }

    pub(in crate::semantics::analysis) fn may_supply_wires(
        &self,
        root: BlockId,
        scope: ScopeId,
    ) -> Result<bool, Internal> {
        let hir = self.hir;
        let mut blocks = vec![root];
        while let Some(block) = blocks.pop() {
            for item in &hir.block(block)?.items {
                match item {
                    BodyItem::Wire(_) => return Ok(true),
                    BodyItem::Placement(id) => {
                        let placement = hir.placement(*id)?;
                        let node = match placement.kind.as_ref() {
                            Some(kind) => {
                                matches!(self.kind_decl(scope, &kind.text)?, Some(Decl::Node(_)))
                            }
                            None => false,
                        };
                        if !node {
                            return Ok(true);
                        }
                        blocks.extend(placement.body);
                    }
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
        Ok(false)
    }
}
