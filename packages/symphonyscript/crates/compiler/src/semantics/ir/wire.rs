use super::attr::Attr;
use super::ident::Ident;
use super::ids::ExprId;
use super::loc::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Wire {
    pub kind: Ident,
    pub flags: Vec<(char, Loc)>,
    pub name: Option<Ident>,
    pub default: Option<ExprId>,
    pub gate: Option<ExprId>,
    pub attrs: Vec<Attr>,
    pub destination: ExprId,
    pub loc: Loc,
}
