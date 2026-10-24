use super::loc::Loc;
use super::type_base::TypeBase;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct TypeRef {
    pub base: TypeBase,
    pub lengths: Vec<(Option<u32>, Loc)>,
    pub loc: Loc,
}
