//! The `registry_of` function: a `StubRegistry` holding exactly the bindings a
//! witness names.

use std::rc::Rc;

use ductus_runtime::provider::Behavior;

use super::stub_registry::StubRegistry;

/// A registry holding `bindings` and nothing else.
///
/// Every witness names its handles and its stubs in one place, at the head of
/// its test, so the module's `uses B@dN` clauses and the executables behind
/// them read side by side. The bindings are CLONED — a [`Behavior`] is a shared
/// `Rc`, so the caller keeps its own handle on a stub it also wants to call
/// directly.
#[must_use]
pub fn registry_of(bindings: &[(u32, Behavior)]) -> StubRegistry {
    let mut registry = StubRegistry::new();

    for (handle, behavior) in bindings {
        registry.register(*handle, Rc::clone(behavior));
    }

    registry
}

#[cfg(test)]
mod tests {
    use ductus_runtime::provider::BehaviorProvider;
    use ductus_runtime::value::Value;

    use crate::stub_library::add1_i64::add1_i64;

    use super::*;

    #[test]
    fn resolves_every_handle_it_was_given_and_no_other() {
        let registry = registry_of(&[(0xd1, add1_i64())]);

        let behavior = registry.resolve(0xd1).expect("the handle is registered");

        assert!(behavior(&[Value::I64(1)]).bits_eq(&Value::I64(2)));
        assert!(registry.resolve(0xd2).is_none());
    }

    #[test]
    fn an_empty_binding_list_makes_an_empty_registry() {
        assert!(registry_of(&[]).resolve(0xd1).is_none());
    }
}
