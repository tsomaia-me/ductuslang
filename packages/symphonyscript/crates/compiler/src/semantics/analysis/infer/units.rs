use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use symphonyscript_core::{BinaryOp, Builtin, FamilyId, UnaryOp};

use super::position::Position;
use super::role::Role;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::feed::Feed;
use crate::semantics::analysis::ids::{BindingId, NodeId, ScopeId};
use crate::semantics::analysis::is_integer::is_integer;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::Loc;

impl Engine<'_> {
    pub(super) fn units(&mut self, positions: &[Position]) -> Result<(), Internal> {
        let mut feeds: HashMap<NodeId, Vec<Feed>> = HashMap::new();
        let mut binding_feeds: HashMap<BindingId, Vec<Feed>> = HashMap::new();
        let mut seen: HashSet<(NodeId, FamilyId, usize)> = HashSet::new();
        for position in positions
            .iter()
            .filter(|position| position.role == Role::Attribute)
        {
            for node in position.val.nodes() {
                let sig = self.known_sig(node)?;
                if sig.is_error() || !sig.is_numeric() {
                    continue;
                }
                match (sig.family, position.unit) {
                    (Some(_), None) => {
                        self.error_at_node(
                            Code::UnitMismatch,
                            node,
                            "a unit value is bound to an attribute of no unit family",
                        )?;
                    }
                    (Some(family), Some(unit)) if family != unit.family => {
                        self.error_at_node(
                            Code::UnitMismatch,
                            node,
                            "a unit value is bound to an attribute of another unit family",
                        )?;
                    }
                    (None, Some(unit)) => {
                        let scale = self.unit_decl(unit.family, unit.default_unit)?.scale;
                        if scale != 1.0 {
                            self.memo.default_scales.insert(node, scale);
                        }
                    }
                    _ => {}
                }
                let Some(unit) = position.unit else {
                    continue;
                };
                let feed = (unit.family, unit.default_unit, position.loc);
                let mut stack = vec![node];
                while let Some(current) = stack.pop() {
                    crate::semantics::counted_walk::CountedWalk::UnitsUnits.step();
                    if !seen.insert((current, feed.0, feed.1)) {
                        continue;
                    }
                    feeds.entry(current).or_default().push(feed);
                    if matches!(self.node(current)?.op, Op::Recursive { .. }) {
                        self.memo.call_feeds.entry(current).or_default().push(feed);
                    }
                    if let Op::Read(binding) = &self.node(current)?.op {
                        binding_feeds.entry(*binding).or_default().push(feed);
                    }
                    if matches!(self.node(current)?.op, Op::History { .. }) {
                        for binding in self.history_bindings(current)? {
                            binding_feeds.entry(binding).or_default().push(feed);
                        }
                    }
                    if self.carries(current, feed.0)? {
                        stack.extend(self.carried_operands(current)?);
                    }
                    stack.extend(self.memo.groups.get(&current).copied());
                }
            }
        }
        let mut takers: Vec<(NodeId, FamilyId)> = self
            .memo
            .takes_family
            .iter()
            .map(|(node, family)| (*node, *family))
            .collect();
        takers.sort();
        let placeholders = self.under_placeholders()?;
        let mut conflicted: HashSet<BindingId> = HashSet::new();
        for (node, family) in takers {
            if self.template_of(self.node(node)?.scope)?.is_some() {
                continue;
            }
            let units = units_of(feeds.get(&node).map_or(&[], Vec::as_slice), family);
            if units.is_empty() && placeholders.contains(&node) {
                continue;
            }
            if let ([_, _, ..], Op::Read(binding)) = (units.as_slice(), &self.node(node)?.op) {
                conflicted.insert(*binding);
            }
            self.resolve_taker(node, family, &units)?;
        }
        let mut cells: Vec<(BindingId, Vec<Feed>)> = binding_feeds.into_iter().collect();
        cells.sort_by_key(|(binding, _)| *binding);
        for (binding, feeds) in cells {
            if conflicted.contains(&binding) {
                continue;
            }
            let init = self.binding(binding)?.init;
            let sig = self.known_sig(init)?;
            let plain = sig.family.is_none() && sig.is_numeric();
            if !plain {
                continue;
            }
            let mut by_family: HashMap<FamilyId, Vec<(usize, Loc)>> = HashMap::new();
            for (family, unit, loc) in feeds {
                by_family.entry(family).or_default().push((unit, loc));
            }
            let mut families: Vec<(FamilyId, Vec<(usize, Loc)>)> = by_family.into_iter().collect();
            families.sort_by_key(|(family, _)| family.index());
            for (_, mut units) in families {
                units.sort();
                units.dedup_by_key(|(unit, _)| *unit);
                if let [(_, first), (_, second), ..] = units.as_slice() {
                    let record = self.binding(binding)?;
                    let message = format!(
                        "the plain cell `{}` is read at two attributes of one unit family with different default units",
                        record.name
                    );
                    self.report.error_with(
                        self.hir,
                        Code::ConflictingUnits,
                        *first,
                        &[(*second, "the other attribute".to_string())],
                        message,
                    );
                }
            }
        }
        Ok(())
    }

    fn under_placeholders(&self) -> Result<HashSet<NodeId>, Internal> {
        let mut reached = HashSet::new();
        let mut stack: Vec<NodeId> = self
            .memo
            .sigs
            .iter()
            .filter(|(_, sig)| sig.is_error())
            .map(|(node, _)| *node)
            .collect();
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::UnitsUnderPlaceholders.step();
            if reached.insert(node) {
                stack.extend(self.carried_operands(node)?);
                if let Op::Native { args, .. } = &self.node(node)?.op {
                    stack.extend(args.iter().copied());
                }
            }
        }
        Ok(reached)
    }

    fn carries(&self, node: NodeId, family: FamilyId) -> Result<bool, Internal> {
        let sig = self.known_sig(node)?;
        Ok(sig.open || sig.family == Some(family))
    }

    fn carried_operands(&self, node: NodeId) -> Result<Vec<NodeId>, Internal> {
        Ok(match &self.node(node)?.op {
            Op::Read(binding) => vec![self.binding(*binding)?.init],
            Op::Pick { items, .. } => items.clone(),
            Op::Element { base, .. } | Op::Slice { base, .. } => vec![*base],
            Op::Native { .. } | Op::Builtin(Builtin::Len | Builtin::Bytes, _) => Vec::new(),
            op => op.operands(),
        })
    }

    fn resolve_taker(
        &mut self,
        node: NodeId,
        family: FamilyId,
        units: &[(usize, Loc)],
    ) -> Result<bool, Internal> {
        Ok(match units {
            [] => {
                let name = self.family_decl(family)?.name.clone();
                self.error_at_node(
                    Code::PlainInFamily,
                    node,
                    format!(
                        "this plain value takes the unit family `{name}`, and its family value reaches no attribute of that family to give its unit"
                    ),
                )?;
                false
            }
            [(unit, _)] => {
                let scale = self.unit_decl(family, *unit)?.scale;
                if scale == 1.0 {
                    false
                } else {
                    self.memo.default_scales.insert(node, scale);
                    scale.fract() != 0.0
                }
            }
            [(_, first), (_, second), ..] => {
                let (loc, scope) = {
                    let record = self.node(node)?;
                    (record.loc, record.scope)
                };
                self.error_with_chain(
                    Code::ConflictingUnits,
                    loc,
                    vec![
                        (*first, "one attribute".to_string()),
                        (*second, "the other attribute".to_string()),
                    ],
                    scope,
                    "this plain value feeds two attributes of one unit family with different default units",
                )?;
                false
            }
        })
    }

    pub(in crate::semantics::analysis) fn template_units(
        &mut self,
        template: ScopeId,
        root: NodeId,
    ) -> Result<(), Internal> {
        let Some(takers) = self.memo.template_takers.remove(&template) else {
            return Ok(());
        };
        let feeds = self
            .memo
            .template_feeds
            .get(&template)
            .cloned()
            .ok_or(Internal::missed("template feeds"))?;
        let mut takers: Vec<(NodeId, FamilyId)> = takers
            .into_iter()
            .filter_map(|node| {
                self.memo
                    .takes_family
                    .get(&node)
                    .map(|family| (node, *family))
            })
            .collect();
        takers.sort();
        takers.dedup();
        let mut fractional = false;
        let mut carried: HashMap<FamilyId, HashSet<NodeId>> = HashMap::new();
        for (node, family) in takers {
            if let Entry::Vacant(entry) = carried.entry(family) {
                entry.insert(self.carried_in(template, root, family)?);
            }
            let reached = carried
                .get(&family)
                .ok_or(Internal::missed("carried family"))?
                .contains(&node);
            let units = if reached {
                units_of(&feeds, family)
            } else {
                Vec::new()
            };
            fractional |= self.resolve_taker(node, family, &units)?;
        }
        if fractional {
            let nodes = self.template_nodes(template, root)?;
            self.read_in_units_of(&nodes)?;
        }
        Ok(())
    }

    fn carried_in(
        &self,
        template: ScopeId,
        root: NodeId,
        family: FamilyId,
    ) -> Result<HashSet<NodeId>, Internal> {
        let mut reached = HashSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::UnitsCarriedIn.step();
            if self.template_of(self.node(node)?.scope)? != Some(template) || !reached.insert(node)
            {
                continue;
            }
            if self.carries(node, family)? {
                stack.extend(self.carried_operands(node)?);
            }
            stack.extend(self.memo.groups.get(&node).copied());
        }
        Ok(reached)
    }

    fn template_nodes(&self, template: ScopeId, root: NodeId) -> Result<Vec<NodeId>, Internal> {
        let mut found = Vec::new();
        let mut seen: HashSet<NodeId> = HashSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::UnitsTemplateNodes.step();
            if !seen.insert(node) || self.template_of(self.node(node)?.scope)? != Some(template) {
                continue;
            }
            found.push(node);
            stack.extend(self.node(node)?.op.operands());
        }
        found.sort();
        Ok(found)
    }

    pub(super) fn read_in_default_units(&mut self) -> Result<(), Internal> {
        let nodes: Vec<NodeId> = (0..self.nodes.len())
            .map(|index| NodeId(u32::try_from(index).unwrap_or(u32::MAX)))
            .collect();
        self.read_in_units_of(&nodes)
    }

    fn read_in_units_of(&mut self, nodes: &[NodeId]) -> Result<(), Internal> {
        let fractional: HashSet<NodeId> = nodes
            .iter()
            .copied()
            .filter(|node| {
                self.memo
                    .default_scales
                    .get(node)
                    .is_some_and(|scale| scale.fract() != 0.0)
            })
            .collect();
        if fractional.is_empty() {
            return Ok(());
        }
        let mut floats: HashSet<NodeId> = HashSet::new();
        let mut changed = true;
        while changed {
            changed = false;
            for node in nodes.iter().copied() {
                let Some(sig) = self.memo.sigs.get(&node).copied() else {
                    continue;
                };
                if sig.kind != Kind::Int || floats.contains(&node) {
                    continue;
                }
                let any = |nodes: &[NodeId]| nodes.iter().any(|node| floats.contains(node));
                let becomes_float = fractional.contains(&node)
                    || match &self.node(node)?.op {
                        Op::Use(operand)
                        | Op::Chosen(operand)
                        | Op::Unary(UnaryOp::Neg, operand) => any(&[*operand]),
                        Op::Binary(
                            BinaryOp::Add
                            | BinaryOp::Sub
                            | BinaryOp::Mul
                            | BinaryOp::Rem
                            | BinaryOp::Pow,
                            lhs,
                            rhs,
                        ) => any(&[*lhs, *rhs]),
                        Op::Binary(BinaryOp::IntDiv, lhs, rhs) => {
                            if any(&[*lhs, *rhs]) {
                                self.error_at_node(
                                    Code::OperandType,
                                    node,
                                    "`div` takes integers",
                                )?;
                                self.memo.sigs.insert(node, Sig::ERROR);
                                changed = true;
                            }
                            continue;
                        }
                        Op::If(_, then, otherwise) => any(&[*then, *otherwise]),
                        Op::Match {
                            arms, otherwise, ..
                        } => {
                            arms.iter().any(|(_, value)| floats.contains(value))
                                || otherwise.is_some_and(|value| floats.contains(&value))
                        }
                        Op::Pick { items, .. } | Op::Array(items) => any(items),
                        Op::Element { base, .. } => any(&[*base]),
                        Op::Builtin(
                            Builtin::Abs | Builtin::Min | Builtin::Max | Builtin::Clamp,
                            args,
                        ) => any(args),
                        Op::Read(binding) => floats.contains(&self.binding(*binding)?.init),
                        Op::Join(lhs, rhs) => any(&[*lhs, *rhs]),
                        Op::Slice { base, .. } => any(&[*base]),
                        Op::History { fallback, .. } => {
                            fallback.is_some_and(|value| floats.contains(&value))
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
                        | Op::Unary(UnaryOp::Not, _)
                        | Op::Names(..)
                        | Op::Binary(
                            BinaryOp::Div
                            | BinaryOp::Is
                            | BinaryOp::IsNot
                            | BinaryOp::Lt
                            | BinaryOp::Le
                            | BinaryOp::Gt
                            | BinaryOp::Ge
                            | BinaryOp::And
                            | BinaryOp::Or,
                            _,
                            _,
                        )
                        | Op::Convert(..)
                        | Op::Annotate(..)
                        | Op::Builtin(
                            Builtin::Floor
                            | Builtin::Ceil
                            | Builtin::Round
                            | Builtin::Sqrt
                            | Builtin::Sin
                            | Builtin::Cos
                            | Builtin::Tan
                            | Builtin::Exp
                            | Builtin::Log
                            | Builtin::Lerp
                            | Builtin::Rand
                            | Builtin::Len
                            | Builtin::Bytes,
                            _,
                        )
                        | Op::Recursive { .. }
                        | Op::ToRead { .. }
                        | Op::Native { .. } => false,
                    };
                if becomes_float {
                    floats.insert(node);
                    let width = match sig.width {
                        Width::Fixed(scalar) if is_integer(scalar) => Width::Widened(scalar),
                        other => other,
                    };
                    self.memo.sigs.insert(
                        node,
                        Sig {
                            kind: Kind::Float,
                            width,
                            ..sig
                        },
                    );
                    changed = true;
                }
            }
        }
        Ok(())
    }
}

fn units_of(feeds: &[Feed], family: FamilyId) -> Vec<(usize, Loc)> {
    let mut units: Vec<(usize, Loc)> = feeds
        .iter()
        .filter(|(fed, _, _)| *fed == family)
        .map(|(_, unit, loc)| (*unit, *loc))
        .collect();
    units.sort();
    units.dedup_by_key(|(unit, _)| *unit);
    units
}
