//! The `BehaviorProvider` trait: the seam resolving behavior handles.

use super::behavior::Behavior;

/// The seam through which the runtime resolves a behavior handle to an
/// executable (M0.md §3, seam 1).
///
/// `handle` is the `u32` a `B@` BID decodes to — the same `u32` the graph AST
/// carries in a cell's `uses` and a gate's `pred`. `None` means this provider
/// does not know the handle. The loader composes module table ∪ provider
/// registry for resolvability (D5), and with the module's behavior table empty
/// throughout M0, the registry is the whole answer: a handle satisfied by
/// neither is the load-time `unresolvable_behavior` diagnostic.
///
/// M0's sole implementation is the conformance harness's stub registry.
/// Object-safe on purpose — the loader takes `&dyn BehaviorProvider`.
pub trait BehaviorProvider {
    /// The executable `handle` names, or `None` when this provider does not
    /// know it.
    fn resolve(&self, handle: u32) -> Option<Behavior>;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::rc::Rc;

    use super::*;
    use crate::value::Value;

    // The shape of the real M0 implementation — the harness stub registry,
    // which lives in ductus-conformance — reduced to what proving the seam
    // needs.
    struct StubRegistry {
        behaviors: HashMap<u32, Behavior>,
    }

    impl BehaviorProvider for StubRegistry {
        fn resolve(&self, handle: u32) -> Option<Behavior> {
            self.behaviors.get(&handle).cloned()
        }
    }

    // Exercised THROUGH `&dyn BehaviorProvider`, exactly as the loader holds
    // it — the test is the object-safety proof as much as the contract's.
    #[test]
    fn resolves_a_registered_handle_and_refuses_an_unknown_one() {
        let mut behaviors: HashMap<u32, Behavior> = HashMap::new();
        behaviors.insert(0x2a, Rc::new(|_| Value::I64(7)));

        let registry = StubRegistry { behaviors };
        let provider: &dyn BehaviorProvider = &registry;
        let behavior = provider.resolve(0x2a).expect("the handle is registered");

        assert!(behavior(&[]).bits_eq(&Value::I64(7)));
        assert!(provider.resolve(0x2b).is_none());
    }

    // Resolution hands out shared ownership: two resolutions of one handle are
    // one executable, per `Behavior`'s contract.
    #[test]
    fn resolving_twice_hands_out_one_shared_executable() {
        let mut behaviors: HashMap<u32, Behavior> = HashMap::new();
        behaviors.insert(1, Rc::new(|_| Value::Bool(true)));

        let registry = StubRegistry { behaviors };
        let provider: &dyn BehaviorProvider = &registry;

        let first = provider.resolve(1).expect("the handle is registered");
        let second = provider.resolve(1).expect("the handle is registered");

        assert!(Rc::ptr_eq(&first, &second));
    }
}
