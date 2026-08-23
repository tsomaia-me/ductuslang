//! The behavior-provider seam: how the runtime resolves behavior handles to
//! executables (M0.md §3, seam 1). M0's sole implementation is the conformance
//! harness's stub registry, per the §4a-1 ruling; the behavior table is the
//! only codegen point in the system, ever — the runtime is otherwise pure data.

pub mod behavior;
pub mod behavior_provider;

pub use self::behavior::Behavior;
pub use self::behavior_provider::BehaviorProvider;
