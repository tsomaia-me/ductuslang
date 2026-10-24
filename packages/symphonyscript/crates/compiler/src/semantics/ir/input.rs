use super::ident::Ident;
use super::ids::ExprId;
use super::loc::Loc;
use super::type_ref::TypeRef;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Input {
    pub name: Option<Ident>,
    pub ty: Option<TypeRef>,
    pub value: ExprId,
    pub loc: Loc,
}
