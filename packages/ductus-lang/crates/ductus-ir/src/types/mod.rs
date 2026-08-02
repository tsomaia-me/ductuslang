//! The type vocabulary — every type the `.duir` text form can name (§15.4.6).
//!
//! Two layers, and they are not the same thing. A [`TypeTag`] is a type USED at
//! a position — a cell's type, a field's type, a closure parameter's type — and
//! is a recursive structure built from [`Prim`] and from references to named
//! types. A [`TypeDef`] is a row of the module's type table: a name bound to a
//! [`Layout`], with the size and alignment the lowering computed, whose
//! [`Field`]s and [`Variant`]s carry tags in turn.
//!
//! Nothing here validates. A tag may name a type the module never defines and a
//! record may repeat a field name; both parse, and the loader rejects them
//! (F4). The parser's job is the grammar, and this is the grammar's shape.
//!
//! The productions are IR_GRAMMAR §2 (`PRIM`, `type_tag`) and §3 (`type_def`,
//! `layout`, `field_list`, `variant`).

pub mod field;
pub mod layout;
pub mod prim;
pub mod type_def;
pub mod type_tag;
pub mod variant;

pub use self::field::Field;
pub use self::layout::Layout;
pub use self::prim::Prim;
pub use self::type_def::TypeDef;
pub use self::type_tag::TypeTag;
pub use self::variant::Variant;
