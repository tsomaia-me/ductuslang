//! Ductus IR — graph-spec and behavior-table types, plus the `.duir` text form
//! (parse + emit). The shared vocabulary crate: `ductus-runtime` depends on
//! this, never the reverse.

pub mod behaviors;
pub mod diag;
pub mod graph;
pub mod module;
pub mod path;
pub mod text;
pub mod types;
pub mod value;
