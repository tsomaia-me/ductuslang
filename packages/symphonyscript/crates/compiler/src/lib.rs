#![forbid(unsafe_code)]

mod compiler;
mod diagnostics;
mod failed;
mod semantics;
mod source;
pub mod syntax;

pub use compiler::Compiler;
pub use diagnostics::{Code, Diagnostic, Severity};
pub use failed::Failed;
pub use source::Source;
