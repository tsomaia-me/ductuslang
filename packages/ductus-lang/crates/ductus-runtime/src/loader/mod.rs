//! The loader: Module → [`LoadedGraph`], with the loader owning ALL legality
//! (R21) — slice checks, duplicate paths, `init` conversion (R57), input
//! linking, behavior resolvability (D5), cycle detection, the static evaluation
//! order (R64), and the gate laws: a `guards` list checked as the STRICT
//! inverse of the guarded constructs' own clauses (R80 item 5), and the
//! `gate_parent` chain proved acyclic. What a predicate may READ is not among
//! them — a gate whose predicate reads its own guarded cells is the
//! self-conditional gate of SPEC §13.9.6 and loads (owner ruling, 2026-08-24).
//! The parser over-admitted the grammar; this is where a `.duir` module becomes
//! a legal program or a named diagnostic.

pub mod cell_index;
pub mod cell_role;
pub mod gate_index;
pub mod load;
pub mod loaded_cell;
pub mod loaded_gate;
pub mod loaded_graph;
pub mod lookback_link;

pub use self::cell_index::CellIndex;
pub use self::cell_role::CellRole;
pub use self::gate_index::GateIndex;
pub use self::load::load;
pub use self::loaded_cell::LoadedCell;
pub use self::loaded_gate::LoadedGate;
pub use self::loaded_graph::LoadedGraph;
pub use self::lookback_link::LookbackLink;
