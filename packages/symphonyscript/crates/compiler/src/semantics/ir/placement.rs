use super::attr::Attr;
use super::ident::Ident;
use super::ids::{BlockId, ExprId};
use super::loc::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Placement {
    pub kind: Option<Ident>,
    pub flags: Vec<(char, Loc)>,
    pub name: Option<Ident>,
    pub has_as: bool,
    pub default: Option<ExprId>,
    pub gate: Option<ExprId>,
    pub attrs: Vec<Attr>,
    pub body: Option<BlockId>,
    pub inline: bool,
    pub loc: Loc,
}
