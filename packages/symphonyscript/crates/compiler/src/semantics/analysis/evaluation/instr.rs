use std::rc::Rc;

use symphonyscript_core::{AttrId, BinaryOp, NativeId};

use super::arg_fact::ArgFact;
use super::concat_part::ConcatPart;
use super::fetch_entry::FetchEntry;
use super::lay::Lay;
use super::operand_post::OperandPost;
use crate::diagnostics::Code;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::for_node::ForNode;
use crate::semantics::analysis::frame_read::FrameRead;
use crate::semantics::analysis::ids::{NodeId, SlotId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::shape_need::ShapeNeed;
use crate::semantics::builtin::Builtin;
use crate::semantics::ir::FnId;

#[derive(Debug)]
pub(crate) enum Instr {
    Push(CVal),
    Arg(u32),
    Slot(SlotId),
    Child(NodeId, EvalTy),
    Fetch(Rc<[FetchEntry]>),
    Narrow(EvalTy),
    Fault(Code, NodeId, Rc<str>),
    Draw(Code, NodeId, Rc<str>),
    Convert(EvalTy),
    Fit(NodeId, EvalTy),
    Shape(NodeId, Rc<ShapeNeed>),
    Scaled(NodeId, f64, EvalTy),
    ScaledInto(f64, EvalTy, EvalTy),
    BeforeUnit(NodeId, EvalTy),
    NarrowFloat(u32),
    Unless(u32, u32),
    Jump(u32),
    BailIfError(u32),
    Pick(NodeId, Rc<[u32]>, u32),
    MatchLabel(u32),
    Drop,
    IfGlobal(u32),
    Element(NodeId, NodeId, Lay),
    Array(NodeId, u32, Lay),
    Not,
    Neg(NodeId, EvalTy),
    Logic(BinaryOp, Lay),
    Arith(BinaryOp, NodeId, EvalTy, Lay),
    Compare(
        BinaryOp,
        Option<(NodeId, EvalTy)>,
        Option<(NodeId, EvalTy)>,
        Lay,
    ),
    FlexPair(u32),
    StrAdd(Lay),
    IntDivFloat(NodeId, EvalTy, bool, Lay),
    Div(OperandPost, OperandPost, Lay),
    FromEnd(NodeId, u32),
    DeferredCheck(NodeId, u32, Lay, bool),
    DeferredSum(NodeId, u32, Lay, bool),
    ConvertTo(EvalTy),
    Join(NodeId, Lay),
    Slice(NodeId, Option<(NodeId, bool)>, Option<(NodeId, bool)>, Lay),
    Length(NodeId),
    Bytes(NodeId),
    Capacity(NodeId, u32),
    Rand(Rc<[NodeId]>, Lay),
    Builtin(NodeId, Builtin, EvalTy, Rc<[OperandPost]>, Lay),
    Concat(Rc<[ConcatPart]>, Lay),
    FitAnd(NodeId, EvalTy),
    Gate(u32),
    Native(NativeId, u32, Lay),
    CallWith(NodeId, EvalTy, FnId, Rc<[ArgFact]>, u32, Lay),
    LoopStart(NodeId, EvalTy, u32),
    ForCheck(bool, Lay, u32),
    ForRun(NodeId, Rc<ForNode>, EvalTy, Lay),
    FrameRead(Rc<FrameRead>, EvalTy, bool),
    ToRead(WireId, ItemRef, AttrId, EvalTy),
}
