use super::c_array::CArray;
use super::c_val::CVal;

pub(crate) fn map_leaves(value: CVal, mut leaf: impl FnMut(CVal) -> CVal) -> CVal {
    let CVal::Array(root) = value else {
        return leaf(value);
    };
    let mut stack: Vec<(CArray, usize, Vec<CVal>)> = vec![(root, 0, Vec::new())];
    while let Some((array, index, built)) = stack.last_mut() {
        let item = array.items().get(*index).cloned();
        *index += 1;
        match item {
            Some(CVal::Array(inner)) => stack.push((inner, 0, Vec::new())),
            Some(item) => built.push(leaf(item)),
            None => {
                let Some((_, _, built)) = stack.pop() else {
                    break;
                };
                let value = CVal::Array(CArray::new(built));
                match stack.last_mut() {
                    Some((_, _, parent)) => parent.push(value),
                    None => return value,
                }
            }
        }
    }
    CVal::Error
}
