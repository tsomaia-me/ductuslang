use crate::semantics::ir::flat::{FlatIr, Op};

pub(crate) fn dot_calls(ir: &FlatIr) -> Option<(FlatIr, Vec<u32>)> {
    if !ir.op.contains(&Op::MethodCall) {
        return None;
    }
    let mut owned = ir.clone();
    let mut calls = Vec::new();
    for local in 0..owned.op.len() {
        if owned.op.get(local) != Some(&Op::MethodCall) {
            continue;
        }
        let start = owned.b.get(local).copied()? as usize;
        let shape = owned.c.get(local).copied()?;
        let count = (shape / 2) as usize;
        let moved: Vec<u32> = owned.kids.get(start..start + count + 1)?.to_vec();
        let fresh = u32::try_from(owned.kids.len()).ok()?;
        owned.kids.extend(moved);
        *owned.op.get_mut(local)? = Op::Call;
        *owned.b.get_mut(local)? = fresh;
        *owned.c.get_mut(local)? = shape + 2;
        calls.push(owned.base + u32::try_from(local).ok()?);
    }
    Some((owned, calls))
}
