use std::collections::{HashMap, HashSet};

use symphonyscript_core::{BinaryOp, Builtin, FamilyId, UnaryOp};

use super::position::{Position, Role};
use crate::diagnostics::Code;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::sig::{Kind, Sig};
use crate::semantics::ir::Loc;

type Feed = (FamilyId, usize, Loc);

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
                    match &self.node(current).op {
                        Op::Read(binding) => {
                            binding_feeds.entry(*binding).or_default().push(feed);
                            stack.push(self.binding(*binding).init);
                        }
                        op => stack.extend(op.operands()),
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
            let mut units: Vec<(usize, Loc)> = feeds
                .get(&node)
                .map(|found| {
                    found
                        .iter()
                        .filter(|(fed, _, _)| *fed == family)
                        .map(|(_, unit, loc)| (*unit, *loc))
                        .collect()
                })
                .unwrap_or_default();
            units.sort();
            units.dedup_by_key(|(unit, _)| *unit);
            match units.as_slice() {
                [] => {
                    let name = self.family_name(family);
                    self.error_at_node(
                        Code::PlainInFamily,
                        node,
                        format!(
                            "this plain value takes the unit family `{name}`, and the expression feeds no attribute of that family to give its unit"
                        ),
                    );
                }
                [(unit, _)] => {
                    if let Some(scale) = self.unit_scale(family, *unit)
                        && scale != 1.0
                    {
                        self.memo.default_scales.insert(node, scale);
                    }
                }
                [(_, first), (_, second), ..] => {
                    let (loc, scope) = {
                        let record = self.node(node);
                        (record.loc, record.scope)
                    };
                    let mut secondary = vec![*first, *second];
                    secondary.extend(self.chain(scope).to_vec());
                    self.report.error_with(
                        self.hir,
                        Code::ConflictingUnits,
                        loc,
                        &secondary,
                        "this plain value feeds two attributes of one unit family with different default units",
                    );
                }
            }
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

    pub(super) fn read_in_default_units(&mut self) {
        let fractional: HashSet<NodeId> = self
            .memo
            .default_scales
            .iter()
            .filter(|(_, scale)| scale.fract() != 0.0)
            .map(|(node, _)| *node)
            .collect();
        if fractional.is_empty() {
            return;
        }
        let mut floats: HashSet<NodeId> = HashSet::new();
        let mut changed = true;
        while changed {
            changed = false;
            for index in 0..self.nodes.len() {
                let node = NodeId(u32::try_from(index).unwrap_or(u32::MAX));
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
                        Op::Pick { items, .. } => any(items),
                        Op::Builtin(
                            Builtin::Abs | Builtin::Min | Builtin::Max | Builtin::Clamp,
                            args,
                        ) => any(args),
                        Op::Read(binding) => floats.contains(&self.binding(*binding).init),
                        _ => false,
                    };
                if becomes_float {
                    floats.insert(node);
                    self.memo.sigs.insert(
                        node,
                        Sig {
                            kind: Kind::Float,
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
