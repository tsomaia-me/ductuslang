use super::attr_value::AttrValue;
use super::ident::Ident;
use super::loc::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Attr {
    pub name: Ident,
    pub value: AttrValue,
    pub loc: Loc,
}
