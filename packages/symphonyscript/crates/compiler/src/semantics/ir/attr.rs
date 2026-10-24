use super::ident::Ident;
use super::ids::ExprId;
use super::loc::Loc;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum AttrValue {
    Expr(ExprId),
    Set(bool),
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Attr {
    pub name: Ident,
    pub value: AttrValue,
    pub loc: Loc,
}
