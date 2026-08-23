//! The `add1_i64` stub: one `i64` in, its successor out.

use std::rc::Rc;

use ductus_runtime::provider::Behavior;
use ductus_runtime::value::Value;

/// `x -> x + 1` over one [`Value::I64`] input.
///
/// The chain stub. Composing it down a chain makes DEPTH readable straight off
/// the trace: `a = 5` settles a chain of three as `6`, `7`, `8`, so a topo
/// order that evaluated a link before its dependency would show a wrong number
/// rather than merely a wrong line order.
///
/// The addition WRAPS, for the reason [`double_i32()`](super::double_i32()) gives:
/// a fixture must not trap by accident.
///
/// # Panics
///
/// On any other argument shape — wrong arity, or an input that is not `I64`.
/// A harness-author bug, loud rather than papered over.
#[must_use]
pub fn add1_i64() -> Behavior {
    Rc::new(|inputs| match inputs {
        [Value::I64(payload)] => Value::I64(payload.wrapping_add(1)),
        other => panic!("add1_i64 wants one i64 input, got {other:?}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_one_to_its_input() {
        let stub = add1_i64();

        assert!(stub(&[Value::I64(0)]).bits_eq(&Value::I64(1)));
        assert!(stub(&[Value::I64(-1)]).bits_eq(&Value::I64(0)));
    }

    #[test]
    fn wraps_instead_of_trapping_at_the_extreme() {
        let stub = add1_i64();

        assert!(stub(&[Value::I64(i64::MAX)]).bits_eq(&Value::I64(i64::MIN)));
    }

    #[test]
    #[should_panic(expected = "add1_i64 wants one i64 input")]
    fn panics_on_the_wrong_value_type() {
        let _ = add1_i64()(&[Value::I32(1)]);
    }

    #[test]
    #[should_panic(expected = "add1_i64 wants one i64 input")]
    fn panics_on_the_wrong_arity() {
        let _ = add1_i64()(&[]);
    }
}
