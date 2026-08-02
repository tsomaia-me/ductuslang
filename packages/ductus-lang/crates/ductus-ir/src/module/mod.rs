//! The module AST — the structural mirror of a `.duir` module's text form (T1):
//! a [`Module`] holds the three sections in grammar order. Section CONTENTS
//! land at their component rows: `type_def`s at increment 5, behaviors at
//! increment 6, scopes and their entries at increment 7.
//!
//! The concern is the MODULE, so it owns the module header and the three
//! section shells only (R55). `graph/` owns the graph section's entries —
//! `Scope`, cells, gates, wires, effects, streams — which are a concern of
//! their own and outnumber everything here.
//!
//! The grammar is IR_GRAMMAR §3 (§15.4.6).

// The file is named after the entity it holds (S1), and the entity is `Module`,
// so `module/module.rs` stutters — the same trade `value/value.rs` makes.
// `expect` rather than `allow` so the annotation reports itself as stale if the
// layout ever changes.
#[expect(
    clippy::module_inception,
    reason = "S1 names files after their entity; `Module` lives in `module/`"
)]
pub mod module;

pub mod behaviors_section;
pub mod graph_section;
pub mod types_section;

pub use self::behaviors_section::BehaviorsSection;
pub use self::graph_section::GraphSection;
pub use self::module::Module;
pub use self::types_section::TypesSection;
