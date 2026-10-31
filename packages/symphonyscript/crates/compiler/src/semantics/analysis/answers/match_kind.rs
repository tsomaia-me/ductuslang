use std::collections::BTreeSet;

use symphonyscript_core::Scalar;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::signature::branch_width_of;
use crate::semantics::analysis::signature::operand;
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
    pub(in crate::semantics::analysis) fn match_sig(
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
        let typed = self.scrutinee_scalar(node, scrutinee)?;
        for (label, _) in arms {
            let Some(value) = self.label(*label, scrutinee, typed)? else {
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

    fn scrutinee_scalar(&self, node: NodeId, scrutinee: &Sig) -> Result<Option<Scalar>, Internal> {
        if let Width::Fixed(scalar) | Width::Widened(scalar) = scrutinee.width {
            return Ok(Some(scalar));
        }
        let Op::Match {
            scrutinee: scrutinee_node,
            ..
        } = self.node(node)?.op
        else {
            return Ok(None);
        };
        let Op::Read(binding) = self.node(self.through_uses(scrutinee_node)?)?.op else {
            return Ok(None);
        };
        let record = self.binding(binding)?;
        Ok(match (record.annotation, &record.kind) {
            (Some(annotation), _) => Some(annotation),
            (None, BindingKind::Input { .. }) if scrutinee.kind == Kind::Int => Some(Scalar::I64),
            (None, _) => None,
        })
    }

    fn label(
        &mut self,
        label: NodeId,
        scrutinee: &Sig,
        typed: Option<Scalar>,
    ) -> Result<Option<Label>, Internal> {
        let op = self.node(label)?.op.clone();
        let found = match &op {
            Op::Bare(name) => match scrutinee.kind {
                Kind::Enum(id) => self.variants.position(id, name)?.map(Label::Variant),
                Kind::Int
                | Kind::Float
                | Kind::Bool
                | Kind::Str
                | Kind::Node
                | Kind::Bare
                | Kind::Any
                | Kind::Error => None,
            },
            Op::Bool(value) => (scrutinee.kind == Kind::Bool).then_some(Label::Bool(*value)),
            Op::Number(Number::Int(value), suffix) if scrutinee.kind == Kind::Int => {
                let family = match suffix {
                    Suffix::Unit { family, .. } => Some(*family),
                    Suffix::Type(scalar)
                        if typed.is_some_and(|typed| typed != *scalar)
                            || !matches!(scalar, Scalar::U8 | Scalar::I32 | Scalar::I64) =>
                    {
                        return self.bad_label(label, scrutinee, typed);
                    }
                    Suffix::None | Suffix::Type(_) => None,
                };
                if family.is_some() && family != scrutinee.family {
                    return self.bad_label(label, scrutinee, typed);
                }
                self.exact_label(*value, *suffix)?.map(Label::Int)
            }
            Op::Number(..)
            | Op::Const(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Read(_)
            | Op::Use(_)
            | Op::Chosen(_)
            | Op::Arg(_)
            | Op::Slot(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::If(..)
            | Op::Match { .. }
            | Op::Pick { .. }
            | Op::Element { .. }
            | Op::Array(_)
            | Op::Convert(..)
            | Op::Annotate(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::For(_)
            | Op::ToRead { .. }
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::History { .. }
            | Op::Stepped(_)
            | Op::Native { .. } => None,
        };
        if found.is_none() {
            return self.bad_label(label, scrutinee, typed);
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

    fn bad_label(
        &mut self,
        label: NodeId,
        scrutinee: &Sig,
        typed: Option<Scalar>,
    ) -> Result<Option<Label>, Internal> {
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
            | Kind::Error => match typed {
                Some(scalar) => format!(
                    "a label here is an integer literal of the scrutinee's type `{}` and its unit family",
                    self.scalar_name(scalar)?
                ),
                None => {
                    "a label here is an integer literal of the scrutinee's unit family".to_string()
                }
            },
        };
        self.error_at_node(Code::MatchLabel, label, message)?;
        Ok(None)
    }
}
