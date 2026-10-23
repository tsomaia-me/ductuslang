//! The behavior provider M0 witnesses use, and the stubs shared across them.
//!
//! [`StubRegistry`] is the ruled harness seam (M0.md §5); the rest of the
//! folder is the SHARED half of the corpus README's "named stubs from the
//! shared stub library".
//!
//! # What "shared" means here
//!
//! A stub earns a file when it is used by two or more witnesses, or by the
//! property tests. A single-use stub — a witness's gating fixture, its
//! trapping fixture — is written INLINE in the witness test that needs it,
//! beside the golden it explains: moving it here would put the reason a golden
//! holds one file away from the golden. The library holds vocabulary, not
//! every closure.
//!
//! Every stub here must be PURE (M0.md §5) and total: it computes from its
//! arguments alone, wraps rather than overflowing, and panics only on an
//! argument shape it was never bound to — which is a harness-author bug, not a
//! Ductus trap.

pub mod add1_i64;
pub mod double_i32;
pub mod registry_of;
pub mod stub_registry;
pub mod sum_i64;

pub use self::add1_i64::add1_i64;
pub use self::double_i32::double_i32;
pub use self::registry_of::registry_of;
pub use self::stub_registry::StubRegistry;
pub use self::sum_i64::sum_i64;
