#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]

pub(crate) mod analysis;
pub(crate) mod analyze;
pub(crate) mod collection;
pub(crate) mod counted_walk;
#[cfg(test)]
mod counted_walk_tests;
mod internal;
pub(crate) mod ir;
pub(crate) mod reporting;
pub(crate) mod scopes;

pub(crate) use internal::Internal;
