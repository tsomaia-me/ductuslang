use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::val::Val;
use crate::semantics::collection::Decl;

pub(crate) enum Found {
    Val(Val),
    Query(Query),
    Item(ItemRef),
    Decl(Decl),
    Nothing,
}
