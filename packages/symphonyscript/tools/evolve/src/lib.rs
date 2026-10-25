#![forbid(unsafe_code)]

mod arguments;
mod checker;
mod dedup_key;
mod differing_codes;
mod finding;
mod guarded;
mod items;
mod measured;
mod observation;
mod over_memory_bound;
mod program;
mod renamed;
mod scaled;
mod shrunk;
mod shuffled;
mod signal;
mod superlinear;
mod variants;
mod verdict;
mod work;

pub use arguments::{Arguments, arguments};
pub use checker::{Checker, DEFAULT_BUDGET};
pub use dedup_key::dedup_key;
pub use differing_codes::differing_codes;
pub use finding::Finding;
pub use guarded::{Guarded, guarded};
pub use measured::Measured;
pub use over_memory_bound::over_memory_bound;
pub use program::{Program, ProgramSource};
pub use scaled::scaled;
pub use shrunk::shrunk;
pub use signal::{Signal, UnknownSignal};
pub use superlinear::superlinear;
pub use verdict::{DiagnosticRecord, Verdict};
pub use work::Work;
