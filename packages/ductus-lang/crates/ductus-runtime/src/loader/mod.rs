//! The loader: Module → [`LoadedGraph`], with the loader owning ALL legality
//! (R21) — M0 slice checks, duplicate paths, `init` conversion (R57), input
//! linking, behavior resolvability (D5), cycle detection, and the static
//! evaluation order (R64). The parser over-admitted the grammar; this is where
//! a `.duir` module becomes a legal program or a named diagnostic.

pub mod cell_index;
pub mod cell_role;
pub mod load;
pub mod loaded_cell;
pub mod loaded_graph;

pub use self::cell_index::CellIndex;
pub use self::cell_role::CellRole;
pub use self::load::load;
pub use self::loaded_cell::LoadedCell;
pub use self::loaded_graph::LoadedGraph;
