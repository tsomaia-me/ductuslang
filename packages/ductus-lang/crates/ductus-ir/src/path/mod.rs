//! Cell identity: `Segment` and `CellPath` — structured segments whose
//! canonical dotted rendering is derived from the structure, never stored.

pub mod cell_path;
pub mod segment;

pub use self::cell_path::CellPath;
pub use self::segment::Segment;
