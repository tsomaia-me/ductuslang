use std::collections::BTreeSet;

use symphonyscript_core::Scalar;

use super::branch_width_of::branch_width_of;
use super::operand::operand;
use crate::diagnostics::Code;
use crate::semantics::Internal;
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
        (scrutinee_node, arms, otherwise): (NodeId, &[(NodeId, NodeId)], Option<NodeId>),
        sigs: &[Sig],
    ) -> Result<Sig, Internal> {
        let scrutinee = operand(sigs, 0)?;
        if scrutinee.kind == Kind::Bare {
            self.memo.bare_matches.push(node);
        } else if !matches!(scrutinee.kind, Kind::Enum(_) | Kind::Int | Kind::Bool) {
            self.error_at_node(
                Code::MatchScrutinee,
                scrutinee_node,
                "a `match` selects by an enum value, an integer or a `bool`",
            )?;
            return Ok(Sig::ERROR);
        } else if !self.match_labels(node, arms, otherwise, &scrutinee)? {
            return Ok(Sig::ERROR);
        }
        let mut values: Vec<(NodeId, Sig)> = Vec::new();
        for (position, (_, value)) in arms.iter().enumerate() {
            values.push((*value, operand(sigs, 2 + position * 2)?));
        }
        if let Some(otherwise) = otherwise {
            values.push((otherwise, operand(sigs, 1 + arms.len() * 2)?));
        }
        let &(_, first) = values.first().ok_or(Internal::missed("match arm"))?;
        let mut kind = first.kind;
        let mut width = first.width;
        for (_, sig) in values.iter().skip(1) {
            let Some(unified) = unify_kinds(kind, sig.kind) else {
                self.error_at_node(
                    Code::TypeMismatch,
                    node,
                    "the arms of a `match` hold values of different types",
                )?;
                return Ok(Sig::ERROR);
            };
            kind = unified;
            width = branch_width_of(kind, width, sig.width);
        }
        let Some(family) = self.one_family(node, &values)? else {
            return Ok(Sig::ERROR);
        };
        let reactive = sigs.iter().any(|sig| sig.reactive);
        if scrutinee.reactive && matches!(width, Width::Flexible) {
            width = Width::Adaptive;
        }
        Ok(Sig::of(kind, width, reactive, family))
    }

    pub(in crate::semantics::analysis) fn match_labels(
        &mut self,
        node: NodeId,
        arms: &[(NodeId, NodeId)],
        otherwise: Option<NodeId>,
        scrutinee: &Sig,
    ) -> Result<bool, Internal> {
        let mut seen: BTreeSet<Label> = BTreeSet::new();
        for (label, _) in arms {
            let Some(value) = self.label(*label, scrutinee)? else {
                return Ok(false);
            };
            if !seen.insert(value) {
                self.error_at_node(
                    Code::DuplicateArm,
                    *label,
                    "this label repeats an earlier one",
                )?;
                return Ok(false);
            }
        }
        let total = match scrutinee.kind {
            Kind::Enum(id) => Some(self.enum_decl(id)?.variants.len()),
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
                )?;
                return Ok(false);
            }
            (false, _, false) => {
                self.error_at_node(
                    Code::MatchIncomplete,
                    node,
                    "a `match` on an integer ends with `else:`",
                )?;
                return Ok(false);
            }
            (true, true, true) => {
                self.error_at_node(
                    Code::RedundantElse,
                    node,
                    "the arms cover every value, so `else:` never applies",
                )?;
                return Ok(false);
            }
            _ => {}
        }
        Ok(true)
    }

    fn label(&mut self, label: NodeId, scrutinee: &Sig) -> Result<Option<Label>, Internal> {
        let op = self.node(label)?.op.clone();
        let found = match (&op, scrutinee.kind) {
            (Op::Bare(name), Kind::Enum(id)) => {
                self.variants.position(id, name)?.map(Label::Variant)
            }
            (Op::Bool(value), Kind::Bool) => Some(Label::Bool(*value)),
            (Op::Number(Number::Int(value), suffix), Kind::Int) => {
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
                self.exact_label(*value, *suffix)?.map(Label::Int)
            }
            _ => None,
        };
        if found.is_none() {
            return self.bad_label(label, scrutinee);
        }
        Ok(found)
    }

    fn exact_label(&self, value: i128, suffix: Suffix) -> Result<Option<i128>, Internal> {
        Ok(match suffix {
            Suffix::Unit { family, unit } => {
                let scale = self.unit_decl(family, unit)?.scale;
                if scale.fract() != 0.0 || scale.abs() >= 1e38 {
                    return Ok(None);
                }
                value.checked_mul(scale as i128)
            }
            Suffix::None | Suffix::Type(_) => Some(value),
        })
    }

    fn bad_label(&mut self, label: NodeId, scrutinee: &Sig) -> Result<Option<Label>, Internal> {
        let message = match scrutinee.kind {
            Kind::Enum(id) => format!(
                "a label here is a variant of `{}`",
                self.enum_decl(id)?.name
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
        self.error_at_node(Code::MatchLabel, label, message)?;
        Ok(None)
    }
}
