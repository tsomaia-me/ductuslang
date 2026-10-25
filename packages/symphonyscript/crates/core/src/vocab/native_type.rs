use crate::types::Ty;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NativeType {
    Value(Ty),
    Open(Ty),
    CapacityOf(Ty, usize),
}
