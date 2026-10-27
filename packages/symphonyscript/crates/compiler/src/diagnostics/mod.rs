#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]

mod code;
mod diagnostic;
mod help;
mod lesson;
mod related;
mod severity;

pub use code::Code;
pub use diagnostic::Diagnostic;
pub(crate) use help::{mismatched_closer_help, stray_closer_help};
pub use related::Related;
pub use severity::Severity;
