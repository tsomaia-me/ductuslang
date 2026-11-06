use crate::semantics::ir::flat::{FlatIr, Op};

pub(crate) fn dot_calls(ir: &FlatIr) -> Option<(FlatIr, Vec<u32>)> {
    if !ir.op.contains(&Op::MethodCall) && !ir.op.contains(&Op::NamedMethodCall) {
        return None;
    }
    let mut owned = ir.clone();
    let mut calls = Vec::new();
    for local in 0..owned.op.len() {
        let op = owned.op.get(local).copied();
        if op != Some(Op::MethodCall) && op != Some(Op::NamedMethodCall) {
            continue;
        }
        let named = op == Some(Op::NamedMethodCall);
        let start = owned.b.get(local).copied()? as usize;
        let shape = owned.c.get(local).copied()?;
        let count = (shape / 2) as usize;
        let fresh = u32::try_from(owned.kids.len()).ok()?;
        if named {
            let Some(first) = first_param(ir, *owned.a.get(local)?) else {
                continue;
            };
            let moved: Vec<u32> = owned.kids.get(start..start + 2 * count + 1)?.to_vec();
            owned.kids.push(first);
            owned.kids.extend(moved);
            *owned.op.get_mut(local)? = Op::NamedCall;
        } else {
            let moved: Vec<u32> = owned.kids.get(start..start + count + 1)?.to_vec();
            owned.kids.extend(moved);
            *owned.op.get_mut(local)? = Op::Call;
        }
        *owned.b.get_mut(local)? = fresh;
        *owned.c.get_mut(local)? = shape + 2;
        calls.push(owned.base + u32::try_from(local).ok()?);
    }
    Some((owned, calls))
}

fn first_param(ir: &FlatIr, method: u32) -> Option<u32> {
    let local = method.checked_sub(ir.base)? as usize;
    if ir.op.get(local) != Some(&Op::Name) {
        return None;
    }
    let sym = ir.name(*ir.a.get(local)?)?.sym;
    let mut found = ir
        .fns
        .iter()
        .filter(|rec| ir.name(rec.name).is_some_and(|entry| entry.sym == sym));
    let rec = found.next()?;
    if found.next().is_some() || rec.params.0 == rec.params.1 {
        return None;
    }
    Some(ir.binders.get(rec.params.0 as usize)?.name)
}
