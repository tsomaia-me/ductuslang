use super::c_val::CVal;

pub(crate) fn lengths_of(value: &CVal) -> Vec<u32> {
    match value {
        CVal::Array(array) => array.lengths(),
        CVal::Int(_)
        | CVal::F32(_)
        | CVal::F64(_)
        | CVal::Bool(_)
        | CVal::Enum(..)
        | CVal::Str(_)
        | CVal::Node(_)
        | CVal::Error => Vec::new(),
    }
}
