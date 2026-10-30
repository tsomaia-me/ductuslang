use super::ident::Ident;
use super::type_ref::TypeRef;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct LoopVar {
    pub name: Option<Ident>,
    pub ty: Option<TypeRef>,
}
