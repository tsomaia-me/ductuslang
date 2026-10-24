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
pub(crate) mod ir;
pub(crate) mod reporting;
pub(crate) mod scopes;
