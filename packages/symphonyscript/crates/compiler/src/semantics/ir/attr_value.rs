use super::ids::ExprId;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum AttrValue {
    Expr(ExprId),
    Set(bool),
}
