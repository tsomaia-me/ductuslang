//! The `StubRegistry` struct: M0's sole behavior provider.

use std::collections::HashMap;

use ductus_runtime::provider::{Behavior, BehaviorProvider};

/// The M0 rendering of the ruled harness seam (M0.md §5): a table from
/// behavior handle to executable stub.
///
/// M0 modules carry `behaviors {}` — no behavior-body compiler exists yet — so
/// the loader checks resolvability against module table ∪ provider registry
/// (D5). With the module half empty throughout M0, THIS is the whole answer: a
/// handle registered here resolves, a handle registered nowhere is the
/// load-time `unresolvable_behavior` diagnostic.
///
/// # Purity is the witness author's contract
///
/// A stub must be pure — same inputs, same output, no observable effects
/// (M0.md §5). The registry cannot enforce it and does not try: a `HashMap`
/// of closures has no way to see inside one. A stub that counts its calls or
/// reads a clock would make its witness's golden a fiction, so the author
/// carries that obligation.
///
/// # No `Debug`
///
/// [`Behavior`] is `Rc<dyn Fn(&[Value]) -> Value>`, which has no `Debug`, so
/// neither does the table holding them. A registry prints as nothing useful
/// anyway — the handles are in the witness's own source.
#[derive(Default)]
pub struct StubRegistry {
    behaviors: HashMap<u32, Behavior>,
}

impl StubRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> StubRegistry {
        StubRegistry::default()
    }

    /// Binds `handle` to `behavior`, replacing any previous binding.
    ///
    /// `handle` is the `u32` a `B@` BID decodes to — the same `u32` the
    /// module's `uses` clauses carry. Last registration wins: a witness that
    /// registers one handle twice means the second, which keeps a shared
    /// helper overridable by the witness that needs a different stub there.
    pub fn register(&mut self, handle: u32, behavior: Behavior) {
        self.behaviors.insert(handle, behavior);
    }
}

impl BehaviorProvider for StubRegistry {
    fn resolve(&self, handle: u32) -> Option<Behavior> {
        self.behaviors.get(&handle).cloned()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use ductus_runtime::value::Value;

    use super::*;

    // Exercised THROUGH `&dyn BehaviorProvider`, exactly as the loader holds
    // it.
    #[test]
    fn resolves_a_registered_handle_and_refuses_an_unknown_one() {
        let mut registry = StubRegistry::new();

        registry.register(0xd1, Rc::new(|_| Value::I64(7)));

        let provider: &dyn BehaviorProvider = &registry;
        let behavior = provider.resolve(0xd1).expect("the handle is registered");

        assert!(behavior(&[]).bits_eq(&Value::I64(7)));
        assert!(provider.resolve(0xd2).is_none());
    }

    // Resolution hands out SHARED ownership, per `Behavior`'s contract: two
    // resolutions of one handle are one executable.
    #[test]
    fn resolving_twice_hands_out_one_shared_executable() {
        let mut registry = StubRegistry::new();

        registry.register(1, Rc::new(|_| Value::Bool(true)));

        let first = registry.resolve(1).expect("the handle is registered");
        let second = registry.resolve(1).expect("the handle is registered");

        assert!(Rc::ptr_eq(&first, &second));
    }

    #[test]
    fn the_last_registration_of_a_handle_wins() {
        let mut registry = StubRegistry::new();

        registry.register(2, Rc::new(|_| Value::I32(1)));
        registry.register(2, Rc::new(|_| Value::I32(2)));

        let behavior = registry.resolve(2).expect("the handle is registered");

        assert!(behavior(&[]).bits_eq(&Value::I32(2)));
    }
}
