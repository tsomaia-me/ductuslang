//! The `Behavior` alias: the executable form of a behavior.

use std::rc::Rc;

use crate::value::Value;

/// The executable form of a behavior — the derived-cell ABI row (SPEC §15.4.4):
/// the input cells' snapshot values in, the new value out (M0.md §5).
///
/// # Why `Rc`
///
/// The registry owns the closures; resolution hands out SHARED ownership, so a
/// loaded graph can hold its behaviors without borrowing the provider's
/// lifetime into every downstream type. M0 is strictly sequential — no threads
/// anywhere, harness included — which makes `Rc` (not `Arc`) right; the seam
/// can revisit that at a threaded milestone.
///
/// # Purity is the author's contract
///
/// A stub must be PURE — same inputs, same output, no observable effects. The
/// runtime cannot enforce it; the harness author must (M0.md §5).
pub type Behavior = Rc<dyn Fn(&[Value]) -> Value>;

#[cfg(test)]
mod tests {
    use super::*;

    // The alias IS the ABI: a slice of input-cell snapshot values in, one new
    // value out.
    #[test]
    fn a_closure_coerces_and_computes_through_the_alias() {
        let double: Behavior = Rc::new(|inputs| match inputs {
            [Value::I64(payload)] => Value::I64(payload * 2),
            other => panic!("one i64 input, got {other:?}"),
        });

        assert!(double(&[Value::I64(21)]).bits_eq(&Value::I64(42)));
    }

    // Shared ownership is the point of the `Rc`: a clone is the same
    // executable, not a copy of it.
    #[test]
    fn clones_share_one_executable() {
        let behavior: Behavior = Rc::new(|_| Value::Bool(true));
        let shared = Rc::clone(&behavior);

        assert!(Rc::ptr_eq(&behavior, &shared));
        assert!(shared(&[]).bits_eq(&Value::Bool(true)));
    }
}
