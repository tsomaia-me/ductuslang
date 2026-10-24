use symphonyscript_core::AttrId;

use super::attr_value_source::AttrValueSource;
use crate::semantics::ir::Loc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct AttrSource {
    pub attr: AttrId,
    pub value: AttrValueSource,
    pub loc: Loc,
}
