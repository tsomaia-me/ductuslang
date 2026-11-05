use super::reason::Reason;
use super::slice::{LANDED, Slice};
use crate::semantics::ir::Suffix;
use crate::semantics::ir::flat::{
    ABSENT, AttrForm, CellRec, FlatIr, FnRec, ForBlockRec, ForRec, ItemRec, NONE, Op, TopRec,
    WireRec,
};
use crate::semantics::operators::EvalTy;

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
        Op::TyScalar => Slice::One,
        Op::TyArray => Slice::One,
        Op::TyName | Op::TyVar => Slice::Pending,
        Op::To => Slice::Two,
        Op::Field => Slice::Two,
        Op::Past | Op::Stepped => Slice::Four,
        Op::Variant => Slice::Five,
        Op::Bad => Slice::One,
        Op::Str | Op::Match | Op::MethodCall | Op::NamedMethodCall => Slice::DefaultFlip,
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
        if let TopRec::Placement(item) = top
            && matches!(ir.items.get(*item as usize), Some(ItemRec::Wire(_)))
            && !landed(top_wire_slice())
        {
            return false;
        }
        if let TopRec::Let(at) = top
            && ir
                .lets
                .get(*at as usize)
                .is_none_or(|rec| rec.is_match || !let_annotation(ir, rec.binder))
        {
            return false;
        }
        if let TopRec::Pattern(at) = top
            && ir.patterns.get(*at as usize).is_none_or(|rec| {
                rec.regions.0 != rec.regions.1
                    || rec.skipped_default
                    || rec.faulty_line
                    || !plain_block(ir, rec.body)
            })
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
    let members = line_members(ir);
    let in_loops = loop_children(ir);
    ir.fors.iter().all(|rec| plain_fold(ir, rec))
        && ir.cells.iter().all(|rec| plain_cell(ir, rec))
        && ir
            .items
            .iter()
            .enumerate()
            .all(|(at, item)| item_landed(ir, item, (&members, &in_loops), at))
}

fn annotated(ir: &FlatIr, binder: u32) -> bool {
    ir.binders
        .get(binder as usize)
        .is_none_or(|binder| binder.annotation != NONE)
}

fn line_members(ir: &FlatIr) -> Vec<u32> {
    let mut members = Vec::new();
    for item in &ir.items {
        if let ItemRec::Line(range) = item {
            members.extend(
                ir.members
                    .get(range.0 as usize..range.1 as usize)
                    .unwrap_or_default(),
            );
        }
    }
    members.sort_unstable();
    members
}

fn loop_children(ir: &FlatIr) -> Vec<u32> {
    let mut found = Vec::new();
    for item in &ir.items {
        let ItemRec::For(rec) = item else {
            continue;
        };
        let Some(block) = ir.blocks.get(rec.body as usize) else {
            continue;
        };
        let mut items: Vec<u32> = ir
            .block_items
            .get(block.items.0 as usize..block.items.1 as usize)
            .unwrap_or_default()
            .to_vec();
        while let Some(child) = items.pop() {
            found.push(child);
            if let Some(ItemRec::Line(range)) = ir.items.get(child as usize) {
                items.extend(
                    ir.members
                        .get(range.0 as usize..range.1 as usize)
                        .unwrap_or_default(),
                );
            }
        }
    }
    found.sort_unstable();
    found
}

fn item_landed(
    ir: &FlatIr,
    item: &ItemRec,
    (members, in_loops): (&[u32], &[u32]),
    at: usize,
) -> bool {
    if !landed(item_slice(item)) {
        return false;
    }
    if let ItemRec::Wire(wire) = item {
        let in_loop = u32::try_from(at).is_ok_and(|at| in_loops.binary_search(&at).is_ok());
        return wire_landed(ir, wire, in_loop);
    }
    if let ItemRec::For(rec) = item {
        return loop_landed(ir, rec);
    }
    if matches!(item, ItemRec::Line(_)) {
        return u32::try_from(at).is_ok_and(|at| ir.block_items.contains(&at));
    }
    let ItemRec::Placement(placement) = item else {
        return true;
    };
    if placement.gate != NONE
        || placement.flags.0 != placement.flags.1
        || placement.attrs_cut
        || placement.skipped_block.is_some()
        || (placement.inline
            && u32::try_from(at).map_or(true, |at| members.binary_search(&at).is_err()))
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

fn loop_landed(ir: &FlatIr, rec: &ForBlockRec) -> bool {
    let head = rec.head;
    let names = head.names.1.saturating_sub(head.names.0);
    (1..=2).contains(&names)
        && (head.names.0..head.names.1).all(|binder| scalar_annotation(ir, binder))
        && head.iterable.0 != NONE
        && (!head.range || head.iterable.1 != NONE)
        && ir.blocks.get(rec.body as usize).is_some_and(|block| {
            block.members.0 == block.members.1
                && block.regions.0 == block.regions.1
                && block.lets.0 == block.lets.1
        })
}

fn value_entries(ir: &FlatIr, attrs: (u32, u32)) -> bool {
    ir.attr_entries
        .get(attrs.0 as usize..attrs.1 as usize)
        .unwrap_or_default()
        .iter()
        .all(|entry| matches!(entry.form, AttrForm::Value(_)))
}

pub(crate) fn top_wire_slice() -> Slice {
    Slice::Pending
}

pub(crate) fn destination_slice(op: Op) -> Slice {
    match op {
        Op::Name | Op::If | Op::Index | Op::Array | Op::Field => Slice::Two,
        Op::Int | Op::Float | Op::Bool => Slice::Two,
        Op::Neg | Op::Add | Op::Sub | Op::Mul => Slice::Pending,
        Op::Div | Op::IntDiv | Op::Rem | Op::Pow => Slice::Pending,
        Op::Not | Op::And | Op::Or => Slice::Pending,
        Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot => Slice::Pending,
        Op::IndexFromEnd | Op::Slice | Op::Convert | Op::For => Slice::Pending,
        Op::Call | Op::NamedCall | Op::TyScalar | Op::TyArray | Op::TyName | Op::TyVar => {
            Slice::Pending
        }
        Op::To | Op::Past | Op::Stepped | Op::Variant | Op::Bad => Slice::Pending,
        Op::Str | Op::Match | Op::MethodCall | Op::NamedMethodCall => Slice::Pending,
    }
}

pub(crate) fn destination_candidates(ir: &FlatIr, root: u32) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let (op, a, b, c) = ir.node(node).ok()?;
        if !landed(destination_slice(op)) {
            return None;
        }
        if op == Op::If {
            stack.extend([c, b]);
        } else if op == Op::Index {
            let (base, start, count, _) = ir.node(a).ok()?;
            if base != Op::Array {
                stack.push(a);
                continue;
            }
            if count == 0 {
                return None;
            }
            for at in (start..start + count).rev() {
                stack.push(ir.kid(at).ok()?);
            }
        } else if op == Op::Field && !instance_path(ir, node) {
            return None;
        } else {
            out.push(node);
        }
    }
    Some(out)
}

fn instance_path(ir: &FlatIr, field: u32) -> bool {
    let mut node = field;
    while let Ok((Op::Field, receiver, _, _)) = ir.node(node) {
        node = receiver;
    }
    matches!(ir.node(node), Ok((Op::Name, ..)))
}

fn wire_landed(ir: &FlatIr, wire: &WireRec, in_loop: bool) -> bool {
    wire.gate == NONE
        && wire.flags.0 == wire.flags.1
        && (wire.as_name == NONE || in_loop)
        && !wire.attrs_cut
        && value_entries(ir, wire.attrs)
        && destination_candidates(ir, wire.destination).is_some()
}

pub(crate) fn reason_slice(reason: Reason) -> Slice {
    match reason {
        Reason::Member => Slice::DefaultFlip,
        Reason::ArrayAttribute => Slice::Pending,
        Reason::PreludeItems | Reason::NameOfInstance | Reason::NameOfVocabulary => {
            Slice::DefaultFlip
        }
        Reason::StringAttribute => Slice::DefaultFlip,
        Reason::PatternOrWirePlacement => Slice::Pending,
        Reason::CellGroup => Slice::Four,
        Reason::FunctionValue | Reason::Recursion => Slice::DefaultFlip,
        Reason::LocalCycle | Reason::LocalClash => Slice::Pending,
        Reason::AnnotatedCall => Slice::Pending,
        Reason::Pending => Slice::One,
        Reason::ReactiveFor => Slice::Pending,
        Reason::DestinationValue | Reason::ReactiveReference | Reason::WireKind => Slice::Pending,
        Reason::BareVariant | Reason::UnitAttribute | Reason::EnumAttribute => Slice::Five,
        Reason::LoopHead | Reason::BodyItem | Reason::PatternUse => Slice::Pending,
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
        && scalar_annotation(ir, rec.binder)
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
            .all(|binder| scalar_annotation(ir, binder))
        && (rec.lets.0..rec.lets.1).all(|at| {
            ir.lets
                .get(at as usize)
                .is_some_and(|rec| !rec.is_match && !annotated(ir, rec.binder))
        })
}

fn let_annotation(ir: &FlatIr, binder: u32) -> bool {
    let Some(rec) = ir.binders.get(binder as usize) else {
        return false;
    };
    if rec.annotation == NONE {
        return true;
    }
    let mut node = rec.annotation;
    while let Ok((Op::TyArray, inner, length, known)) = ir.node(node) {
        if known != 1 || !(1..1 << 31).contains(&length) {
            return false;
        }
        node = inner;
    }
    scalar_node(ir, node)
}

fn scalar_annotation(ir: &FlatIr, binder: u32) -> bool {
    ir.binders
        .get(binder as usize)
        .is_some_and(|binder| binder.annotation == NONE || scalar_node(ir, binder.annotation))
}

fn scalar_node(ir: &FlatIr, node: u32) -> bool {
    let Ok((op, code, _, _)) = ir.node(node) else {
        return false;
    };
    op == Op::TyScalar
        && super::width::target_ty(code).is_some_and(|ty| {
            matches!(
                ty,
                EvalTy::U8 | EvalTy::I32 | EvalTy::I64 | EvalTy::F32 | EvalTy::F64 | EvalTy::Bool
            )
        })
}

fn plain_block(ir: &FlatIr, body: u32) -> bool {
    ir.blocks.get(body as usize).is_some_and(|block| {
        block.items.0 != block.items.1
            && block.members.0 == block.members.1
            && block.regions.0 == block.regions.1
            && block.lets.0 == block.lets.1
    })
}
