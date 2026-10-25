#![forbid(unsafe_code)]

mod arguments;
mod checker;
mod dedup_key;
mod differing_codes;
mod finding;
mod guarded;
mod items;
mod observation;
mod program;
mod renamed;
mod shrunk;
mod shuffled;
mod signal;
mod variants;
mod verdict;

pub use arguments::{Arguments, arguments};
pub use checker::{Checker, DEFAULT_BUDGET};
pub use dedup_key::dedup_key;
pub use differing_codes::differing_codes;
pub use finding::Finding;
pub use guarded::{Guarded, guarded};
pub use program::{Program, ProgramSource};
pub use shrunk::shrunk;
pub use signal::{Signal, UnknownSignal};
pub use verdict::{DiagnosticRecord, Verdict};
