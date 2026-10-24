use super::ids::ExprId;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Iterable {
    Range(ExprId, ExprId),
    Values(ExprId),
}
