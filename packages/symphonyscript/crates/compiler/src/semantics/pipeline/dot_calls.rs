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
            let method = *owned.a.get(local)?;
            let first = match first_param(ir, method) {
                Some(first) => first,
                None if !names_a_function(ir, method) => method_name(ir, method)?,
                None => continue,
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

fn method_name(ir: &FlatIr, method: u32) -> Option<u32> {
    let local = method.checked_sub(ir.base)? as usize;
    if ir.op.get(local) != Some(&Op::Name) {
        return None;
    }
    ir.a.get(local).copied()
}

fn names_a_function(ir: &FlatIr, method: u32) -> bool {
    let Some(sym) = method_name(ir, method)
        .and_then(|name| ir.name(name))
        .map(|entry| entry.sym)
    else {
        return true;
    };
    ir.fns
        .iter()
        .any(|rec| ir.name(rec.name).is_some_and(|entry| entry.sym == sym))
}

pub(crate) fn field_calls(ir: &FlatIr) -> Option<(FlatIr, Vec<(u32, u32, u32)>)> {
    let mut found = Vec::new();
    for local in 0..ir.op.len() {
        if ir.op.get(local) != Some(&Op::Field) {
            continue;
        }
        let receiver = *ir.a.get(local)?;
        let field = *ir.b.get(local)?;
        let receiver_op = ir
            .op
            .get(receiver.checked_sub(ir.base)? as usize)
            .copied()?;
        if matches!(receiver_op, Op::Field | Op::To) || ir.c.get(local) == Some(&1) {
            continue;
        }
        let sym = ir.name(field)?.sym;
        let mut named = ir
            .fns
            .iter()
            .filter(|rec| ir.name(rec.name).is_some_and(|entry| entry.sym == sym));
        let one = named.next().is_some_and(|rec| rec.params.0 != rec.params.1);
        if one && named.next().is_none() {
            found.push(local);
        }
    }
    if found.is_empty() {
        return None;
    }
    let mut owned = ir.clone();
    let mut calls = Vec::new();
    for local in found {
        let field_node = owned.base + u32::try_from(local).ok()?;
        let receiver = *owned.a.get(local)?;
        let name = *owned.b.get(local)?;
        let entry = *owned.name(name)?;
        let callee = owned.base + u32::try_from(owned.op.len()).ok()?;
        owned.op.push(Op::Name);
        owned.a.push(name);
        owned.b.push(0);
        owned.c.push(0);
        owned.lo.push(entry.lo);
        owned.hi.push(entry.hi);
        let call = owned.base + u32::try_from(owned.op.len()).ok()?;
        let start = u32::try_from(owned.kids.len()).ok()?;
        owned.kids.push(callee);
        owned.kids.push(receiver);
        owned.op.push(Op::Call);
        owned.a.push(callee);
        owned.b.push(start + 1);
        owned.c.push(2);
        owned.lo.push(*owned.lo.get(local)?);
        owned.hi.push(*owned.hi.get(local)?);
        calls.push((field_node, call, callee));
    }
    Some((owned, calls))
}
