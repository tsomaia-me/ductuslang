use super::expr::Expr;
use crate::source::Span;
use crate::types::Ty;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ty: Ty,
    pub expr: Expr,
    pub span: Span,
}
