use super::reason::Reason;
use super::slice::{LANDED, Slice};
use crate::semantics::ir::Suffix;
use crate::semantics::ir::flat::{
    ABSENT, AttrForm, CellRec, FlatIr, FnRec, ForRec, ItemRec, NONE, Op, TopRec,
};

pub(crate) fn op_slice(op: Op) -> Slice {
    match op {
        Op::Int | Op::Float | Op::Bool | Op::Name => Slice::One,
        Op::Neg | Op::Add | Op::Sub | Op::Mul => Slice::One,
        Op::Array | Op::Index => Slice::One,
        Op::Div | Op::IntDiv | Op::Rem | Op::Pow => Slice::One,
        Op::Not | Op::And | Op::Or => Slice::One,
        Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot => Slice::One,
        Op::If => Slice::One,
        Op::IndexFromEnd | Op::Slice => Slice::One,
        Op::Convert => Slice::One,
        Op::For => Slice::One,
        Op::Call => Slice::One,
        Op::NamedCall => Slice::One,
        Op::TyScalar | Op::TyName | Op::TyArray | Op::TyVar => Slice::Pending,
        Op::To => Slice::Two,
        Op::Past | Op::Stepped => Slice::Four,
        Op::Variant => Slice::Five,
        Op::Bad => Slice::One,
        Op::Str | Op::Match | Op::Field | Op::MethodCall | Op::NamedMethodCall => {
            Slice::DefaultFlip
        }
    }
}

pub(crate) fn top_slice(top: TopRec) -> Slice {
    match top {
        TopRec::Let(_) | TopRec::Placement(_) => Slice::One,
        TopRec::Cell(_) => Slice::One,
        TopRec::Fn(_) => Slice::One,
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

pub(crate) fn covers(ir: &FlatIr) -> bool {
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
                .is_none_or(|rec| rec.is_match || annotated(ir, rec.binder))
        {
            return false;
        }
        if let TopRec::Fn(at) = top
            && ir
                .fns
                .get(*at as usize)
                .is_none_or(|rec| !plain_function(ir, rec))
        {
            return false;
        }
        if let TopRec::Cell(at) = top
            && ir
                .cells
                .get(*at as usize)
                .is_none_or(|rec| !plain_cell(ir, rec))
        {
            return false;
        }
    }
    ir.fors.iter().all(|rec| plain_fold(ir, rec))
        && ir.items.iter().all(|item| item_landed(ir, item))
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

pub(crate) fn reason_slice(reason: Reason) -> Slice {
    match reason {
        Reason::ChildName => Slice::DefaultFlip,
        Reason::ArrayAttribute => Slice::Pending,
        Reason::PreludeItems | Reason::NameOfInstance | Reason::NameOfVocabulary => {
            Slice::DefaultFlip
        }
        Reason::StringAttribute => Slice::DefaultFlip,
        Reason::PatternOrWirePlacement => Slice::Three,
        Reason::CellGroup => Slice::Four,
        Reason::FunctionValue | Reason::Recursion => Slice::DefaultFlip,
        Reason::LocalCycle | Reason::LocalClash => Slice::Pending,
        Reason::NestedScope => Slice::Pending,
        Reason::ReactiveFor => Slice::Three,
        Reason::BareVariant | Reason::UnitAttribute | Reason::EnumAttribute => Slice::Five,
    }
}

pub(crate) fn declines(reason: Reason) -> bool {
    !landed(reason_slice(reason))
}

fn plain_cell(ir: &FlatIr, rec: &CellRec) -> bool {
    let host_fed = rec.rule == NONE && rec.init != NONE;
    let derived = rec.rule != NONE && rec.init == NONE;
    (host_fed || derived)
        && rec.clock.0 == rec.clock.1
        && rec.capacity.is_none()
        && !annotated(ir, rec.binder)
}

fn plain_function(ir: &FlatIr, rec: &FnRec) -> bool {
    rec.regions.0 == rec.regions.1
        && (rec.params.0..rec.params.1).all(|binder| {
            ir.binders
                .get(binder as usize)
                .is_some_and(|binder| binder.annotation == NONE && binder.default == NONE)
        })
        && (rec.lets.0..rec.lets.1).all(|at| {
            ir.lets
                .get(at as usize)
                .is_some_and(|rec| !rec.is_match && !annotated(ir, rec.binder))
        })
}

fn plain_fold(ir: &FlatIr, rec: &ForRec) -> bool {
    let names = rec.names.1.saturating_sub(rec.names.0);
    (1..=2).contains(&names)
        && rec.accumulator != NONE
        && (rec.names.0..rec.names.1)
            .chain([rec.accumulator])
            .all(|binder| !annotated(ir, binder))
        && (rec.lets.0..rec.lets.1).all(|at| {
            ir.lets
                .get(at as usize)
                .is_some_and(|rec| !rec.is_match && !annotated(ir, rec.binder))
        })
}
