use super::c_array::CArray;
use super::c_val::CVal;
use super::fast_hash::FastMap;

pub(crate) fn map_leaves<E>(
    value: CVal,
    mapped: &mut FastMap<*const (), (CArray, CVal)>,
    mut leaf: impl FnMut(CVal) -> Result<CVal, E>,
) -> Result<CVal, E> {
    let CVal::Array(root) = value else {
        return leaf(value);
    };
    let mut stack: Vec<(CArray, bool)> = vec![(root.clone(), false)];
    while let Some((array, ready)) = stack.pop() {
        crate::semantics::counted_walk::CountedWalk::MapLeavesMapLeaves.step();
        if mapped.contains_key(&array.identity()) {
            continue;
        }
        CArray::count_visit();
        if !ready {
            stack.push((array.clone(), true));
            if let Some((base, ..)) = array.sliced() {
                stack.push((base.clone(), false));
                continue;
            }
            match array.operands() {
                Some((left, right)) => {
                    stack.push((right.clone(), false));
                    stack.push((left.clone(), false));
                }
                None => stack.extend(array.items().iter().rev().filter_map(|item| match item {
                    CVal::Array(inner) => Some((inner.clone(), false)),
                    _ => None,
                })),
            }
            continue;
        }
        let value = match (array.sliced(), array.operands()) {
            (Some((base, start, length)), _) => match mapped_of(mapped, base) {
                CVal::Array(new_base) if new_base.identity() == base.identity() => {
                    CVal::Array(array.clone())
                }
                CVal::Array(new_base) => CVal::Array(CArray::slice(
                    &new_base,
                    start,
                    start.saturating_add(length),
                )),
                _ => CVal::Error,
            },
            (None, Some((left, right))) => {
                let left_mapped = mapped_of(mapped, left);
                let right_mapped = mapped_of(mapped, right);
                match (left_mapped, right_mapped) {
                    (CVal::Array(new_left), CVal::Array(new_right))
                        if new_left.identity() == left.identity()
                            && new_right.identity() == right.identity() =>
                    {
                        CVal::Array(array.clone())
                    }
                    (CVal::Array(new_left), CVal::Array(new_right)) => {
                        CArray::join(&new_left, &new_right).map_or(CVal::Error, CVal::Array)
                    }
                    _ => CVal::Error,
                }
            }
            (None, None) => {
                let mut built = Vec::with_capacity(array.len());
                let mut unchanged = true;
                for item in array.items() {
                    let new = match item {
                        CVal::Array(inner) => mapped_of(mapped, inner),
                        item => leaf(item.clone())?,
                    };
                    unchanged &= same(item, &new);
                    built.push(new);
                }
                if unchanged {
                    CVal::Array(array.clone())
                } else {
                    CVal::Array(CArray::new(built))
                }
            }
        };
        if let CVal::Array(result) = &value
            && result.identity() != array.identity()
        {
            mapped.insert(result.identity(), (result.clone(), value.clone()));
        }
        mapped.insert(array.identity(), (array, value));
    }
    Ok(mapped_of(mapped, &root))
}

fn mapped_of(mapped: &FastMap<*const (), (CArray, CVal)>, array: &CArray) -> CVal {
    mapped
        .get(&array.identity())
        .map_or(CVal::Error, |(_, value)| value.clone())
}

fn same(old: &CVal, new: &CVal) -> bool {
    match (old, new) {
        (CVal::Array(old), CVal::Array(new)) => old.identity() == new.identity(),
        (CVal::F32(old), CVal::F32(new)) => old.to_bits() == new.to_bits(),
        (CVal::F64(old), CVal::F64(new)) => old.to_bits() == new.to_bits(),
        (old, new) => old == new,
    }
}
