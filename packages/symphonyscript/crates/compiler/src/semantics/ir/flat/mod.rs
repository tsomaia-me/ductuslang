mod adapter;
#[cfg(test)]
mod adapter_tests;
mod attr_entry_rec;
mod attr_form;
mod binder;
mod binder_kind;
mod block_rec;
mod builder;
mod bundle_for_rec;
mod bundle_rec;
mod cell_rec;
#[cfg(test)]
mod cost_tests;
mod drawn;
mod dropped;
mod field_rec;
mod file_rec;
mod flag_rec;
mod flat_ir;
mod float_entry;
mod fn_rec;
mod for_block_rec;
mod for_rec;
mod item_rec;
mod iteration;
mod let_rec;
mod name_kind;
mod name_range;
#[cfg(test)]
mod names_tests;
mod op;
mod part;
mod pattern_rec;
mod placement_rec;
mod sym;
mod symbol_table;
mod symbols;
mod top_rec;
mod type_rec;
mod when_rec;
mod wire_rec;

pub(crate) use adapter::{adapt_file, adapt_file_mapped};
pub(crate) use attr_form::AttrForm;
pub(crate) use builder::{SCALARS, build_flat_file};
pub(crate) use cell_rec::CellRec;
pub(crate) use flat_ir::FlatIr;
pub(crate) use fn_rec::FnRec;
pub(crate) use for_rec::ForRec;
pub(crate) use item_rec::ItemRec;
pub(crate) use op::Op;
pub(crate) use placement_rec::PlacementRec;
pub(crate) use sym::Sym;
pub(crate) use top_rec::TopRec;

use symphonyscript_core::BinaryOp;

pub(crate) const NONE: u32 = u32::MAX;
pub(crate) const ABSENT: u32 = u32::MAX - 1;

pub(crate) fn index(length: usize) -> u32 {
    u32::try_from(length).unwrap_or(u32::MAX)
}

pub(crate) fn digits_value(text: &str) -> Option<u64> {
    let digits: String = text.chars().filter(|c| *c != '_').collect();
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(digits.parse::<u64>().unwrap_or(u64::MAX))
}

pub(crate) fn length_value(text: &str) -> Option<u32> {
    digits_value(text).map(|value| u32::try_from(value).unwrap_or(u32::MAX))
}

const BINARY: [(BinaryOp, Op); 15] = [
    (BinaryOp::Add, Op::Add),
    (BinaryOp::Sub, Op::Sub),
    (BinaryOp::Mul, Op::Mul),
    (BinaryOp::Div, Op::Div),
    (BinaryOp::IntDiv, Op::IntDiv),
    (BinaryOp::Rem, Op::Rem),
    (BinaryOp::Pow, Op::Pow),
    (BinaryOp::Is, Op::Is),
    (BinaryOp::IsNot, Op::IsNot),
    (BinaryOp::Lt, Op::Lt),
    (BinaryOp::Le, Op::Le),
    (BinaryOp::Gt, Op::Gt),
    (BinaryOp::Ge, Op::Ge),
    (BinaryOp::And, Op::And),
    (BinaryOp::Or, Op::Or),
];

pub(crate) fn binary_op_code(op: BinaryOp) -> Op {
    BINARY
        .iter()
        .find(|(known, _)| *known == op)
        .map_or(Op::Bad, |(_, code)| *code)
}

pub(crate) fn binary_of(op: Op) -> Option<BinaryOp> {
    BINARY
        .iter()
        .find(|(_, code)| *code == op)
        .map(|(known, _)| *known)
}
