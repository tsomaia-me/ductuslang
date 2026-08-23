//! The runtime's value vocabulary — the M0 word tier (SPEC §14.3): exactly the
//! five word-sized primitives, shared by the behavior ABI, the store, and the
//! trace.

// The file is named after the entity it holds (S1), and the entity is `Value`,
// so `value/value.rs` stutters — the same trade `module/module.rs` makes in
// `ductus-ir`. `expect` rather than `allow` so the annotation reports itself
// as stale if the layout ever changes.
#[expect(
    clippy::module_inception,
    reason = "S1 names files after their entity; `Value` lives in `value/`"
)]
pub mod value;

pub mod value_type;

pub use self::value::Value;
pub use self::value_type::ValueType;
