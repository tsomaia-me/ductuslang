#![cfg_attr(not(test), forbid(unsafe_code))]
#![cfg_attr(test, deny(unsafe_code))]

mod compiler;
#[cfg(test)]
mod counting_allocator;
mod diagnostics;
mod failed;
#[cfg(test)]
mod pinned_row;
#[cfg(test)]
mod pinned_row_tests;
mod selector;
mod semantics;
mod source;
pub mod syntax;

pub use compiler::Compiler;
pub use diagnostics::{Code, Diagnostic, Related, Severity};
pub use failed::Failed;
pub use selector::Selector;
pub use source::Source;

#[cfg(test)]
#[global_allocator]
static ALLOCATOR: counting_allocator::CountingAllocator = counting_allocator::CountingAllocator;
