use super::expr_kind::ExprKind;
use super::loc::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Expr {
    pub kind: ExprKind,
    pub loc: Loc,
}
