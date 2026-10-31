use symphonyscript_core::{BinaryOp, Scalar, UnaryOp};

use super::args::Args;
use super::for_expr::ForExpr;
use super::history_form::HistoryForm;
use super::ident::Ident;
use super::ids::ExprId;
use super::match_arm::MatchArm;
use super::number::Number;
use super::slice_bound::SliceBound;
use super::str_part::StrPart;
use super::suffix::Suffix;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum ExprKind {
    Error,
    Number(Number, Suffix),
    Bool(bool),
    Str(Vec<StrPart>),
    Name(String),
    Variant {
        enumeration: Ident,
        variant: Ident,
    },
    To,
    Convert(Scalar, ExprId),
    Array(Vec<ExprId>),
    Field(ExprId, Ident),
    Index(ExprId, ExprId),
    IndexFromEnd(ExprId, ExprId),
    Slice {
        base: ExprId,
        lower: Option<SliceBound>,
        upper: Option<SliceBound>,
    },
    History {
        target: Ident,
        offset: ExprId,
        fallback: Option<ExprId>,
        form: HistoryForm,
    },
    Stepped(Ident),
    Call(ExprId, Args),
    Unary(UnaryOp, ExprId),
    Binary(BinaryOp, ExprId, ExprId),
    If(ExprId, ExprId, ExprId),
    Match {
        scrutinee: ExprId,
        arms: Vec<MatchArm>,
        otherwise: Option<ExprId>,
    },
    For(Box<ForExpr>),
}

impl ExprKind {
    pub fn children(&self) -> Vec<ExprId> {
        match self {
            ExprKind::Error
            | ExprKind::Number(..)
            | ExprKind::Bool(_)
            | ExprKind::Name(_)
            | ExprKind::Variant { .. }
            | ExprKind::To
            | ExprKind::Stepped(_) => Vec::new(),
            ExprKind::Str(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    StrPart::Text(_) => None,
                    StrPart::Hole(expr) => Some(*expr),
                })
                .collect(),
            ExprKind::Convert(_, operand)
            | ExprKind::Field(operand, _)
            | ExprKind::Unary(_, operand) => vec![*operand],
            ExprKind::Array(elements) => elements.clone(),
            ExprKind::Index(base, index) | ExprKind::IndexFromEnd(base, index) => {
                vec![*base, *index]
            }
            ExprKind::Slice { base, lower, upper } => {
                let mut children = vec![*base];
                children.extend(lower.iter().chain(upper.iter()).map(|bound| bound.value));
                children
            }
            ExprKind::History {
                offset, fallback, ..
            } => {
                let mut children = vec![*offset];
                children.extend(fallback.iter().copied());
                children
            }
            ExprKind::Call(callee, args) => {
                let mut children = vec![*callee];
                children.extend(args.values());
                children
            }
            ExprKind::Binary(_, lhs, rhs) => vec![*lhs, *rhs],
            ExprKind::If(condition, then, otherwise) => vec![*condition, *then, *otherwise],
            ExprKind::Match {
                scrutinee,
                arms,
                otherwise,
            } => {
                let mut children = vec![*scrutinee];
                for arm in arms {
                    children.push(arm.label);
                    children.push(arm.value);
                }
                children.extend(otherwise.iter().copied());
                children
            }
            ExprKind::For(for_expr) => for_expr.children(),
        }
    }
}
