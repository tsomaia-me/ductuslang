#![cfg_attr(not(test), forbid(unsafe_code))]
#![cfg_attr(test, deny(unsafe_code))]

mod compiler;
#[cfg(test)]
mod counting_allocator;
mod diagnostics;
mod failed;
mod semantics;
mod source;
pub mod syntax;

pub use compiler::Compiler;
pub use diagnostics::{Code, Diagnostic, Related, Severity};
pub use failed::Failed;
pub use source::Source;

#[cfg(test)]
#[global_allocator]
static ALLOCATOR: counting_allocator::CountingAllocator = counting_allocator::CountingAllocator;
