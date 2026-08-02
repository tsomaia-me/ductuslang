//! Runtime value vocabulary: the `Value` enum (i64/i32/f64/f32/bool) and the
//! canonical rendering the trace writes.

// The file is named after the entity it holds (S1), and the entity is `Value`,
// so `value/value.rs` stutters. Owner-accepted; `expect` rather than `allow` so
// the annotation reports itself as stale if the layout ever changes.
#[expect(
    clippy::module_inception,
    reason = "S1 names files after their entity; `Value` lives in `value/`"
)]
pub mod value;

pub use self::value::Value;
