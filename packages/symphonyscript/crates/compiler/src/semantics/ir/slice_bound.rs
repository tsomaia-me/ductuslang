use super::ids::ExprId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct SliceBound {
    pub value: ExprId,
    pub from_end: bool,
}
