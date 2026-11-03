use super::slice::{LANDED, Slice};
use crate::semantics::ir::Suffix;
use crate::semantics::ir::flat::{ABSENT, AttrForm, FlatIr, ItemRec, NONE, Op, TopRec};

pub(crate) fn op_slice(op: Op) -> Slice {
    match op {
        Op::Int | Op::Float | Op::Bool | Op::Name => Slice::One,
        Op::Neg | Op::Add | Op::Sub | Op::Mul => Slice::One,
        Op::Array | Op::Index => Slice::One,
        Op::Div | Op::IntDiv | Op::Rem | Op::Pow => Slice::Pending,
        Op::Not | Op::And | Op::Or => Slice::Pending,
        Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot => Slice::Pending,
        Op::If | Op::IndexFromEnd | Op::Slice | Op::Convert | Op::For => Slice::Pending,
        Op::Call | Op::NamedCall => Slice::Pending,
        Op::TyScalar | Op::TyName | Op::TyArray | Op::TyVar => Slice::Pending,
        Op::To => Slice::Two,
        Op::Past | Op::Stepped => Slice::Four,
        Op::Variant => Slice::Five,
        Op::Str | Op::Match | Op::Field | Op::MethodCall | Op::NamedMethodCall | Op::Bad => {
            Slice::DefaultFlip
        }
    }
}

pub(crate) fn top_slice(top: TopRec) -> Slice {
    match top {
        TopRec::Let(_) | TopRec::Placement(_) => Slice::One,
        TopRec::Cell(_) | TopRec::Fn(_) => Slice::Pending,
        TopRec::Pattern(_) => Slice::Three,
        TopRec::Placeholder { .. } | TopRec::Drawn(_) | TopRec::Skip => Slice::DefaultFlip,
    }
}

pub(crate) fn item_slice(item: &ItemRec) -> Slice {
    match item {
        ItemRec::Placement(_) => Slice::One,
        ItemRec::Wire(_) => Slice::Two,
        ItemRec::For(_) | ItemRec::Line(_) => Slice::Three,
        ItemRec::Bundle(_) | ItemRec::When(_) | ItemRec::Paren(_) => Slice::DefaultFlip,
    }
}

fn landed(slice: Slice) -> bool {
    slice <= LANDED
}

fn literal_landed(ir: &FlatIr, suffix: u32) -> bool {
    suffix == NONE || matches!(ir.suffixes.get(suffix as usize), Some(Suffix::None))
}

pub(crate) fn covers(prelude: &FlatIr, ir: &FlatIr) -> bool {
    if !prelude.top.is_empty() || !ir.drawn.is_empty() || !ir.regions.is_empty() {
        return false;
    }
    if ir.names.iter().any(|name| name.faulty) {
        return false;
    }
    for (local, op) in ir.op.iter().enumerate() {
        if !landed(op_slice(*op)) {
            return false;
        }
        if matches!(op, Op::Int | Op::Float)
            && !ir
                .b
                .get(local)
                .is_some_and(|suffix| literal_landed(ir, *suffix))
        {
            return false;
        }
    }
    for top in &ir.top {
        if !landed(top_slice(*top)) {
            return false;
        }
        if let TopRec::Let(at) = top
            && ir
                .lets
                .get(*at as usize)
                .is_none_or(|rec| rec.is_match || rec.faulty_line || annotated(ir, rec.binder))
        {
            return false;
        }
    }
    ir.items.iter().all(|item| item_landed(ir, item))
}

fn annotated(ir: &FlatIr, binder: u32) -> bool {
    ir.binders
        .get(binder as usize)
        .is_none_or(|binder| binder.annotation != NONE)
}

fn item_landed(ir: &FlatIr, item: &ItemRec) -> bool {
    if !landed(item_slice(item)) {
        return false;
    }
    let ItemRec::Placement(placement) = item else {
        return true;
    };
    if placement.gate != NONE
        || placement.flags.0 != placement.flags.1
        || placement.attrs_cut
        || placement.skipped_block.is_some()
        || placement.inline
    {
        return false;
    }
    let entries = ir
        .attr_entries
        .get(placement.attrs.0 as usize..placement.attrs.1 as usize)
        .unwrap_or_default();
    if entries
        .iter()
        .any(|entry| !matches!(entry.form, AttrForm::Value(_)))
    {
        return false;
    }
    if placement.body == NONE {
        return true;
    }
    if placement.body == ABSENT {
        return false;
    }
    ir.blocks.get(placement.body as usize).is_some_and(|block| {
        block.members.0 == block.members.1
            && block.regions.0 == block.regions.1
            && block.lets.0 == block.lets.1
    })
}
