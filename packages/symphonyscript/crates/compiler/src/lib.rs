#![forbid(unsafe_code)]

mod diagnostics;
mod source;
pub mod syntax;

pub use diagnostics::{Code, Diagnostic, Severity};
pub use source::Source;
