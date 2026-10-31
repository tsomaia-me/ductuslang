use super::cell_word::CellWord;
use super::ident::Ident;
use super::ids::ExprId;
use super::loc::Loc;
use super::type_ref::TypeRef;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct StateDecl {
    pub name: Option<Ident>,
    pub word: CellWord,
    pub init: Option<ExprId>,
    pub clock: Vec<ExprId>,
    pub ty: Option<TypeRef>,
    pub value: ExprId,
    pub loc: Loc,
}
