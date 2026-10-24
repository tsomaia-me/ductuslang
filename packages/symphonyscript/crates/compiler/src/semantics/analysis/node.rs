use symphonyscript_core::{AttrId, BinaryOp, Builtin, EnumId, Scalar, UnaryOp, Value};

use super::ids::{BindingId, NodeId, ScopeId, WireId};
use super::item_ref::ItemRef;
use super::val::Val;
use crate::semantics::ir::{FnId, Loc, Number, Suffix};

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Piece {
    Text(String),
    Value(NodeId),
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Op {
    Number(Number, Suffix),
    Const(Value),
    Bool(bool),
    Str(String),
    Concat(Vec<Piece>),
    Variant(EnumId, u16),
    Bare(String),
    Pi,
    NodeRef(ItemRef),
    Read(BindingId),
    Use(NodeId),
    Arg(u32),
    Unary(UnaryOp, NodeId),
    Binary(BinaryOp, NodeId, NodeId),
    If(NodeId, NodeId, NodeId),
    Match {
        scrutinee: NodeId,
        arms: Vec<(NodeId, NodeId)>,
        otherwise: Option<NodeId>,
    },
    Pick {
        index: NodeId,
        items: Vec<NodeId>,
    },
    Convert(Scalar, NodeId),
    Annotate(Scalar, NodeId),
    Builtin(Builtin, Vec<NodeId>),
    Recursive {
        function: FnId,
        args: Vec<Option<Val>>,
    },
    ToRead {
        wire: WireId,
        attr: String,
        attrs: Vec<(ItemRef, AttrId)>,
    },
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Node {
    pub op: Op,
    pub loc: Loc,
    pub scope: ScopeId,
}

impl Op {
    pub fn operands(&self) -> Vec<NodeId> {
        match self {
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Read(_)
            | Op::Arg(_)
            | Op::ToRead { .. } => Vec::new(),
            Op::Concat(pieces) => pieces
                .iter()
                .filter_map(|piece| match piece {
                    Piece::Text(_) => None,
                    Piece::Value(node) => Some(*node),
                })
                .collect(),
            Op::Unary(_, operand)
            | Op::Convert(_, operand)
            | Op::Annotate(_, operand)
            | Op::Use(operand) => vec![*operand],
            Op::Binary(_, lhs, rhs) => vec![*lhs, *rhs],
            Op::If(condition, then, otherwise) => vec![*condition, *then, *otherwise],
            Op::Match {
                scrutinee,
                arms,
                otherwise,
            } => {
                let mut operands = vec![*scrutinee];
                for (label, value) in arms {
                    operands.push(*label);
                    operands.push(*value);
                }
                operands.extend(otherwise.iter().copied());
                operands
            }
            Op::Pick { index, items } => {
                let mut operands = vec![*index];
                operands.extend(items.iter().copied());
                operands
            }
            Op::Builtin(_, args) => args.clone(),
            Op::Recursive { args, .. } => args.iter().flatten().flat_map(Val::nodes).collect(),
        }
    }
}
