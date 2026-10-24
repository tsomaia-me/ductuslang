#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]

mod rand;
mod spec_float;

pub use rand::rand;
pub use spec_float::SpecFloat;
