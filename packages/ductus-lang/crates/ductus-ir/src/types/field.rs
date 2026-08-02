//! The `Field` struct: one named member of a `record` layout.

use crate::types::TypeTag;

/// One `NAME ':' type_tag` pair of a `record` layout's `field_list`
/// (IR_GRAMMAR §3, §15.4.6).
///
/// Fields are held in DECLARATION ORDER by their layout, and the order is
/// meaning, not presentation: it is what the emitter writes back and what a
/// layout-sensitive consumer reads. Nothing sorts them.
///
/// Plain data with public fields. Legality — a duplicate field name, a tag
/// naming no known type — belongs to the loader (F4).
#[derive(Debug, Clone)]
pub struct Field {
    /// The field's NAME.
    pub name: String,
    /// The field's type tag.
    pub tag: TypeTag,
}
