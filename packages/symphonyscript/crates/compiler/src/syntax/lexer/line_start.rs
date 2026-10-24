use crate::syntax::token::Token;

pub(crate) struct LineStart<'t> {
    pub(crate) first: &'t Token,
    pub(crate) rest: &'t [Token],
    pub(crate) rest_depths: &'t [u32],
    pub(crate) top_level: bool,
    pub(crate) closes_region: bool,
}
