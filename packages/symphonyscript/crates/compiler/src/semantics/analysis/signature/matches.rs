use std::collections::BTreeSet;

use symphonyscript_core::Scalar;

use super::branch_width_of::branch_width_of;
use crate::diagnostics::Code;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::unify_kinds::unify_kinds;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::{Number, Suffix};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Label {
    Variant(u16),
    Int(i128),
    Bool(bool),
}

impl Engine<'_> {
    pub(super) fn match_sig(
        &mut self,
        node: NodeId,
        op: &Op,
        operands: &[NodeId],
        sigs: &[Sig],
    ) -> Sig {
        let Op::Match {
            arms, otherwise, ..
        } = op
        else {
            return Sig::ERROR;
        };
        let scrutinee = sigs.first().copied().unwrap_or(Sig::ERROR);
        let scrutinee_node = operands.first().copied().unwrap_or(node);
        if !matches!(scrutinee.kind, Kind::Enum(_) | Kind::Int | Kind::Bool) {
            self.error_at_node(
                Code::MatchScrutinee,
                scrutinee_node,
                "a `match` selects by an enum value, an integer or a `bool`",
            );
            return Sig::ERROR;
        }
        let mut seen: BTreeSet<Label> = BTreeSet::new();
        for (label, _) in arms {
            let Some(value) = self.label(*label, &scrutinee) else {
                return Sig::ERROR;
            };
            if !seen.insert(value) {
                self.error_at_node(
                    Code::DuplicateArm,
                    *label,
                    "this label repeats an earlier one",
                );
                return Sig::ERROR;
            }
        }
        let total = match scrutinee.kind {
            Kind::Enum(id) => self
                .vocabulary
                .enum_decl(id)
                .map(|decl| decl.variants.len()),
            Kind::Bool => Some(2),
            Kind::Int
            | Kind::Float
            | Kind::Str
            | Kind::Node
            | Kind::Bare
            | Kind::Any
            | Kind::Error => None,
        };
        let complete = total.is_some_and(|total| seen.len() >= total);
        match (otherwise.is_some(), complete, total.is_some()) {
            (false, false, true) => {
                self.error_at_node(
                    Code::MatchIncomplete,
                    node,
                    "this `match` covers some values only and ends with no `else:`",
                );
                return Sig::ERROR;
            }
            (false, _, false) => {
                self.error_at_node(
                    Code::MatchIncomplete,
                    node,
                    "a `match` on an integer ends with `else:`",
                );
                return Sig::ERROR;
            }
            (true, true, true) => {
                self.error_at_node(
                    Code::RedundantElse,
                    node,
                    "the arms cover every value, so `else:` never applies",
                );
                return Sig::ERROR;
            }
            _ => {}
        }
        let mut values: Vec<(NodeId, Sig)> = Vec::new();
        for (position, (_, value)) in arms.iter().enumerate() {
            let sig = sigs.get(2 + position * 2).copied().unwrap_or(Sig::ERROR);
            values.push((*value, sig));
        }
        if let Some(otherwise) = otherwise {
            let sig = sigs.last().copied().unwrap_or(Sig::ERROR);
            values.push((*otherwise, sig));
        }
        let mut kind = values.first().map_or(Kind::Error, |(_, sig)| sig.kind);
        let mut width = values.first().map_or(Width::Flexible, |(_, sig)| sig.width);
        for (_, sig) in values.iter().skip(1) {
            let Some(unified) = unify_kinds(kind, sig.kind) else {
                self.error_at_node(
                    Code::TypeMismatch,
                    node,
                    "the arms of a `match` hold values of different types",
                );
                return Sig::ERROR;
            };
            kind = unified;
            width = branch_width_of(kind, width, sig.width);
        }
        let Some(family) = self.one_family(node, &values) else {
            return Sig::ERROR;
        };
        let reactive = sigs.iter().any(|sig| sig.reactive);
        if scrutinee.reactive && matches!(width, Width::Flexible) {
            width = Width::Adaptive;
        }
        Sig::of(kind, width, reactive, family)
    }

    fn label(&mut self, label: NodeId, scrutinee: &Sig) -> Option<Label> {
        let op = self.node(label).op.clone();
        let found = match (&op, scrutinee.kind) {
            (Op::Bare(name), Kind::Enum(id)) => {
                self.variants.position(id, name).map(Label::Variant)
            }
            (Op::Bool(value), Kind::Bool) => Some(Label::Bool(*value)),
            (Op::Number(Number::Int(_), suffix), Kind::Int) => {
                let family = match suffix {
                    Suffix::Unit { family, .. } => Some(*family),
                    Suffix::Type(scalar) if !matches!(scalar, Scalar::I32 | Scalar::I64) => {
                        return self.bad_label(label, scrutinee);
                    }
                    Suffix::None | Suffix::Type(_) => None,
                };
                if family.is_some() && family != scrutinee.family {
                    return self.bad_label(label, scrutinee);
                }
                self.exact_label(&op).map(Label::Int)
            }
            _ => None,
        };
        if found.is_none() {
            return self.bad_label(label, scrutinee);
        }
        found
    }

    fn exact_label(&self, op: &Op) -> Option<i128> {
        let Op::Number(Number::Int(value), suffix) = op else {
            return None;
        };
        match suffix {
            Suffix::Unit { family, unit } => {
                let scale = self.vocabulary.family(*family)?.units.get(*unit)?.scale;
                if scale.fract() != 0.0 || scale.abs() >= 1e38 {
                    return None;
                }
                value.checked_mul(scale as i128)
            }
            Suffix::None | Suffix::Type(_) => Some(*value),
        }
    }

    fn bad_label(&mut self, label: NodeId, scrutinee: &Sig) -> Option<Label> {
        let message = match scrutinee.kind {
            Kind::Enum(id) => format!(
                "a label here is a variant of `{}`",
                self.vocabulary
                    .enum_decl(id)
                    .map_or("", |decl| decl.name.as_str())
            ),
            Kind::Bool => "a label here is `true` or `false`".to_string(),
            Kind::Int
            | Kind::Float
            | Kind::Str
            | Kind::Node
            | Kind::Bare
            | Kind::Any
            | Kind::Error => {
                "a label here is an integer literal of the scrutinee's unit family".to_string()
            }
        };
        self.error_at_node(Code::MatchLabel, label, message);
        None
    }
}
