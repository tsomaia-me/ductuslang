use std::collections::{HashMap, HashSet};

use symphonyscript_core::{BinaryOp, Builtin, FamilyId, UnaryOp};

use super::position::Position;
use super::role::Role;
use crate::diagnostics::Code;
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
    pub(super) fn units(&mut self, positions: &[Position]) {
        let mut feeds: HashMap<NodeId, Vec<Feed>> = HashMap::new();
        let mut binding_feeds: HashMap<BindingId, Vec<Feed>> = HashMap::new();
        let mut seen: HashSet<(NodeId, FamilyId, usize)> = HashSet::new();
        for position in positions
            .iter()
            .filter(|position| position.role == Role::Attribute)
        {
            for node in position.val.nodes() {
                let Some(sig) = self.memo.sigs.get(&node).copied() else {
                    continue;
                };
                if sig.is_error() || !sig.is_numeric() {
                    continue;
                }
                match (sig.family, position.unit) {
                    (Some(_), None) => {
                        self.error_at_node(
                            Code::UnitMismatch,
                            node,
                            "a unit value is bound to an attribute of no unit family",
                        );
                    }
                    (Some(family), Some(unit)) if family != unit.family => {
                        self.error_at_node(
                            Code::UnitMismatch,
                            node,
                            "a unit value is bound to an attribute of another unit family",
                        );
                    }
                    (None, Some(unit)) => {
                        if let Some(scale) = self.unit_scale(unit.family, unit.default_unit)
                            && scale != 1.0
                        {
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
                    if !seen.insert((current, feed.0, feed.1)) {
                        continue;
                    }
                    feeds.entry(current).or_default().push(feed);
                    if matches!(self.node(current).op, Op::Recursive { .. }) {
                        self.memo.call_feeds.entry(current).or_default().push(feed);
                    }
                    if let Op::Read(binding) = &self.node(current).op {
                        binding_feeds.entry(*binding).or_default().push(feed);
                    }
                    if self.carries(current, feed.0) {
                        stack.extend(self.carried_operands(current));
                    }
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
        for (node, family) in takers {
            if self.template_of(self.node(node).scope).is_some() {
                continue;
            }
            let units = units_of(feeds.get(&node).map_or(&[], Vec::as_slice), family);
            self.resolve_taker(node, family, &units);
        }
        let mut cells: Vec<(BindingId, Vec<Feed>)> = binding_feeds.into_iter().collect();
        cells.sort_by_key(|(binding, _)| *binding);
        for (binding, feeds) in cells {
            let init = self.binding(binding).init;
            let plain = self
                .memo
                .sigs
                .get(&init)
                .is_some_and(|sig| sig.family.is_none() && sig.is_numeric());
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
                    let record = self.binding(binding);
                    let message = format!(
                        "the plain cell `{}` is read at two attributes of one unit family with different default units",
                        record.name
                    );
                    self.report.error_with(
                        self.hir,
                        Code::ConflictingUnits,
                        *first,
                        &[*second],
                        message,
                    );
                }
            }
        }
    }

    fn carries(&self, node: NodeId, family: FamilyId) -> bool {
        self.memo
            .sigs
            .get(&node)
            .is_some_and(|sig| sig.open || sig.family == Some(family))
    }

    fn carried_operands(&self, node: NodeId) -> Vec<NodeId> {
        match &self.node(node).op {
            Op::Read(binding) => vec![self.binding(*binding).init],
            op => op.operands(),
        }
    }

    fn resolve_taker(&mut self, node: NodeId, family: FamilyId, units: &[(usize, Loc)]) -> bool {
        match units {
            [] => {
                let name = self.family_name(family);
                self.error_at_node(
                    Code::PlainInFamily,
                    node,
                    format!(
                        "this plain value takes the unit family `{name}`, and its family value reaches no attribute of that family to give its unit"
                    ),
                );
                false
            }
            [(unit, _)] => match self.unit_scale(family, *unit) {
                Some(scale) if scale != 1.0 => {
                    self.memo.default_scales.insert(node, scale);
                    scale.fract() != 0.0
                }
                _ => false,
            },
            [(_, first), (_, second), ..] => {
                let (loc, scope) = {
                    let record = self.node(node);
                    (record.loc, record.scope)
                };
                self.error_with_chain(
                    Code::ConflictingUnits,
                    loc,
                    vec![*first, *second],
                    scope,
                    "this plain value feeds two attributes of one unit family with different default units",
                );
                false
            }
        }
    }

    pub(in crate::semantics::analysis) fn template_units(
        &mut self,
        template: ScopeId,
        root: NodeId,
    ) {
        let Some(takers) = self.memo.template_takers.remove(&template) else {
            return;
        };
        let feeds = self
            .memo
            .template_feeds
            .get(&template)
            .cloned()
            .unwrap_or_default();
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
            let reached = carried
                .entry(family)
                .or_insert_with(|| self.carried_in(template, root, family))
                .contains(&node);
            let units = if reached {
                units_of(&feeds, family)
            } else {
                Vec::new()
            };
            fractional |= self.resolve_taker(node, family, &units);
        }
        if fractional {
            let nodes = self.template_nodes(template, root);
            self.read_in_units_of(&nodes);
        }
    }

    fn carried_in(&self, template: ScopeId, root: NodeId, family: FamilyId) -> HashSet<NodeId> {
        let mut reached = HashSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if self.template_of(self.node(node).scope) != Some(template) || !reached.insert(node) {
                continue;
            }
            if self.carries(node, family) {
                stack.extend(self.carried_operands(node));
            }
        }
        reached
    }

    fn template_nodes(&self, template: ScopeId, root: NodeId) -> Vec<NodeId> {
        let mut found = Vec::new();
        let mut seen: HashSet<NodeId> = HashSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !seen.insert(node) || self.template_of(self.node(node).scope) != Some(template) {
                continue;
            }
            found.push(node);
            stack.extend(self.node(node).op.operands());
        }
        found.sort();
        found
    }

    pub(super) fn read_in_default_units(&mut self) {
        let nodes: Vec<NodeId> = (0..self.nodes.len())
            .map(|index| NodeId(u32::try_from(index).unwrap_or(u32::MAX)))
            .collect();
        self.read_in_units_of(&nodes);
    }

    fn read_in_units_of(&mut self, nodes: &[NodeId]) {
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
            return;
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
                    || match &self.node(node).op {
                        Op::Use(operand) | Op::Unary(UnaryOp::Neg, operand) => any(&[*operand]),
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
                                self.error_at_node(Code::OperandType, node, "`div` takes integers");
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
                        Op::Read(binding) => floats.contains(&self.binding(*binding).init),
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
                            | Builtin::Rand,
                            _,
                        )
                        | Op::Recursive { .. }
                        | Op::ToRead { .. } => false,
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
    }

    fn unit_scale(&self, family: FamilyId, unit: usize) -> Option<f64> {
        self.vocabulary
            .family(family)
            .and_then(|decl| decl.units.get(unit))
            .map(|unit| unit.scale)
    }

    fn family_name(&self, family: FamilyId) -> String {
        self.vocabulary
            .family(family)
            .map_or(String::new(), |decl| decl.name.clone())
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
